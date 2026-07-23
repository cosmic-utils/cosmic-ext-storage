//! Typed logical-action dispatch to native UDisks methods.

use std::collections::HashMap;

use async_trait::async_trait;
use storage_contracts::{
    BtrfsResizeRequest, LogicalAction, LogicalActionOutcome, LogicalOperations,
    MdRaidCreateProfile, MdRaidSyncAction, StorageError, StorageErrorKind,
};
use storage_types::{BlockDeviceRef, ConfirmedDestructiveScope, LogicalEntityId};
use udisks2::{manager::ManagerProxy, mdraid::MDRaidProxy};
use zbus::zvariant::{ObjectPath, OwnedObjectPath, Value};

use crate::UdisksBackend;

use super::{
    error::{conflict, native_error, unsupported},
    proxy::{BtrfsProxy, LogicalVolumeProxy, LvmManagerProxy, VolumeGroupProxy},
    resolve::{
        BTRFS_INTERFACE, MDRAID_INTERFACE, VOLUME_GROUP_INTERFACE, blocks, managed_paths,
        resolve_block,
    },
};

type Options = HashMap<&'static str, Value<'static>>;

#[async_trait]
impl LogicalOperations for UdisksBackend {
    async fn execute_logical_action(
        &self,
        action: LogicalAction,
    ) -> Result<LogicalActionOutcome, StorageError> {
        action.validate()?;
        let affected_entity_ids = action_affected_ids(&action);
        match &action {
            LogicalAction::CreateLvmVolumeGroup { name, devices } => {
                let blocks = resolve_fresh_blocks(self, devices).await?;
                let manager = LvmManagerProxy::new(self.manager().connection())
                    .await
                    .map_err(native_error)?;
                let paths: Vec<ObjectPath<'_>> =
                    blocks.iter().map(|block| block.path.as_ref()).collect();
                manager
                    .volume_group_create(name, &paths, empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::DeleteLvmVolumeGroup {
                volume_group,
                confirmed_scope,
                ..
            } => {
                let path = find_volume_group(self, volume_group).await?;
                verify_vg_scope(self, &path, volume_group, confirmed_scope).await?;
                let proxy = VolumeGroupProxy::builder(self.manager().connection())
                    .path(path)
                    .map_err(native_error)?
                    .build()
                    .await
                    .map_err(native_error)?;
                proxy
                    .delete(preserve_delete_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::AddLvmPhysicalVolume {
                volume_group,
                device,
            } => {
                let path = find_volume_group(self, volume_group).await?;
                let device = resolve_fresh_block(self, device).await?;
                let proxy = VolumeGroupProxy::builder(self.manager().connection())
                    .path(path)
                    .map_err(native_error)?
                    .build()
                    .await
                    .map_err(native_error)?;
                proxy
                    .add_device(&device.path.as_ref(), empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::RemoveLvmPhysicalVolume {
                volume_group,
                device,
                ..
            } => {
                let path = find_volume_group(self, volume_group).await?;
                let device = resolve_fresh_block(self, device).await?;
                let proxy = VolumeGroupProxy::builder(self.manager().connection())
                    .path(path)
                    .map_err(native_error)?
                    .build()
                    .await
                    .map_err(native_error)?;
                proxy
                    .remove_device(&device.path.as_ref(), preserve_wipe_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::CreateLvmLogicalVolume {
                volume_group,
                name,
                size_bytes,
            } => {
                let path = find_volume_group(self, volume_group).await?;
                let proxy = VolumeGroupProxy::builder(self.manager().connection())
                    .path(path)
                    .map_err(native_error)?
                    .build()
                    .await
                    .map_err(native_error)?;
                proxy
                    .create_plain_volume(name, *size_bytes, empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::DeleteLvmLogicalVolume {
                logical_volume,
                confirmed_scope,
                ..
            } => {
                verify_empty_scope(logical_volume, confirmed_scope)?;
                let path = find_logical_volume(self, logical_volume).await?;
                let proxy = LogicalVolumeProxy::builder(self.manager().connection())
                    .path(path)
                    .map_err(native_error)?
                    .build()
                    .await
                    .map_err(native_error)?;
                proxy
                    .delete(preserve_teardown_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::ResizeLvmLogicalVolume {
                logical_volume,
                size_bytes,
            } => {
                let path = find_logical_volume(self, logical_volume).await?;
                let proxy = LogicalVolumeProxy::builder(self.manager().connection())
                    .path(path)
                    .map_err(native_error)?
                    .build()
                    .await
                    .map_err(native_error)?;
                proxy
                    .resize(*size_bytes, empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::ActivateLvmLogicalVolume { logical_volume } => {
                let path = find_logical_volume(self, logical_volume).await?;
                let proxy = LogicalVolumeProxy::builder(self.manager().connection())
                    .path(path)
                    .map_err(native_error)?
                    .build()
                    .await
                    .map_err(native_error)?;
                proxy
                    .activate(empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::DeactivateLvmLogicalVolume { logical_volume } => {
                let path = find_logical_volume(self, logical_volume).await?;
                let proxy = LogicalVolumeProxy::builder(self.manager().connection())
                    .path(path)
                    .map_err(native_error)?
                    .build()
                    .await
                    .map_err(native_error)?;
                proxy
                    .deactivate(empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::CreateMdRaidArray {
                name,
                level,
                devices,
                profile,
            } => {
                if profile.level != *level {
                    return Err(StorageError::new(
                        StorageErrorKind::InvalidInput,
                        "MD RAID profile does not match its level.",
                    ));
                }
                let blocks = resolve_fresh_blocks(self, devices).await?;
                let manager = ManagerProxy::new(self.manager().connection())
                    .await
                    .map_err(native_error)?;
                let paths: Vec<ObjectPath<'_>> =
                    blocks.iter().map(|block| block.path.as_ref()).collect();
                manager
                    .mdraid_create(
                        &paths,
                        level.source_name(),
                        &name.0,
                        profile.chunk_bytes,
                        mdraid_profile_options(),
                    )
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::DeleteMdRaidArray {
                array,
                confirmed_scope,
                ..
            } => {
                let path = find_mdraid(self, array).await?;
                verify_mdraid_scope(self, &path, array, confirmed_scope).await?;
                let proxy = MDRaidProxy::builder(self.manager().connection())
                    .path(path)
                    .map_err(native_error)?
                    .build()
                    .await
                    .map_err(native_error)?;
                proxy
                    .delete(preserve_teardown_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::StartMdRaidArray { array } => {
                let proxy = mdraid_proxy(self, array).await?;
                proxy.start(empty_options()).await.map_err(native_error)?;
            }
            LogicalAction::StopMdRaidArray { array } => {
                let proxy = mdraid_proxy(self, array).await?;
                proxy.stop(empty_options()).await.map_err(native_error)?;
            }
            LogicalAction::AddMdRaidMember { array, device } => {
                let proxy = mdraid_proxy(self, array).await?;
                let device = resolve_fresh_block(self, device).await?;
                proxy
                    .add_device(&device.path.as_ref(), empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::RemoveMdRaidMember { array, device, .. } => {
                let proxy = mdraid_proxy(self, array).await?;
                let device = resolve_fresh_block(self, device).await?;
                proxy
                    .remove_device(&device.path.as_ref(), preserve_wipe_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::RequestMdRaidSync { array, action } => {
                let proxy = mdraid_proxy(self, array).await?;
                let command = match action {
                    MdRaidSyncAction::Check => "check",
                    MdRaidSyncAction::Repair => "repair",
                };
                proxy
                    .request_sync_action(command, empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::AddBtrfsDevice { filesystem, device } => {
                let proxy = btrfs_proxy(self, filesystem).await?;
                let device = resolve_fresh_block(self, device).await?;
                proxy
                    .add_device(&device.path.as_ref(), empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::RemoveBtrfsDevice { filesystem, device } => {
                let proxy = btrfs_proxy(self, filesystem).await?;
                let device = resolve_fresh_block(self, device).await?;
                proxy
                    .remove_device(&device.path.as_ref(), empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::ResizeBtrfsFilesystem {
                filesystem,
                request,
            } => {
                let BtrfsResizeRequest::AbsoluteBytes(size) = request else {
                    return Err(unsupported(
                        "This Btrfs size syntax has no audited native UDisks mapping.",
                    ));
                };
                let proxy = btrfs_proxy(self, filesystem).await?;
                proxy
                    .resize(*size, empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::SetBtrfsLabel { filesystem, label } => {
                let proxy = btrfs_proxy(self, filesystem).await?;
                proxy
                    .set_label(label, empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::SetBtrfsDefaultSubvolume {
                filesystem,
                subvolume_id,
            } => {
                let proxy = btrfs_proxy(self, filesystem).await?;
                proxy
                    .set_default_subvolume_id(subvolume_id.get(), empty_options())
                    .await
                    .map_err(native_error)?;
            }
        }
        Ok(LogicalActionOutcome {
            action,
            affected_entity_ids,
            native_job_id: None,
            progress: None,
        })
    }
}

async fn resolve_fresh_blocks(
    backend: &UdisksBackend,
    references: &[BlockDeviceRef],
) -> Result<Vec<super::resolve::ResolvedBlock>, StorageError> {
    let epoch = backend.logical_epoch();
    let mut blocks = Vec::with_capacity(references.len());
    for reference in references {
        let first = resolve_block(backend.manager(), epoch, reference).await?;
        let second = resolve_block(backend.manager(), backend.logical_epoch(), reference).await?;
        if first.path != second.path || first.fingerprint != second.fingerprint {
            return Err(conflict("The selected device changed; no action was sent."));
        }
        blocks.push(second);
    }
    Ok(blocks)
}

async fn resolve_fresh_block(
    backend: &UdisksBackend,
    reference: &BlockDeviceRef,
) -> Result<super::resolve::ResolvedBlock, StorageError> {
    Ok(
        resolve_fresh_blocks(backend, std::slice::from_ref(reference))
            .await?
            .remove(0),
    )
}

async fn find_volume_group(
    backend: &UdisksBackend,
    target: &LogicalEntityId,
) -> Result<OwnedObjectPath, StorageError> {
    let uuid = target.0.strip_prefix("lvm-vg:").ok_or_else(|| {
        StorageError::new(
            StorageErrorKind::InvalidInput,
            "Expected an LVM volume group ID.",
        )
    })?;
    for (path, interfaces) in managed_paths(backend.manager()).await? {
        if !interfaces
            .iter()
            .any(|interface| interface == VOLUME_GROUP_INTERFACE)
        {
            continue;
        }
        let proxy = VolumeGroupProxy::builder(backend.manager().connection())
            .path(&path)
            .map_err(native_error)?
            .build()
            .await
            .map_err(native_error)?;
        if proxy.uuid().await.map_err(native_error)? == uuid {
            return Ok(path);
        }
    }
    Err(StorageError::new(
        StorageErrorKind::NotFound,
        "LVM volume group no longer exists.",
    ))
}

async fn find_logical_volume(
    backend: &UdisksBackend,
    target: &LogicalEntityId,
) -> Result<OwnedObjectPath, StorageError> {
    let suffix = target.0.strip_prefix("lvm-lv:").ok_or_else(|| {
        StorageError::new(
            StorageErrorKind::InvalidInput,
            "Expected an LVM logical volume ID.",
        )
    })?;
    let Some((vg_uuid, lv_name)) = suffix.split_once(':') else {
        return Err(StorageError::new(
            StorageErrorKind::InvalidInput,
            "Malformed LVM logical volume ID.",
        ));
    };
    let vg_path = find_volume_group(backend, &LogicalEntityId(format!("lvm-vg:{vg_uuid}"))).await?;
    let vg = VolumeGroupProxy::builder(backend.manager().connection())
        .path(vg_path)
        .map_err(native_error)?
        .build()
        .await
        .map_err(native_error)?;
    for path in vg.logical_volumes().await.map_err(native_error)? {
        let lv = LogicalVolumeProxy::builder(backend.manager().connection())
            .path(&path)
            .map_err(native_error)?
            .build()
            .await
            .map_err(native_error)?;
        if lv.name().await.map_err(native_error)? == lv_name {
            return Ok(path);
        }
    }
    Err(StorageError::new(
        StorageErrorKind::NotFound,
        "LVM logical volume no longer exists.",
    ))
}

async fn find_mdraid(
    backend: &UdisksBackend,
    target: &LogicalEntityId,
) -> Result<OwnedObjectPath, StorageError> {
    let uuid = target.0.strip_prefix("mdraid:").ok_or_else(|| {
        StorageError::new(StorageErrorKind::InvalidInput, "Expected an MD RAID ID.")
    })?;
    for (path, interfaces) in managed_paths(backend.manager()).await? {
        if !interfaces
            .iter()
            .any(|interface| interface == MDRAID_INTERFACE)
        {
            continue;
        }
        let proxy = MDRaidProxy::builder(backend.manager().connection())
            .path(&path)
            .map_err(native_error)?
            .build()
            .await
            .map_err(native_error)?;
        if proxy.uuid().await.map_err(native_error)? == uuid {
            return Ok(path);
        }
    }
    Err(StorageError::new(
        StorageErrorKind::NotFound,
        "MD RAID array no longer exists.",
    ))
}

async fn mdraid_proxy(
    backend: &UdisksBackend,
    target: &LogicalEntityId,
) -> Result<MDRaidProxy<'static>, StorageError> {
    let path = find_mdraid(backend, target).await?;
    MDRaidProxy::builder(backend.manager().connection())
        .path(path)
        .map_err(native_error)?
        .build()
        .await
        .map_err(native_error)
}

async fn btrfs_proxy(
    backend: &UdisksBackend,
    target: &LogicalEntityId,
) -> Result<BtrfsProxy<'static>, StorageError> {
    let fsid = target.0.strip_prefix("btrfs:").ok_or_else(|| {
        StorageError::new(
            StorageErrorKind::InvalidInput,
            "Expected a Btrfs filesystem ID.",
        )
    })?;
    for (path, interfaces) in managed_paths(backend.manager()).await? {
        if !interfaces
            .iter()
            .any(|interface| interface == BTRFS_INTERFACE)
        {
            continue;
        }
        let blocks = blocks(backend.manager()).await?;
        if !blocks
            .iter()
            .any(|block| block.path == path && block.id_uuid == fsid)
        {
            continue;
        }
        return BtrfsProxy::builder(backend.manager().connection())
            .path(path)
            .map_err(native_error)?
            .build()
            .await
            .map_err(native_error);
    }
    Err(StorageError::new(
        StorageErrorKind::Unsupported,
        "Btrfs UDisks plugin is unavailable.",
    ))
}

async fn verify_vg_scope(
    backend: &UdisksBackend,
    path: &OwnedObjectPath,
    target: &LogicalEntityId,
    expected: &ConfirmedDestructiveScope,
) -> Result<(), StorageError> {
    let proxy = VolumeGroupProxy::builder(backend.manager().connection())
        .path(path)
        .map_err(native_error)?
        .build()
        .await
        .map_err(native_error)?;
    let uuid = proxy.uuid().await.map_err(native_error)?;
    let mut entity_ids = Vec::new();
    for lv_path in proxy.logical_volumes().await.map_err(native_error)? {
        let lv = LogicalVolumeProxy::builder(backend.manager().connection())
            .path(lv_path)
            .map_err(native_error)?
            .build()
            .await
            .map_err(native_error)?;
        entity_ids.push(LogicalEntityId(format!(
            "lvm-lv:{uuid}:{}",
            lv.name().await.map_err(native_error)?
        )));
    }
    entity_ids.sort();
    let actual = ConfirmedDestructiveScope::new(entity_ids, Vec::new())
        .map_err(|error| StorageError::new(StorageErrorKind::Conflict, error.to_string()))?;
    if &actual != expected || !expected.excludes(target) {
        return Err(conflict("The LVM deletion scope changed; review it again."));
    }
    Ok(())
}

fn verify_empty_scope(
    target: &LogicalEntityId,
    expected: &ConfirmedDestructiveScope,
) -> Result<(), StorageError> {
    let actual =
        ConfirmedDestructiveScope::new(Vec::new(), Vec::new()).expect("empty scope is valid");
    if &actual != expected || !expected.excludes(target) {
        return Err(conflict(
            "The logical-volume deletion scope changed; review it again.",
        ));
    }
    Ok(())
}

async fn verify_mdraid_scope(
    backend: &UdisksBackend,
    path: &OwnedObjectPath,
    target: &LogicalEntityId,
    expected: &ConfirmedDestructiveScope,
) -> Result<(), StorageError> {
    let proxy = MDRaidProxy::builder(backend.manager().connection())
        .path(path)
        .map_err(native_error)?
        .build()
        .await
        .map_err(native_error)?;
    let current_blocks = blocks(backend.manager()).await?;
    let mut device_refs = Vec::new();
    for (member_path, _, _, _, _) in proxy.active_devices().await.map_err(native_error)? {
        let Some(block) = current_blocks
            .iter()
            .find(|block| block.path == member_path)
        else {
            return Err(conflict(
                "An MD RAID member disappeared; no action was sent.",
            ));
        };
        let Some(fingerprint) = &block.fingerprint else {
            return Err(conflict(
                "An MD RAID member lost its strong identity; refresh and retry.",
            ));
        };
        device_refs.push(BlockDeviceRef::new(
            block.id.clone(),
            fingerprint.clone(),
            backend.logical_epoch(),
        ));
    }
    device_refs.sort();
    let actual = ConfirmedDestructiveScope::new(Vec::new(), device_refs)
        .map_err(|error| StorageError::new(StorageErrorKind::Conflict, error.to_string()))?;
    if &actual != expected || !expected.excludes(target) {
        return Err(conflict(
            "The MD RAID deletion scope changed; review it again.",
        ));
    }
    Ok(())
}

fn action_affected_ids(action: &LogicalAction) -> Vec<LogicalEntityId> {
    match action {
        LogicalAction::DeleteLvmVolumeGroup { volume_group, .. }
        | LogicalAction::AddLvmPhysicalVolume { volume_group, .. }
        | LogicalAction::RemoveLvmPhysicalVolume { volume_group, .. }
        | LogicalAction::CreateLvmLogicalVolume { volume_group, .. } => vec![volume_group.clone()],
        LogicalAction::DeleteLvmLogicalVolume { logical_volume, .. }
        | LogicalAction::ResizeLvmLogicalVolume { logical_volume, .. }
        | LogicalAction::ActivateLvmLogicalVolume { logical_volume }
        | LogicalAction::DeactivateLvmLogicalVolume { logical_volume } => {
            vec![logical_volume.clone()]
        }
        LogicalAction::DeleteMdRaidArray { array, .. }
        | LogicalAction::StartMdRaidArray { array }
        | LogicalAction::StopMdRaidArray { array }
        | LogicalAction::AddMdRaidMember { array, .. }
        | LogicalAction::RemoveMdRaidMember { array, .. }
        | LogicalAction::RequestMdRaidSync { array, .. } => vec![array.clone()],
        LogicalAction::AddBtrfsDevice { filesystem, .. }
        | LogicalAction::RemoveBtrfsDevice { filesystem, .. }
        | LogicalAction::ResizeBtrfsFilesystem { filesystem, .. }
        | LogicalAction::SetBtrfsLabel { filesystem, .. }
        | LogicalAction::SetBtrfsDefaultSubvolume { filesystem, .. } => vec![filesystem.clone()],
        LogicalAction::CreateLvmVolumeGroup { .. } | LogicalAction::CreateMdRaidArray { .. } => {
            Vec::new()
        }
    }
}

fn empty_options() -> Options {
    HashMap::new()
}

fn preserve_wipe_options() -> Options {
    HashMap::from([("wipe", Value::from(false))])
}

fn preserve_teardown_options() -> Options {
    HashMap::from([("tear-down", Value::from(false))])
}

fn preserve_delete_options() -> Options {
    HashMap::from([
        ("wipe", Value::from(false)),
        ("tear-down", Value::from(false)),
    ])
}

fn mdraid_profile_options() -> Options {
    HashMap::from([(
        "version",
        Value::from(MdRaidCreateProfile::METADATA_VERSION.to_vec()),
    )])
}

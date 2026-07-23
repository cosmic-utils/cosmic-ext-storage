//! Native logical-topology discovery.

use std::collections::{BTreeMap, HashMap};

use storage_contracts::StorageError;
use storage_types::{
    BlockDeviceRef, LogicalBlockedReason, LogicalCapabilities, LogicalEntity, LogicalEntityId,
    LogicalEntityKind, LogicalMember, LogicalMemberId, LogicalOperation, ProgressRatio,
};
use udisks2::mdraid::MDRaidProxy;

use crate::DiskManager;

use super::{
    error::native_error,
    proxy::{BtrfsProxy, LogicalVolumeProxy, VolumeGroupProxy},
    resolve::{BTRFS_INTERFACE, MDRAID_INTERFACE, VOLUME_GROUP_INTERFACE, blocks, managed_paths},
};

pub(crate) async fn entities(
    manager: &DiskManager,
    observed_generation: u64,
) -> Result<Vec<LogicalEntity>, StorageError> {
    let paths = managed_paths(manager).await?;
    let blocks = blocks(manager).await?;
    let block_by_path: HashMap<_, _> = blocks
        .iter()
        .map(|block| (block.path.clone(), block.clone()))
        .collect();

    let mut output = Vec::new();
    output.extend(volume_groups(manager, &paths, &block_by_path, observed_generation).await?);
    output.extend(mdraid_arrays(manager, &paths, &block_by_path, observed_generation).await?);
    output.extend(btrfs_filesystems(manager, &paths, &block_by_path).await?);
    output.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
    output.dedup_by(|left, right| left.id == right.id);
    Ok(output)
}

async fn volume_groups(
    manager: &DiskManager,
    paths: &HashMap<zbus::zvariant::OwnedObjectPath, Vec<String>>,
    block_by_path: &HashMap<zbus::zvariant::OwnedObjectPath, super::resolve::ResolvedBlock>,
    generation: u64,
) -> Result<Vec<LogicalEntity>, StorageError> {
    let mut output = Vec::new();
    for (path, interfaces) in paths {
        if !interfaces
            .iter()
            .any(|interface| interface == VOLUME_GROUP_INTERFACE)
        {
            continue;
        }
        let proxy = VolumeGroupProxy::builder(manager.connection())
            .path(path)
            .map_err(native_error)?
            .build()
            .await
            .map_err(native_error)?;
        let uuid = proxy.uuid().await.map_err(native_error)?;
        if uuid.trim().is_empty() {
            continue;
        }
        let id = LogicalEntityId(format!("lvm-vg:{uuid}"));
        let name = proxy.name().await.unwrap_or_else(|_| uuid.clone());
        let size = proxy.size().await.unwrap_or(0);
        let free = proxy.free_size().await.unwrap_or(0);
        let logical_volumes = proxy.logical_volumes().await.unwrap_or_default();
        let physical_volumes = proxy.physical_volumes().await.unwrap_or_default();
        let mut members = Vec::new();
        let mut children = Vec::new();

        for lv_path in logical_volumes {
            let lv = LogicalVolumeProxy::builder(manager.connection())
                .path(&lv_path)
                .map_err(native_error)?
                .build()
                .await
                .map_err(native_error)?;
            let lv_name = lv
                .name()
                .await
                .unwrap_or_else(|_| "Logical volume".to_string());
            let lv_id = LogicalEntityId(format!("lvm-lv:{uuid}:{lv_name}"));
            let active = lv.active().await.unwrap_or(false);
            let lv_size = lv.size().await.unwrap_or(0);
            members.push(LogicalMember {
                id: LogicalMemberId(format!("member:{}", lv_id.0)),
                kind: LogicalEntityKind::LvmLogicalVolume,
                name: lv_name.clone(),
                device_ref: None,
                device_path: None,
                role: Some("lv".into()),
                state: Some(if active { "active" } else { "inactive" }.into()),
                size_bytes: Some(lv_size),
            });
            children.push(LogicalEntity {
                id: lv_id,
                kind: LogicalEntityKind::LvmLogicalVolume,
                name: lv_name,
                uuid: None,
                parent_id: Some(id.clone()),
                device_path: None,
                size_bytes: lv_size,
                used_bytes: None,
                free_bytes: None,
                health_status: Some(if active { "active" } else { "inactive" }.into()),
                progress_fraction: None,
                members: Vec::new(),
                capabilities: logical_volume_capabilities(active),
                metadata: BTreeMap::new(),
            });
        }

        for pv_path in physical_volumes {
            let Some(block) = block_by_path.get(&pv_path) else {
                continue;
            };
            let Some(fingerprint) = &block.fingerprint else {
                continue;
            };
            let device_ref = BlockDeviceRef::new(block.id.clone(), fingerprint.clone(), generation);
            let pv_id = LogicalEntityId(format!("lvm-pv:{uuid}:{}", fingerprint.as_str()));
            members.push(LogicalMember {
                id: LogicalMemberId(format!("member:{}", pv_id.0)),
                kind: LogicalEntityKind::LvmPhysicalVolume,
                name: block.device_path.clone(),
                device_ref: Some(device_ref.clone()),
                device_path: Some(block.device_path.clone()),
                role: Some("pv".into()),
                state: None,
                size_bytes: None,
            });
            children.push(LogicalEntity {
                id: pv_id,
                kind: LogicalEntityKind::LvmPhysicalVolume,
                name: block.device_path.clone(),
                uuid: None,
                parent_id: Some(id.clone()),
                device_path: Some(block.device_path.clone()),
                size_bytes: 0,
                used_bytes: None,
                free_bytes: None,
                health_status: None,
                progress_fraction: None,
                members: Vec::new(),
                capabilities: LogicalCapabilities::normalized(
                    vec![LogicalOperation::RemoveMember],
                    Vec::new(),
                ),
                metadata: BTreeMap::new(),
            });
        }

        output.push(LogicalEntity {
            id,
            kind: LogicalEntityKind::LvmVolumeGroup,
            name,
            uuid: Some(uuid),
            parent_id: None,
            device_path: None,
            size_bytes: size,
            used_bytes: Some(size.saturating_sub(free)),
            free_bytes: Some(free),
            health_status: None,
            progress_fraction: None,
            members,
            capabilities: LogicalCapabilities::normalized(
                vec![
                    LogicalOperation::Delete,
                    LogicalOperation::AddMember,
                    LogicalOperation::RemoveMember,
                    LogicalOperation::Create,
                ],
                Vec::new(),
            ),
            metadata: BTreeMap::new(),
        });
        output.extend(children);
    }
    Ok(output)
}

async fn mdraid_arrays(
    manager: &DiskManager,
    paths: &HashMap<zbus::zvariant::OwnedObjectPath, Vec<String>>,
    block_by_path: &HashMap<zbus::zvariant::OwnedObjectPath, super::resolve::ResolvedBlock>,
    generation: u64,
) -> Result<Vec<LogicalEntity>, StorageError> {
    let mut output = Vec::new();
    for (path, interfaces) in paths {
        if !interfaces
            .iter()
            .any(|interface| interface == MDRAID_INTERFACE)
        {
            continue;
        }
        let proxy = MDRaidProxy::builder(manager.connection())
            .path(path)
            .map_err(native_error)?
            .build()
            .await
            .map_err(native_error)?;
        let uuid = proxy.uuid().await.unwrap_or_default();
        if uuid.trim().is_empty() {
            continue;
        }
        let id = LogicalEntityId(format!("mdraid:{uuid}"));
        let running = proxy.running().await.unwrap_or(false);
        let degraded = proxy.degraded().await.unwrap_or(0) > 0;
        let mut members = Vec::new();
        let mut children = Vec::new();
        for (member_path, _slot, states, member_size, _) in
            proxy.active_devices().await.unwrap_or_default()
        {
            let Some(block) = block_by_path.get(&member_path) else {
                continue;
            };
            let Some(fingerprint) = &block.fingerprint else {
                continue;
            };
            let device_ref = BlockDeviceRef::new(block.id.clone(), fingerprint.clone(), generation);
            let member_id = LogicalEntityId(format!("mdraid-member:{}", fingerprint.as_str()));
            let state = states.join(",");
            members.push(LogicalMember {
                id: LogicalMemberId(format!("member:{}", member_id.0)),
                kind: LogicalEntityKind::MdRaidMember,
                name: block.device_path.clone(),
                device_ref: Some(device_ref),
                device_path: Some(block.device_path.clone()),
                role: Some("member".into()),
                state: Some(state.clone()),
                size_bytes: Some(member_size),
            });
            children.push(LogicalEntity {
                id: member_id,
                kind: LogicalEntityKind::MdRaidMember,
                name: block.device_path.clone(),
                uuid: None,
                parent_id: Some(id.clone()),
                device_path: Some(block.device_path.clone()),
                size_bytes: member_size,
                used_bytes: None,
                free_bytes: None,
                health_status: Some(state),
                progress_fraction: None,
                members: Vec::new(),
                capabilities: LogicalCapabilities::default(),
                metadata: BTreeMap::new(),
            });
        }
        output.push(LogicalEntity {
            id,
            kind: LogicalEntityKind::MdRaidArray,
            name: proxy.name().await.unwrap_or_else(|_| uuid.clone()),
            uuid: Some(uuid),
            parent_id: None,
            device_path: None,
            size_bytes: proxy.size().await.unwrap_or(0),
            used_bytes: None,
            free_bytes: None,
            health_status: Some(if degraded { "degraded" } else { "ok" }.into()),
            progress_fraction: proxy
                .sync_completed()
                .await
                .ok()
                .map(ProgressRatio::from_fraction),
            members,
            capabilities: mdraid_capabilities(running),
            metadata: BTreeMap::from([("level".into(), proxy.level().await.unwrap_or_default())]),
        });
        output.extend(children);
    }
    Ok(output)
}

async fn btrfs_filesystems(
    manager: &DiskManager,
    paths: &HashMap<zbus::zvariant::OwnedObjectPath, Vec<String>>,
    block_by_path: &HashMap<zbus::zvariant::OwnedObjectPath, super::resolve::ResolvedBlock>,
) -> Result<Vec<LogicalEntity>, StorageError> {
    let mut output = Vec::new();
    for (path, interfaces) in paths {
        if !interfaces
            .iter()
            .any(|interface| interface == BTRFS_INTERFACE)
        {
            continue;
        }
        let Some(block) = block_by_path.get(path) else {
            continue;
        };
        if block.id_uuid.trim().is_empty() {
            continue;
        }
        let fsid = block.id_uuid.clone();
        let id = LogicalEntityId(format!("btrfs:{fsid}"));
        let proxy = BtrfsProxy::builder(manager.connection())
            .path(path)
            .map_err(native_error)?
            .build()
            .await
            .map_err(native_error)?;
        let default_id = proxy.get_default_subvolume_id(HashMap::new()).await.ok();
        let mut metadata = BTreeMap::new();
        if let Some(default_id) = default_id {
            metadata.insert("default_subvolume_id".into(), default_id.to_string());
        }
        let mut children = Vec::new();
        for (subvolume_id, subvolume_path) in proxy
            .get_subvolumes(false, HashMap::new())
            .await
            .map_err(native_error)?
        {
            children.push(LogicalEntity {
                id: LogicalEntityId(format!("btrfs-subvolume:{fsid}:{subvolume_id}")),
                kind: LogicalEntityKind::BtrfsSubvolume,
                name: subvolume_path.clone(),
                uuid: None,
                parent_id: Some(id.clone()),
                device_path: None,
                size_bytes: 0,
                used_bytes: None,
                free_bytes: None,
                health_status: None,
                progress_fraction: None,
                members: Vec::new(),
                capabilities: LogicalCapabilities::default(),
                metadata: BTreeMap::from([("subvolume_id".into(), subvolume_id.to_string())]),
            });
        }
        output.push(LogicalEntity {
            id,
            kind: LogicalEntityKind::BtrfsFilesystem,
            name: if block.device_path.is_empty() {
                fsid.clone()
            } else {
                block.device_path.clone()
            },
            uuid: Some(fsid),
            parent_id: None,
            device_path: Some(block.device_path.clone()),
            size_bytes: 0,
            used_bytes: None,
            free_bytes: None,
            health_status: None,
            progress_fraction: None,
            members: Vec::new(),
            capabilities: LogicalCapabilities::normalized(
                vec![
                    LogicalOperation::Create,
                    LogicalOperation::Delete,
                    LogicalOperation::AddMember,
                    LogicalOperation::RemoveMember,
                    LogicalOperation::Resize,
                    LogicalOperation::SetLabel,
                    LogicalOperation::SetDefaultSubvolume,
                ],
                Vec::new(),
            ),
            metadata,
        });
        output.extend(children);
    }
    Ok(output)
}

fn logical_volume_capabilities(active: bool) -> LogicalCapabilities {
    let mut supported = vec![LogicalOperation::Delete, LogicalOperation::Resize];
    supported.push(if active {
        LogicalOperation::Deactivate
    } else {
        LogicalOperation::Activate
    });
    LogicalCapabilities::normalized(supported, Vec::new())
}

fn mdraid_capabilities(running: bool) -> LogicalCapabilities {
    let mut supported = vec![
        LogicalOperation::Delete,
        LogicalOperation::AddMember,
        LogicalOperation::RemoveMember,
        LogicalOperation::Check,
        LogicalOperation::Repair,
    ];
    supported.push(if running {
        LogicalOperation::Stop
    } else {
        LogicalOperation::Start
    });
    LogicalCapabilities::normalized(supported, Vec::<LogicalBlockedReason>::new())
}

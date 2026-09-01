//! Fresh object-manager resolution for logical actions.

use std::collections::HashMap;

use storage_contracts::{StorageError, StorageErrorKind};
use storage_types::{BlockDeviceFingerprint, BlockDeviceId, BlockDeviceRef};
use udisks2::{
    block::BlockProxy, drive::DriveProxy, nvme::namespace::NamespaceProxy,
    partition::PartitionProxy,
};
use zbus::{fdo::ObjectManagerProxy, zvariant::OwnedObjectPath};

use crate::{DiskManager, dbus::bytestring::decode_c_string_bytes};

use super::error::{conflict, native_error};

pub(crate) const BLOCK_INTERFACE: &str = "org.freedesktop.UDisks2.Block";
pub(crate) const VOLUME_GROUP_INTERFACE: &str = "org.freedesktop.UDisks2.VolumeGroup";
pub(crate) const MDRAID_INTERFACE: &str = "org.freedesktop.UDisks2.MDRaid";
pub(crate) const BTRFS_INTERFACE: &str = "org.freedesktop.UDisks2.Filesystem.BTRFS";

#[derive(Debug, Clone)]
pub(crate) struct ResolvedBlock {
    pub path: OwnedObjectPath,
    pub id: BlockDeviceId,
    pub fingerprint: Option<BlockDeviceFingerprint>,
    pub device_path: String,
    pub id_type: String,
    pub id_uuid: String,
    pub id_label: String,
    pub size: Option<u64>,
    pub read_only: Option<bool>,
}

pub(crate) async fn managed_paths(
    manager: &DiskManager,
) -> Result<HashMap<OwnedObjectPath, Vec<String>>, StorageError> {
    let object_manager = ObjectManagerProxy::builder(manager.connection())
        .destination("org.freedesktop.UDisks2")
        .map_err(native_error)?
        .path("/org/freedesktop/UDisks2")
        .map_err(native_error)?
        .build()
        .await
        .map_err(native_error)?;
    let objects = object_manager
        .get_managed_objects()
        .await
        .map_err(native_error)?;
    Ok(objects
        .into_iter()
        .map(|(path, interfaces)| {
            (
                path,
                interfaces
                    .into_keys()
                    .map(|interface| interface.to_string())
                    .collect(),
            )
        })
        .collect())
}

pub(crate) async fn blocks(manager: &DiskManager) -> Result<Vec<ResolvedBlock>, StorageError> {
    let paths = managed_paths(manager).await?;
    let mut blocks = Vec::new();
    for (path, interfaces) in paths {
        if !interfaces
            .iter()
            .any(|interface| interface == BLOCK_INTERFACE)
        {
            continue;
        }
        let proxy = BlockProxy::builder(manager.connection())
            .path(&path)
            .map_err(native_error)?
            .build()
            .await
            .map_err(native_error)?;
        let number = proxy.device_number().await.map_err(native_error)?;
        let id = BlockDeviceId::new(device_major(number), device_minor(number));
        let id_type = proxy.id_type().await.unwrap_or_default();
        let id_uuid = proxy.id_uuid().await.unwrap_or_default();
        let device = decode_c_string_bytes(&proxy.device().await.unwrap_or_default());
        let preferred = decode_c_string_bytes(&proxy.preferred_device().await.unwrap_or_default());
        let device_path = if preferred.is_empty() {
            device
        } else {
            preferred
        };
        let fingerprint = strong_fingerprint(manager, &path, &proxy, &id).await;
        blocks.push(ResolvedBlock {
            path: path.clone(),
            id,
            fingerprint,
            device_path,
            id_type,
            id_uuid,
            id_label: proxy.id_label().await.unwrap_or_default(),
            size: proxy.size().await.ok(),
            read_only: proxy.read_only().await.ok(),
        });
    }
    Ok(blocks)
}

/// Derive the only identities that may authorize a logical mutation.  In
/// particular `Block.IdUUID` and a bare partition UUID are intentionally not
/// considered fingerprints: both can be cloned across devices.
async fn strong_fingerprint(
    manager: &DiskManager,
    path: &OwnedObjectPath,
    block: &BlockProxy<'_>,
    block_id: &BlockDeviceId,
) -> Option<BlockDeviceFingerprint> {
    if let Some(fingerprint) = drive_fingerprint(manager, path, block).await {
        return Some(fingerprint);
    }

    // A cleartext `/dev/mapper/*` device has no Drive of its own.  UDisks
    // exposes its LUKS backing block explicitly, however, so bind navigation
    // to that backing partition/drive identity instead of treating a normal
    // encrypted-root mapping as an unidentifiable transient device.  The
    // major/minor in BlockDeviceRef still identifies this mapper node; this
    // fingerprint prevents it being silently reused for another backing disk.
    if let Some(backing_path) = block
        .crypto_backing_device()
        .await
        .ok()
        .filter(|path| path.as_str() != "/")
        && let Ok(backing) = BlockProxy::builder(manager.connection())
            .path(&backing_path)
            .ok()?
            .build()
            .await
        && let Some(fingerprint) = drive_fingerprint(manager, &backing_path, &backing).await
    {
        return Some(fingerprint);
    }

    loop_fingerprint(manager, path, block_id).await
}

async fn drive_fingerprint(
    manager: &DiskManager,
    path: &OwnedObjectPath,
    block: &BlockProxy<'_>,
) -> Option<BlockDeviceFingerprint> {
    let drive = block
        .drive()
        .await
        .ok()
        .filter(|drive| drive.as_str() != "/");
    if let Some(drive_path) = drive {
        let drive = DriveProxy::builder(manager.connection())
            .path(drive_path)
            .ok()?
            .build()
            .await
            .ok()?;
        let wwn = drive.wwn().await.ok()?;
        let serial = drive.serial().await.ok()?;
        let wwn = if wwn.trim().is_empty() {
            nvme_namespace_wwn(manager, path).await?
        } else {
            wwn
        };
        if !wwn.trim().is_empty() && !serial.trim().is_empty() {
            if let Ok(partition) = PartitionProxy::builder(manager.connection())
                .path(path)
                .ok()?
                .build()
                .await
                && let Ok(partition_uuid) = partition.uuid().await
                && !partition_uuid.trim().is_empty()
            {
                return BlockDeviceFingerprint::partition_uuid_bound(partition_uuid, wwn, serial)
                    .ok();
            }
            return BlockDeviceFingerprint::drive_wwn_serial(wwn, serial).ok();
        }
    }
    None
}

/// UDisks exposes a blank `Drive.WWN` for NVMe controllers. Its own Drive
/// interface directs callers to the namespace-level WWN instead. A partition
/// reaches that namespace through `Partition.Table`; a whole namespace already
/// lives at the correct object path.
async fn nvme_namespace_wwn(manager: &DiskManager, path: &OwnedObjectPath) -> Option<String> {
    let mut namespace_path = path.clone();
    if let Ok(builder) = PartitionProxy::builder(manager.connection()).path(path)
        && let Ok(partition) = builder.build().await
        && let Ok(table) = partition.table().await
    {
        namespace_path = table;
    }
    let namespace = NamespaceProxy::builder(manager.connection())
        .path(&namespace_path)
        .ok()?
        .build()
        .await
        .ok()?;
    let wwn = namespace.wwn().await.ok()?;
    (!wwn.trim().is_empty()).then_some(wwn)
}

async fn loop_fingerprint(
    manager: &DiskManager,
    path: &OwnedObjectPath,
    block_id: &BlockDeviceId,
) -> Option<BlockDeviceFingerprint> {
    let proxy = zbus::Proxy::new(
        manager.connection(),
        "org.freedesktop.UDisks2",
        path.as_str(),
        "org.freedesktop.UDisks2.Loop",
    )
    .await
    .ok()?;
    let bytes = proxy.get_property::<Vec<u8>>("BackingFile").await.ok()?;
    let bytes = bytes.split(|byte| *byte == 0).next().unwrap_or_default();
    let backing_file = std::str::from_utf8(bytes).ok()?.trim();
    let metadata = std::fs::metadata(backing_file).ok()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let (major, minor) = block_id.major_minor();
        let device = (major << 32) | minor;
        Some(BlockDeviceFingerprint::loop_backing_file(
            device,
            metadata.ino(),
        ))
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        None
    }
}

pub(crate) async fn resolve_block(
    manager: &DiskManager,
    epoch: u64,
    reference: &BlockDeviceRef,
) -> Result<ResolvedBlock, StorageError> {
    if reference.observed_generation != epoch {
        return Err(conflict(
            "The selected device changed; refresh and select it again.",
        ));
    }
    let matching: Vec<_> = blocks(manager)
        .await?
        .into_iter()
        .filter(|candidate| candidate.id == reference.id)
        .collect();
    if matching.is_empty() {
        return Err(StorageError::new(
            StorageErrorKind::NotFound,
            "The selected block device is no longer present.",
        ));
    }
    let mut fingerprint_matches = matching
        .into_iter()
        .filter(|candidate| candidate.fingerprint.as_ref() == Some(&reference.fingerprint));
    let Some(resolved) = fingerprint_matches.next() else {
        return Err(conflict(
            "The selected device identity changed; no action was sent.",
        ));
    };
    if fingerprint_matches.next().is_some() {
        return Err(conflict(
            "The selected device identity is ambiguous; no action was sent.",
        ));
    }
    Ok(resolved)
}

pub(crate) fn device_major(number: u64) -> u64 {
    ((number >> 8) & 0x0fff) | ((number >> 32) & 0xffff_f000)
}

pub(crate) fn device_minor(number: u64) -> u64 {
    (number & 0x00ff) | ((number >> 12) & 0xffff_ff00)
}

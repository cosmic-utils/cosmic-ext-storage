//! Fresh object-manager resolution for logical actions.

use std::collections::HashMap;

use storage_contracts::{StorageError, StorageErrorKind};
use storage_types::{BlockDeviceFingerprint, BlockDeviceId, BlockDeviceRef};
use udisks2::block::BlockProxy;
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
    pub id_uuid: String,
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
        let fingerprint = if id_uuid.trim().is_empty() || id_type.trim().is_empty() {
            None
        } else {
            BlockDeviceFingerprint::filesystem_uuid(&id_uuid, &id_type).ok()
        };
        blocks.push(ResolvedBlock {
            path,
            id,
            fingerprint,
            device_path,
            id_uuid,
        });
    }
    Ok(blocks)
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

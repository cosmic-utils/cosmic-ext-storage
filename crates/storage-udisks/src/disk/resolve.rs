// SPDX-License-Identifier: GPL-3.0-only

//! Resolve device path or mount point to UDisks2 block object path.
//! Used by domain modules (filesystem, encryption, etc.) to find the D-Bus object for a device.

use std::collections::HashMap;

use crate::dbus::bytestring as bs;
use crate::error::DiskError;
use crate::manager::UDisks2ManagerProxy;
use udisks2::block::BlockProxy;
use udisks2::filesystem::FilesystemProxy;
use zbus::zvariant::OwnedObjectPath;

fn canonicalize_best_effort(p: &str) -> Option<String> {
    std::fs::canonicalize(p)
        .ok()
        .map(|c| c.to_string_lossy().to_string())
}

/// Resolve a device path (e.g. "/dev/sda1") to the UDisks2 block object path.
/// Uses preferred_device or device from Block proxy; matches exact path or canonical path.
pub(crate) async fn block_object_path_for_device(
    device: &str,
) -> Result<OwnedObjectPath, DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let manager_proxy = UDisks2ManagerProxy::new(&connection)
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let block_paths = manager_proxy
        .get_block_devices(HashMap::new())
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let device_canon = canonicalize_best_effort(device);

    for obj in &block_paths {
        let proxy = match BlockProxy::builder(&connection).path(obj)?.build().await {
            Ok(p) => p,
            Err(_) => continue,
        };

        let preferred_device = bs::decode_c_string_bytes(
            &proxy
                .preferred_device()
                .await
                .map_err(|e| DiskError::DBusError(e.to_string()))?,
        );
        let block_device = if preferred_device.is_empty() {
            bs::decode_c_string_bytes(
                &proxy
                    .device()
                    .await
                    .map_err(|e| DiskError::DBusError(e.to_string()))?,
            )
        } else {
            preferred_device
        };

        if block_device.is_empty() {
            continue;
        }

        if block_device == device {
            return Ok(obj.clone());
        }
        if let Some(ref canon) = device_canon
            && let Some(block_canon) = canonicalize_best_effort(&block_device)
            && block_canon == *canon
        {
            return Ok(obj.clone());
        }
    }

    Err(DiskError::DeviceNotFound(device.to_string()))
}

/// Resolve a mount point path (e.g. "/run/media/user/DISK") to the UDisks2 block object path.
/// Used when unmounting by mount point.
pub(crate) async fn block_object_path_for_mount_point(
    mount_point: &str,
) -> Result<OwnedObjectPath, DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let manager_proxy = UDisks2ManagerProxy::new(&connection)
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let block_paths = manager_proxy
        .get_block_devices(HashMap::new())
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    for obj in &block_paths {
        let fs_proxy = match FilesystemProxy::builder(&connection)
            .path(obj)?
            .build()
            .await
        {
            Ok(p) => p,
            Err(_) => continue,
        };

        let mps = match fs_proxy.mount_points().await {
            Ok(m) => m,
            Err(_) => continue,
        };

        let decoded = bs::decode_mount_points(mps);
        if decoded.iter().any(|mp| mp == mount_point) {
            return Ok(obj.clone());
        }
    }

    Err(DiskError::DeviceNotFound(mount_point.to_string()))
}

/// Resolve a block device path (e.g. "/dev/sda") to the UDisks2 drive object path.
/// Used for SMART and other drive-level operations.
pub(crate) async fn drive_object_path_for_device(
    device: &str,
) -> Result<OwnedObjectPath, DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let block_path = block_object_path_for_device(device).await?;
    let block_proxy = BlockProxy::builder(&connection)
        .path(&block_path)?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let drive_path = block_proxy
        .drive()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    if drive_path.as_str() == "/" {
        return Err(DiskError::DeviceNotFound(format!(
            "{} has no associated drive (e.g. loop device)",
            device
        )));
    }

    Ok(drive_path)
}

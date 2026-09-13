// SPDX-License-Identifier: GPL-3.0-only

//! Device-path-based APIs for in-process operations
//!
//! These are convenience functions that accept device paths and internally
//! resolve to UDisks2 block object paths, keeping OwnedObjectPath
//! handling inside storage-udisks (not in in-process operations).

use anyhow::Result;
use std::os::fd::OwnedFd;

/// Open a block device for backup (read-only access) by device path
///
/// This is a convenience wrapper that resolves the device path to a UDisks2
/// block object path and calls open_for_backup with the object path.
pub async fn open_for_backup_by_device(device: &str) -> Result<OwnedFd> {
    let connection = crate::manager::shared_connection().await?;
    open_for_backup_by_device_with_connection(connection.as_ref(), device).await
}

pub(crate) async fn open_for_backup_by_device_with_connection(
    connection: &zbus::Connection,
    device: &str,
) -> Result<OwnedFd> {
    let block_path =
        crate::disk::resolve::block_object_path_for_device_with_connection(connection, device)
            .await?;
    crate::image::backup::open_for_backup_with_connection(connection, block_path).await
}

/// Open a block device for restore (read-write access) by device path
///
/// This is a convenience wrapper that resolves the device path to a UDisks2
/// block object path and calls open_for_restore with the object path.
pub async fn open_for_restore_by_device(device: &str) -> Result<OwnedFd> {
    let connection = crate::manager::shared_connection().await?;
    open_for_restore_by_device_with_connection(connection.as_ref(), device).await
}

pub(crate) async fn open_for_restore_by_device_with_connection(
    connection: &zbus::Connection,
    device: &str,
) -> Result<OwnedFd> {
    let block_path =
        crate::disk::resolve::block_object_path_for_device_with_connection(connection, device)
            .await?;
    crate::image::backup::open_for_restore_with_connection(connection, block_path).await
}

/// Set up a loop device for an image file and return the loop device path
///
/// This function calls the underlying loop_setup (which returns OwnedObjectPath)
/// and extracts the loop device name (e.g., "/dev/loop0") from the object path.
/// The service can then return the device path string directly to clients
/// without parsing object paths.
pub async fn loop_setup_device_path(image_path: &str) -> Result<String> {
    let connection = crate::manager::shared_connection().await?;
    loop_setup_device_path_with_connection(connection.as_ref(), image_path).await
}

pub(crate) async fn loop_setup_device_path_with_connection(
    connection: &zbus::Connection,
    image_path: &str,
) -> Result<String> {
    let object_path =
        crate::image::loop_setup::loop_setup_with_connection(connection, image_path).await?;

    // Extract device name from object path: /org/freedesktop/UDisks2/block_devices/loop0 -> loop0
    let device_name = object_path.as_str().rsplit('/').next().unwrap_or("unknown");

    Ok(format!("/dev/{}", device_name))
}

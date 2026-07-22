// SPDX-License-Identifier: GPL-3.0-only

//! Filesystem label management

use crate::error::DiskError;
use std::collections::HashMap;
use udisks2::{block::BlockProxy, filesystem::FilesystemProxy};
use zbus::zvariant::Value;

/// Get filesystem label for a device
///
/// # Arguments
/// * `device` - Device path (e.g., "/dev/sda1")
///
/// # Returns
/// The filesystem label (may be empty string if no label set)
pub async fn get_filesystem_label(device: &str) -> Result<String, DiskError> {
    let connection = crate::manager::shared_connection().await.map_err(|e| {
        DiskError::ConnectionFailed(format!("Failed to connect to system bus: {}", e))
    })?;

    let block_path = crate::disk::resolve::block_object_path_for_device(device).await?;

    let block_proxy = BlockProxy::builder(&connection)
        .path(&block_path)
        .map_err(|e| DiskError::InvalidPath(format!("Invalid block path: {}", e)))?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let label = block_proxy.id_label().await.unwrap_or_default();

    Ok(label)
}

/// Set filesystem label
pub async fn set_filesystem_label(device_path: &str, label: &str) -> Result<(), DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let fs_path = crate::disk::resolve::block_object_path_for_device(device_path).await?;

    let fs_proxy = FilesystemProxy::builder(&connection)
        .path(&fs_path)
        .map_err(|e| DiskError::DBusError(e.to_string()))?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let opts: HashMap<&str, Value<'_>> = HashMap::new();
    fs_proxy
        .set_label(label, opts)
        .await
        .map_err(|e| DiskError::OperationFailed(format!("Set label failed: {}", e)))?;

    Ok(())
}

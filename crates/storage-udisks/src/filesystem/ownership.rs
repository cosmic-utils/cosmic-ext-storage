// SPDX-License-Identifier: GPL-3.0-only

//! Filesystem ownership management

use crate::error::DiskError;
use std::collections::HashMap;
use udisks2::filesystem::FilesystemProxy;
use zbus::zvariant::Value;

/// Take ownership of a mounted filesystem
///
/// # Arguments
/// * `device` - Device path (e.g., "/dev/sda1")
/// * `recursive` - Take ownership of child mounts
pub async fn take_filesystem_ownership(device: &str, recursive: bool) -> Result<(), DiskError> {
    let connection = crate::manager::shared_connection().await.map_err(|e| {
        DiskError::ConnectionFailed(format!("Failed to connect to system bus: {}", e))
    })?;

    let block_path = crate::disk::resolve::block_object_path_for_device(device).await?;

    let fs_proxy = FilesystemProxy::builder(&connection)
        .path(&block_path)
        .map_err(|e| DiskError::InvalidPath(format!("Invalid filesystem path: {}", e)))?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let mut options: HashMap<&str, Value<'_>> = HashMap::new();
    options.insert("recursive", Value::from(recursive));

    fs_proxy
        .take_ownership(options)
        .await
        .map_err(|e| DiskError::OperationFailed(format!("Take ownership failed: {}", e)))?;

    Ok(())
}

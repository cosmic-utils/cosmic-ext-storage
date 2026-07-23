// SPDX-License-Identifier: GPL-3.0-only

//! Filesystem check and repair operations

use crate::error::DiskError;
use std::collections::HashMap;
use udisks2::filesystem::FilesystemProxy;
use zbus::zvariant::Value;

/// Check and repair a filesystem
///
/// Returns true if filesystem is clean, false if errors were found (and repaired if repair=true)
pub async fn check_filesystem(device_path: &str, repair: bool) -> Result<bool, DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let fs_path = crate::disk::resolve::block_object_path_for_device(device_path).await?;

    let fs_proxy = FilesystemProxy::builder(&connection)
        .path(&fs_path)?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let mut opts: HashMap<&str, Value<'_>> = HashMap::new();
    opts.insert("repair", Value::from(repair));

    let result = fs_proxy
        .check(opts)
        .await
        .map_err(|e| DiskError::OperationFailed(format!("Filesystem check failed: {}", e)))?;

    Ok(result)
}

/// Repair a filesystem
///
/// This is a convenience wrapper around check_filesystem with repair=true
pub async fn repair_filesystem(device_path: &str) -> Result<(), DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let fs_path = crate::disk::resolve::block_object_path_for_device(device_path).await?;

    let fs_proxy = FilesystemProxy::builder(&connection)
        .path(&fs_path)?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let opts: HashMap<&str, Value<'_>> = HashMap::new();
    let ok = fs_proxy
        .repair(opts)
        .await
        .map_err(|e| DiskError::OperationFailed(format!("Filesystem repair failed: {}", e)))?;

    if ok {
        Ok(())
    } else {
        Err(DiskError::OperationFailed(
            "Filesystem repair completed but reported failure".to_string(),
        ))
    }
}

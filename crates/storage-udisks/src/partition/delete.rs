// SPDX-License-Identifier: GPL-3.0-only

//! Partition deletion operations

use crate::error::DiskError;
use std::collections::HashMap;
use udisks2::partition::PartitionProxy;
use zbus::zvariant::{OwnedObjectPath, Value};

/// Delete a partition
///
/// This operation attempts to unmount the partition first before deleting it.
/// If the unmount fails (e.g., partition already unmounted), the error is ignored
/// and deletion proceeds.
pub async fn delete_partition(partition_path: &str) -> Result<(), DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let obj_path: OwnedObjectPath = partition_path
        .try_into()
        .map_err(|e| DiskError::InvalidPath(format!("Invalid partition path: {}", e)))?;

    // Try to unmount first (ignore errors - might not be mounted)
    // This logic merged from volume_model implementation
    let _ = crate::filesystem::unmount_filesystem(partition_path, false).await;

    // Delete the partition
    let partition_proxy = PartitionProxy::builder(&connection)
        .path(&obj_path)
        .map_err(|e| DiskError::DBusError(e.to_string()))?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let options: HashMap<&str, Value<'_>> = HashMap::new();
    partition_proxy
        .delete(options)
        .await
        .map_err(|e| DiskError::OperationFailed(format!("Delete partition failed: {}", e)))?;

    Ok(())
}

// SPDX-License-Identifier: GPL-3.0-only

//! Partition resizing operations

use crate::error::DiskError;
use std::collections::HashMap;
use udisks2::partition::PartitionProxy;
use zbus::zvariant::{OwnedObjectPath, Value};

/// Resize a partition
pub async fn resize_partition(partition_path: &str, new_size: u64) -> Result<(), DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let obj_path: OwnedObjectPath = partition_path
        .try_into()
        .map_err(|e| DiskError::InvalidPath(format!("Invalid partition path: {}", e)))?;

    let partition_proxy = PartitionProxy::builder(&connection)
        .path(&obj_path)
        .map_err(|e| DiskError::DBusError(e.to_string()))?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let options: HashMap<&str, Value<'_>> = HashMap::new();
    partition_proxy
        .resize(new_size, options)
        .await
        .map_err(|e| DiskError::OperationFailed(format!("Resize partition failed: {}", e)))?;

    Ok(())
}

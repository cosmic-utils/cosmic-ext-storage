// SPDX-License-Identifier: GPL-3.0-only

//! Partition editing operations

use crate::error::DiskError;
use enumflags2::BitFlags;
use std::collections::HashMap;
use udisks2::partition::PartitionProxy;
use zbus::zvariant::{OwnedObjectPath, Value};

/// Set partition type
pub async fn set_partition_type(partition_path: &str, type_id: &str) -> Result<(), DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;
    set_partition_type_with_connection(connection.as_ref(), partition_path, type_id).await
}

pub(crate) async fn set_partition_type_with_connection(
    connection: &zbus::Connection,
    partition_path: &str,
    type_id: &str,
) -> Result<(), DiskError> {
    let obj_path: OwnedObjectPath = partition_path
        .try_into()
        .map_err(|e| DiskError::InvalidPath(format!("Invalid partition path: {}", e)))?;

    let partition_proxy = PartitionProxy::builder(connection)
        .path(&obj_path)
        .map_err(|e| DiskError::DBusError(e.to_string()))?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let options: HashMap<&str, Value<'_>> = HashMap::new();
    partition_proxy
        .set_type(type_id, options)
        .await
        .map_err(|e| DiskError::OperationFailed(format!("Set partition type failed: {}", e)))?;

    Ok(())
}

/// Set partition flags
pub async fn set_partition_flags(partition_path: &str, flags: u64) -> Result<(), DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;
    set_partition_flags_with_connection(connection.as_ref(), partition_path, flags).await
}

pub(crate) async fn set_partition_flags_with_connection(
    connection: &zbus::Connection,
    partition_path: &str,
    flags: u64,
) -> Result<(), DiskError> {
    let obj_path: OwnedObjectPath = partition_path
        .try_into()
        .map_err(|e| DiskError::InvalidPath(format!("Invalid partition path: {}", e)))?;

    let partition_proxy = PartitionProxy::builder(connection)
        .path(&obj_path)
        .map_err(|e| DiskError::DBusError(e.to_string()))?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let options: HashMap<&str, Value<'_>> = HashMap::new();
    let flags_bitfield = BitFlags::from_bits_truncate(flags);
    partition_proxy
        .set_flags(flags_bitfield, options)
        .await
        .map_err(|e| DiskError::OperationFailed(format!("Set partition flags failed: {}", e)))?;

    Ok(())
}

/// Set partition name (GPT only)
pub async fn set_partition_name(partition_path: &str, name: &str) -> Result<(), DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;
    set_partition_name_with_connection(connection.as_ref(), partition_path, name).await
}

pub(crate) async fn set_partition_name_with_connection(
    connection: &zbus::Connection,
    partition_path: &str,
    name: &str,
) -> Result<(), DiskError> {
    let obj_path: OwnedObjectPath = partition_path
        .try_into()
        .map_err(|e| DiskError::InvalidPath(format!("Invalid partition path: {}", e)))?;

    let partition_proxy = PartitionProxy::builder(connection)
        .path(&obj_path)
        .map_err(|e| DiskError::DBusError(e.to_string()))?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let options: HashMap<&str, Value<'_>> = HashMap::new();
    partition_proxy
        .set_name(name, options)
        .await
        .map_err(|e| DiskError::OperationFailed(format!("Set partition name failed: {}", e)))?;

    Ok(())
}

/// Edit partition (all-in-one: type, name, and flags)
///
/// This is a convenience function that sets partition type, name, and flags
/// in sequence. Useful for UI workflows where all three need to be updated
/// together.
pub async fn edit_partition(
    partition_path: &str,
    partition_type: &str,
    name: &str,
    flags: u64,
) -> Result<(), DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;
    edit_partition_with_connection(
        connection.as_ref(),
        partition_path,
        partition_type,
        name,
        flags,
    )
    .await
}

pub(crate) async fn edit_partition_with_connection(
    connection: &zbus::Connection,
    partition_path: &str,
    partition_type: &str,
    name: &str,
    flags: u64,
) -> Result<(), DiskError> {
    // Merged from volume_model - convenient all-in-one operation
    crate::partition::edit::set_partition_type_with_connection(
        connection,
        partition_path,
        partition_type,
    )
    .await?;
    crate::partition::edit::set_partition_name_with_connection(connection, partition_path, name)
        .await?;
    crate::partition::edit::set_partition_flags_with_connection(connection, partition_path, flags)
        .await?;
    Ok(())
}

// SPDX-License-Identifier: GPL-3.0-only

//! Partition creation operations

use crate::error::DiskError;
use std::collections::HashMap;
use storage_types::CreatePartitionInfo;
use udisks2::{block::BlockProxy, partitiontable::PartitionTableProxy};
use zbus::zvariant::{OwnedObjectPath, Value};

/// Create a partition table on a disk
pub async fn create_partition_table(disk_path: &str, table_type: &str) -> Result<(), DiskError> {
    let _connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    // Use the new flat format_disk function from disk module
    crate::disk::format::format_disk(disk_path.to_string(), table_type, false)
        .await
        .map_err(|e| DiskError::OperationFailed(format!("Format disk failed: {}", e)))?;

    Ok(())
}

/// Create a partition (low-level, no formatting)
pub async fn create_partition(
    disk_path: &str,
    offset: u64,
    size: u64,
    type_id: &str,
) -> Result<String, DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let block_path: OwnedObjectPath = disk_path
        .try_into()
        .map_err(|e| DiskError::InvalidPath(format!("Invalid device path: {}", e)))?;

    // Create partition table proxy
    let table_proxy = PartitionTableProxy::builder(&connection)
        .path(&block_path)
        .map_err(|e| DiskError::DBusError(e.to_string()))?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    // Create partition
    let options: HashMap<&str, Value<'_>> = HashMap::new();
    let partition_path = table_proxy
        .create_partition(offset, size, type_id, "", options)
        .await
        .map_err(|e| DiskError::OperationFailed(format!("Create partition failed: {}", e)))?;

    // Get device path
    let block_proxy = BlockProxy::builder(&connection)
        .path(&partition_path)
        .map_err(|e| DiskError::DBusError(e.to_string()))?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let device_bytes = block_proxy
        .preferred_device()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let device_path = String::from_utf8(device_bytes.into_iter().filter(|&b| b != 0).collect())
        .unwrap_or_else(|_| {
            format!(
                "/dev/{}",
                partition_path
                    .as_str()
                    .rsplit('/')
                    .next()
                    .unwrap_or("unknown")
            )
        });

    Ok(device_path)
}

/// Create a partition with optional filesystem formatting.
///
/// This is a high-level function that handles the complete partition creation flow:
/// 1. Creates the partition
/// 2. If `filesystem_type` is set and not empty:
///    - If LUKS is requested: formats as LUKS, unlocks, then formats the cleartext device
///    - Otherwise: formats the partition directly
/// 3. Returns the final device path
pub async fn create_partition_with_filesystem(
    disk_path: &str,
    info: &CreatePartitionInfo,
) -> Result<String, DiskError> {
    // Step 1: Create the partition
    let partition_path =
        create_partition(disk_path, info.offset, info.size, &info.selected_type).await?;

    tracing::info!(
        "Created partition {} at offset {}, size {}",
        partition_path,
        info.offset,
        info.size
    );

    // Step 2: Handle formatting if filesystem_type is specified
    let fs_type = info.filesystem_type.trim();
    if fs_type.is_empty() {
        // No filesystem requested, just return the partition path
        return Ok(partition_path);
    }

    // Step 3: Format based on whether LUKS is requested
    if info.password_protected && !info.password.is_empty() {
        // LUKS + filesystem flow
        tracing::info!("Formatting {} as LUKS", partition_path);

        // Format as LUKS
        crate::format_luks(&partition_path, &info.password, "luks2").await?;

        // After formatting with encrypt.passphrase, UDisks2 auto-unlocks the device
        // Get the cleartext device path from the Encrypted interface
        let cleartext_path = match crate::encryption::get_cleartext_device(&partition_path).await {
            Ok(path) if !path.is_empty() && path != "/" => {
                tracing::info!("Using auto-unlocked cleartext device: {}", path);
                path
            }
            _ => {
                // Not auto-unlocked, unlock manually
                tracing::info!("Unlocking LUKS device {}", partition_path);
                crate::unlock_luks(&partition_path, &info.password).await?
            }
        };

        tracing::info!(
            "Formatting cleartext device {} as {}",
            cleartext_path,
            fs_type
        );

        // Format the cleartext device with the requested filesystem
        crate::format_filesystem(
            &cleartext_path,
            fs_type,
            &info.name,
            storage_types::FormatOptions::default(),
        )
        .await?;

        Ok(partition_path)
    } else {
        // Direct filesystem formatting (no LUKS)
        tracing::info!("Formatting {} as {}", partition_path, fs_type);

        let options = storage_types::FormatOptions {
            label: info.name.clone(),
            force: false,
            erase: info.erase,
            discard: false,
            ..Default::default()
        };

        crate::format_filesystem(&partition_path, fs_type, &info.name, options).await?;

        Ok(partition_path)
    }
}

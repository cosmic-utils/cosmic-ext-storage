// SPDX-License-Identifier: GPL-3.0-only

//! Helper functions for loading UiDrive instances from in-process operations

use super::UiDrive;
use crate::operations::{DisksClient, error::OperationError};
use std::time::Instant;
use storage_types::DiskInfo;

pub async fn load_drive_candidates() -> Result<Vec<DiskInfo>, OperationError> {
    DisksClient::new().await?.list_disks().await
}

pub async fn load_drive_candidates_with_operations(
    operations: std::sync::Arc<crate::operations::StorageOperations>,
) -> Result<Vec<DiskInfo>, OperationError> {
    DisksClient::with_operations(operations).list_disks().await
}

pub async fn build_drive_timed(disk: DiskInfo) -> (Result<UiDrive, String>, u128) {
    let operations = match crate::operations::shared().await {
        Ok(operations) => operations,
        Err(error) => return (Err(error.to_string()), 0),
    };
    build_drive_timed_with_operations(disk, operations).await
}

pub async fn build_drive_timed_with_operations(
    disk: DiskInfo,
    operations: std::sync::Arc<crate::operations::StorageOperations>,
) -> (Result<UiDrive, String>, u128) {
    let started = Instant::now();
    let device = disk.device.clone();
    let result = UiDrive::with_operations(disk, operations)
        .await
        .map_err(|error| error.to_string());
    let elapsed_ms = started.elapsed().as_millis();
    match &result {
        Ok(_) => tracing::info!(%device, elapsed_ms, "drive build complete"),
        Err(error) => tracing::warn!(%device, elapsed_ms, %error, "drive build failed"),
    }
    (result, elapsed_ms)
}

/// Load all drives from in-process operations as UiDrive instances
///
/// Each UiDrive is created with its own client and initial data load.
///
/// # Example
/// ```no_run
/// let drives = load_all_drives().await?;
/// for drive in drives {
///     println!("Drive: {} ({} volumes)", drive.device(), drive.volumes.len());
/// }
/// ```
pub async fn load_all_drives() -> Result<Vec<UiDrive>, OperationError> {
    load_all_drives_with_operations(crate::operations::shared().await?).await
}

/// Refresh the same UI models without falling back to a global adapter context.
pub async fn load_all_drives_with_operations(
    operations: std::sync::Arc<crate::operations::StorageOperations>,
) -> Result<Vec<UiDrive>, OperationError> {
    let disks = DisksClient::with_operations(operations.clone())
        .list_disks()
        .await?;

    let mut drives = Vec::new();
    for disk in disks {
        match UiDrive::with_operations(disk, operations.clone()).await {
            Ok(drive) => drives.push(drive),
            Err(e) => {
                tracing::warn!("Failed to load drive data: {}", e);
                // Continue with other drives even if one fails
            }
        }
    }

    Ok(drives)
}

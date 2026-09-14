// SPDX-License-Identifier: GPL-3.0-only

//! Helper functions for loading UiDrive instances from in-process operations

use super::UiDrive;
use crate::operations::{DisksClient, error::OperationError};
use std::time::Instant;
use storage_types::DiskInfo;

pub async fn load_drive_candidates() -> Result<Vec<DiskInfo>, OperationError> {
    DisksClient::new().await?.list_disks().await
}

pub async fn build_drive_timed(disk: DiskInfo) -> (Result<UiDrive, String>, u128) {
    let started = Instant::now();
    let device = disk.device.clone();
    let result = UiDrive::new(disk).await.map_err(|error| error.to_string());
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
    let disks = load_drive_candidates().await?;

    let mut drives = Vec::new();
    for disk in disks {
        match UiDrive::new(disk).await {
            Ok(drive) => drives.push(drive),
            Err(e) => {
                tracing::warn!("Failed to load drive data: {}", e);
                // Continue with other drives even if one fails
            }
        }
    }

    Ok(drives)
}

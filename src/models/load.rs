// SPDX-License-Identifier: GPL-3.0-only

//! Helper functions for loading UiDrive instances from in-process operations

use super::UiDrive;
use crate::operations::{DisksClient, error::OperationError};

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
    let client = DisksClient::new().await?;
    let disks = client.list_disks().await?;

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

//! Image backup/restore via in-process operations (Phase 1: start operation, return operation_id).
//! Progress and completion are handled by subscription in the app.

use crate::models::UiDrive;
use crate::operations::{FilesystemsClient, ImageClient};
use crate::state::dialogs::ImageOperationKind;
use storage_types::VolumeInfo;

/// Start a backup or restore operation via the in-process operations.
/// Returns the operation_id for progress tracking and cancel.
/// Caller is responsible for unmounting before restore (this function does it).
pub(super) async fn start_image_operation(
    operations: std::sync::Arc<crate::operations::StorageOperations>,
    kind: ImageOperationKind,
    drive: UiDrive,
    partition: Option<VolumeInfo>,
    image_path: String,
) -> anyhow::Result<String> {
    let image_client = ImageClient::with_operations(operations.clone());

    match kind {
        ImageOperationKind::CreateFromDrive => {
            let device = &drive.disk.device;
            let operation_id = image_client
                .backup_drive(device, &image_path)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to start backup: {}", e))?;
            Ok(operation_id)
        }
        ImageOperationKind::CreateFromPartition => {
            let Some(ref p) = partition else {
                anyhow::bail!("No partition selected");
            };
            let device = p
                .device_path
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Partition has no device path"))?;
            let operation_id = image_client
                .backup_partition(device, &image_path)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to start backup: {}", e))?;
            Ok(operation_id)
        }
        ImageOperationKind::RestoreToDrive => {
            let fs_client = FilesystemsClient::with_operations(operations.clone());
            for p in &drive.volumes_flat {
                if p.is_mounted() {
                    let device = p
                        .volume
                        .device_path
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("Partition has no device path"))?;
                    unmount_for_restore(&fs_client, device).await?;
                }
            }
            let device = &drive.disk.device;
            let operation_id = image_client
                .restore_drive(device, &image_path)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to start restore: {}", e))?;
            Ok(operation_id)
        }
        ImageOperationKind::RestoreToPartition => {
            let Some(ref p) = partition else {
                anyhow::bail!("No partition selected");
            };
            if p.is_mounted() {
                let fs_client = FilesystemsClient::with_operations(operations.clone());
                let device = p
                    .device_path
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Partition has no device path"))?;
                unmount_for_restore(&fs_client, device).await?;
            }
            let device = p
                .device_path
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Partition has no device path"))?;
            let operation_id = image_client
                .restore_partition(device, &image_path)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to start restore: {}", e))?;
            Ok(operation_id)
        }
    }
}

async fn unmount_for_restore(client: &FilesystemsClient, device: &str) -> anyhow::Result<()> {
    let result = client.unmount(device, false, false).await?;
    if !result.success {
        anyhow::bail!(
            "Failed to unmount {device}: {}",
            result.error.unwrap_or_else(|| "device is busy".into())
        );
    }
    Ok(())
}

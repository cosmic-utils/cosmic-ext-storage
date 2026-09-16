use crate::models::{UiDrive, load::load_all_drives_with_operations};
use cosmic::Task;
use std::future::Future;

use crate::app::Message;
use crate::operations::FilesystemsClient;
use crate::state::dialogs::{ShowDialog, UnmountBusyDialog};
use storage_types::MountOptions;

use crate::state::volumes::VolumesControl;

/// Generic helper for volume mount/unmount operations
fn perform_volume_operation<F, Fut>(
    operation: F,
    operation_name: &'static str,
    preserve_selection: Option<String>,
    operations: std::sync::Arc<crate::operations::StorageOperations>,
) -> Task<cosmic::Action<Message>>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<()>> + Send,
{
    Task::perform(
        async move {
            operation().await.map_err(|e| anyhow::anyhow!(e))?;
            load_all_drives_with_operations(operations)
                .await
                .map_err(|e| anyhow::anyhow!(e))
        },
        move |result: Result<Vec<UiDrive>, anyhow::Error>| match result {
            Ok(drives) => {
                // Pass the selected volume to preserve selection after reload
                Message::UpdateNavWithChildSelection(drives, preserve_selection.clone()).into()
            }
            Err(e) => {
                tracing::error!(?e, "{operation_name} failed");
                Message::None.into()
            }
        },
    )
}

pub(super) fn mount(
    control: &mut VolumesControl,
    operations: std::sync::Arc<crate::operations::StorageOperations>,
) -> Task<cosmic::Action<Message>> {
    let Some(volume) = control
        .segments
        .get(control.selected_segment)
        .and_then(|s| s.volume.clone())
    else {
        return Task::none();
    };

    let device = volume
        .device_path
        .clone()
        .unwrap_or_else(|| volume.label.clone());
    let device_path_for_selection = device.clone();

    let client = FilesystemsClient::with_operations(operations.clone());
    perform_volume_operation(
        || async move {
            client.mount(&device, "", MountOptions::default()).await?;
            Ok(())
        },
        "mount",
        Some(device_path_for_selection),
        operations,
    )
}

// Helper enum to distinguish busy errors from generic errors
#[derive(Debug)]
enum UnmountResult {
    Success(Vec<crate::models::UiDrive>),
    Busy {
        device: String,
        mount_point: String,
        processes: Vec<storage_types::ProcessInfo>,
        device_path: String,
    },
    GenericError(String),
}

pub(super) fn unmount(
    control: &mut VolumesControl,
    operations: std::sync::Arc<crate::operations::StorageOperations>,
) -> Task<cosmic::Action<Message>> {
    let Some(volume) = control
        .segments
        .get(control.selected_segment)
        .and_then(|s| s.volume.clone())
    else {
        return Task::none();
    };

    let device = volume
        .device_path
        .clone()
        .unwrap_or_else(|| volume.label.clone());
    let mount_point = volume.mount_points.first().cloned();
    let device_path = volume
        .device_path
        .clone()
        .unwrap_or_else(|| volume.label.clone());
    unmount_device(operations, device, mount_point, device_path, false)
}

pub(super) fn child_mount(
    control: &mut VolumesControl,
    device_path: String,
    operations: std::sync::Arc<crate::operations::StorageOperations>,
) -> Task<cosmic::Action<Message>> {
    let Some(node) =
        crate::state::volumes::find_volume_in_ui_tree(&control.volumes, &device_path).cloned()
    else {
        return Task::none();
    };

    let device = node
        .volume
        .device_path
        .clone()
        .unwrap_or_else(|| device_path.clone());
    let device_path_for_selection = device_path.clone();

    let client = FilesystemsClient::with_operations(operations.clone());
    perform_volume_operation(
        || async move {
            let _mount_point = client
                .mount(&device, "", MountOptions::default())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to mount: {}", e))?;
            Ok(())
        },
        "child mount",
        Some(device_path_for_selection),
        operations,
    )
}

pub(crate) fn child_unmount(
    control: &VolumesControl,
    device_path: String,
    operations: std::sync::Arc<crate::operations::StorageOperations>,
) -> Task<cosmic::Action<Message>> {
    let Some(node) =
        crate::state::volumes::find_volume_in_ui_tree(&control.volumes, &device_path).cloned()
    else {
        return Task::none();
    };

    let device = node
        .volume
        .device_path
        .clone()
        .unwrap_or_else(|| device_path.clone());
    let mount_point = node.volume.mount_points.first().cloned();
    unmount_device(operations, device, mount_point, device_path, false)
}

/// One production path for segment, child, sidebar and busy-dialog retries.
pub(crate) fn unmount_device(
    operations: std::sync::Arc<crate::operations::StorageOperations>,
    device: String,
    mount_point: Option<String>,
    device_path: String,
    kill_processes: bool,
) -> Task<cosmic::Action<Message>> {
    let device_path_for_selection = device_path.clone();
    let device_path_for_retry = device_path.clone();

    Task::perform(
        async move {
            let client = FilesystemsClient::with_operations(operations.clone());

            let unmount_result = match client.unmount(&device, false, kill_processes).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(?e, "Failed to unmount");
                    return UnmountResult::GenericError(e.to_string());
                }
            };

            if unmount_result.success {
                // Success - reload drives
                match load_all_drives_with_operations(operations).await {
                    Ok(drives) => UnmountResult::Success(drives),
                    Err(e) => {
                        tracing::error!(?e, "Failed to reload drives");
                        UnmountResult::GenericError(e.to_string())
                    }
                }
            } else if !unmount_result.blocking_processes.is_empty() {
                // Device is busy with processes
                let mp = mount_point.unwrap_or_default();
                UnmountResult::Busy {
                    device,
                    mount_point: mp,
                    processes: unmount_result.blocking_processes,
                    device_path: device_path_for_retry,
                }
            } else {
                // Generic error
                UnmountResult::GenericError(
                    unmount_result
                        .error
                        .unwrap_or_else(|| "Unknown unmount failure".into()),
                )
            }
        },
        move |result| match result {
            UnmountResult::Success(drives) => Message::UpdateNavWithChildSelection(
                drives,
                Some(device_path_for_selection.clone()),
            )
            .into(),
            UnmountResult::Busy {
                device,
                mount_point,
                processes,
                device_path,
            } => {
                // Show busy dialog
                Message::Dialog(Box::new(ShowDialog::UnmountBusy(UnmountBusyDialog {
                    device,
                    mount_point,
                    processes,
                    device_path,
                })))
                .into()
            }
            UnmountResult::GenericError(error) => Message::Dialog(Box::new(ShowDialog::Info {
                title: crate::fl!("unmount-failed"),
                body: error,
            }))
            .into(),
        },
    )
}

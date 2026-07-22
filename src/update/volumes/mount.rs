use crate::models::{UiDrive, load_all_drives};
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
) -> Task<cosmic::Action<Message>>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<()>> + Send,
{
    Task::perform(
        async move {
            operation().await.map_err(|e| anyhow::anyhow!(e))?;
            load_all_drives().await.map_err(|e| anyhow::anyhow!(e))
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

pub(super) fn mount(control: &mut VolumesControl) -> Task<cosmic::Action<Message>> {
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

    perform_volume_operation(
        || async move {
            let client = FilesystemsClient::new().await?;
            client.mount(&device, "", MountOptions::default()).await?;
            Ok(())
        },
        "mount",
        Some(device_path_for_selection),
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
    GenericError,
}

pub(super) fn unmount(control: &mut VolumesControl) -> Task<cosmic::Action<Message>> {
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
    let device_path_for_retry = device_path.clone();

    Task::perform(
        async move {
            let client = match FilesystemsClient::new().await {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!(?e, "Failed to create client");
                    return UnmountResult::GenericError;
                }
            };

            let unmount_result = match client.unmount(&device, false, false).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(?e, "Failed to unmount");
                    return UnmountResult::GenericError;
                }
            };

            if unmount_result.success {
                // Success - reload drives
                match load_all_drives().await {
                    Ok(drives) => UnmountResult::Success(drives),
                    Err(e) => {
                        tracing::error!(?e, "Failed to reload drives");
                        UnmountResult::GenericError
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
                if let Some(err) = unmount_result.error {
                    tracing::error!("unmount failed: {}", err);
                } else {
                    tracing::error!("unmount failed with unknown error");
                }
                UnmountResult::GenericError
            }
        },
        move |result| match result {
            UnmountResult::Success(drives) => {
                Message::UpdateNavWithChildSelection(drives, Some(device_path.clone())).into()
            }
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
            UnmountResult::GenericError => {
                // Generic error already logged
                Message::None.into()
            }
        },
    )
}

pub(super) fn child_mount(
    control: &mut VolumesControl,
    device_path: String,
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

    perform_volume_operation(
        || async move {
            let client = FilesystemsClient::new()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create client: {}", e))?;
            let _mount_point = client
                .mount(&device, "", MountOptions::default())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to mount: {}", e))?;
            Ok(())
        },
        "child mount",
        Some(device_path_for_selection),
    )
}

pub(super) fn child_unmount(
    control: &mut VolumesControl,
    device_path: String,
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
    let device_path_for_selection = device_path.clone();
    let device_path_for_retry = device_path.clone();

    Task::perform(
        async move {
            let client = match FilesystemsClient::new().await {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!(?e, "Failed to create client");
                    return UnmountResult::GenericError;
                }
            };

            let unmount_result = match client.unmount(&device, false, false).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(?e, "Failed to unmount");
                    return UnmountResult::GenericError;
                }
            };

            if unmount_result.success {
                // Success - reload drives
                match load_all_drives().await {
                    Ok(drives) => UnmountResult::Success(drives),
                    Err(e) => {
                        tracing::error!(?e, "Failed to reload drives");
                        UnmountResult::GenericError
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
                if let Some(err) = unmount_result.error {
                    tracing::error!("child unmount failed: {}", err);
                } else {
                    tracing::error!("child unmount failed with unknown error");
                }
                UnmountResult::GenericError
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
            UnmountResult::GenericError => {
                // Generic error already logged
                Message::None.into()
            }
        },
    )
}

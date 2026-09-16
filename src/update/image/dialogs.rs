use crate::fl;
use crate::message::dialogs::{
    AttachDiskImageDialogMessage, AttachDiskResult, ImageOperationDialogMessage,
    NewDiskImageDialogMessage,
};
use crate::models::load_all_drives;
use crate::state::dialogs::{AttachDiskImageDialog, NewDiskImageDialog, ShowDialog};
use cosmic::app::Task;
use tokio::fs::OpenOptions;

use super::ops::start_image_operation;
use crate::message::app::Message;
use crate::operations::{FilesystemsClient, ImageClient};
use crate::state::app::AppModel;
use storage_types::MountOptions;

pub(super) fn new_disk_image(app: &mut AppModel) {
    app.dialog = Some(ShowDialog::NewDiskImage(Box::new(NewDiskImageDialog {
        path: String::new(),
        size_bytes: 16 * 1024 * 1024,
        running: false,
        error: None,
    })));
}

pub(super) fn attach_disk(app: &mut AppModel) {
    app.dialog = Some(ShowDialog::AttachDiskImage(Box::new(
        AttachDiskImageDialog {
            path: String::new(),
            running: false,
            error: None,
        },
    )));
}

pub(super) fn new_disk_image_dialog(
    app: &mut AppModel,
    msg: NewDiskImageDialogMessage,
) -> Task<Message> {
    let Some(ShowDialog::NewDiskImage(state)) = app.dialog.as_mut() else {
        return Task::none();
    };

    match msg {
        NewDiskImageDialogMessage::SizeUpdate(v) => state.size_bytes = v,
        NewDiskImageDialogMessage::Cancel => {
            if !state.running {
                app.dialog = None;
            }
        }
        NewDiskImageDialogMessage::Create => {
            if state.running {
                return Task::none();
            }

            let path = state.path.clone();
            let size_bytes = state.size_bytes;

            state.running = true;
            state.error = None;

            return Task::perform(
                async move {
                    if path.trim().is_empty() {
                        anyhow::bail!("Destination path is required");
                    }

                    let file = OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&path)
                        .await?;
                    file.set_len(size_bytes).await?;
                    Ok(())
                },
                |res: anyhow::Result<()>| {
                    Message::NewDiskImageDialog(NewDiskImageDialogMessage::Complete(
                        res.map_err(|e| e.to_string()),
                    ))
                    .into()
                },
            );
        }
        NewDiskImageDialogMessage::Complete(res) => {
            state.running = false;
            match res {
                Ok(()) => {
                    app.dialog = Some(ShowDialog::Info {
                        title: fl!("app-title"),
                        body: "Disk image created.".to_string(),
                    });
                }
                Err(e) => {
                    tracing::error!(%e, "new disk image dialog error");
                    state.error = Some(e);
                }
            }
        }
    }

    Task::none()
}

pub(super) fn attach_disk_image_dialog(
    app: &mut AppModel,
    msg: AttachDiskImageDialogMessage,
) -> Task<Message> {
    let Some(ShowDialog::AttachDiskImage(state)) = app.dialog.as_mut() else {
        return Task::none();
    };

    match msg {
        AttachDiskImageDialogMessage::Cancel => {
            if !state.running {
                app.dialog = None;
            }
        }
        AttachDiskImageDialogMessage::Attach => {
            if state.running {
                return Task::none();
            }

            let path = state.path.clone();
            state.running = true;
            state.error = None;

            return Task::perform(
                async move {
                    if path.trim().is_empty() {
                        anyhow::bail!("Image file path is required");
                    }

                    let image_client = ImageClient::new()
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to create image client: {}", e))?;
                    let device_name = image_client
                        .loop_setup(&path)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to set up loop device: {}", e))?;
                    let device_path = format!("/dev/{}", device_name);

                    let fs_client = FilesystemsClient::new().await.map_err(|e| {
                        anyhow::anyhow!("Failed to create filesystems client: {}", e)
                    })?;
                    match fs_client
                        .mount(&device_path, "", MountOptions::default())
                        .await
                    {
                        Ok(_mount_point) => Ok(AttachDiskResult {
                            mounted: true,
                            message: "Attached and mounted image.".to_string(),
                        }),
                        Err(e) => {
                            tracing::warn!(%e, "attach image: mount attempt failed");
                            Ok(AttachDiskResult {
                                mounted: false,
                                message: "Attached image. If it contains partitions, select and mount them from the main view.".to_string(),
                            })
                        }
                    }
                },
                |res: anyhow::Result<AttachDiskResult>| {
                    Message::AttachDiskImageDialog(AttachDiskImageDialogMessage::Complete(
                        res.map_err(|e| e.to_string()),
                    ))
                    .into()
                },
            );
        }
        AttachDiskImageDialogMessage::Complete(res) => {
            state.running = false;
            match res {
                Ok(r) => {
                    app.dialog = Some(ShowDialog::Info {
                        title: fl!("app-title"),
                        body: r.message,
                    });

                    return Task::perform(async { load_all_drives().await.ok() }, |drives| {
                        match drives {
                            None => Message::None.into(),
                            Some(drives) => Message::UpdateNav(drives, None).into(),
                        }
                    });
                }
                Err(e) => {
                    tracing::error!(%e, "attach disk image dialog error");
                    state.error = Some(e);
                }
            }
        }
    }

    Task::none()
}

pub(super) fn image_operation_dialog(
    app: &mut AppModel,
    msg: ImageOperationDialogMessage,
) -> Task<Message> {
    let operations = app.runtime.operations();
    let client = ImageClient::with_operations(operations.clone());
    // A discarded start may already have created an operation. Cancel and
    // release only that owned operation; never bind it to a replacement dialog.
    if let ImageOperationDialogMessage::Started { request_id, result } = &msg
        && !matches!(&app.dialog, Some(ShowDialog::ImageOperation(state)) if state.running && state.request_id == Some(*request_id))
    {
        if let Ok(operation_id) = result {
            let operation_id = operation_id.clone();
            return Task::perform(
                async move {
                    client.cancel_operation(&operation_id).await?;
                    client.forget_operation(&operation_id).await
                },
                |result: Result<(), crate::operations::OperationError>| {
                    if let Err(error) = result {
                        tracing::warn!(%error, "Discarded image operation cleanup failed");
                    }
                    Message::None.into()
                },
            );
        }
        return Task::none();
    }
    let Some(ShowDialog::ImageOperation(state)) = app.dialog.as_mut() else {
        return Task::none();
    };
    match msg {
        ImageOperationDialogMessage::CancelOperation => {
            if state.running {
                state.cancel_requested = true;
                if let Some(operation_id) = state.operation_id.clone() {
                    return cancel_image(client, operation_id);
                }
            } else {
                app.dialog = None;
            }
        }
        ImageOperationDialogMessage::Start => {
            if state.running {
                return Task::none();
            }
            let image_path = state.image_path.clone();
            if image_path.trim().is_empty() {
                state.error = Some("Image path is required".into());
                return Task::none();
            }
            let kind = state.kind;
            let drive = state.drive.clone();
            let partition = state.partition.clone();
            let request_id = uuid::Uuid::new_v4();
            state.request_id = Some(request_id);
            state.cancel_requested = false;
            state.running = true;
            state.error = None;
            return Task::perform(
                async move {
                    start_image_operation(operations, kind, drive, partition, image_path)
                        .await
                        .map_err(|e| e.to_string())
                },
                move |result| {
                    Message::ImageOperationDialog(ImageOperationDialogMessage::Started {
                        request_id,
                        result,
                    })
                    .into()
                },
            );
        }
        ImageOperationDialogMessage::Started { result, .. } => match result {
            Ok(operation_id) => {
                app.image_op_operation_id = Some(operation_id.clone());
                state.operation_id = Some(operation_id.clone());
                if state.cancel_requested {
                    return cancel_image(client, operation_id);
                }
            }
            Err(error) => {
                state.running = false;
                state.request_id = None;
                state.error = Some(error);
            }
        },
        ImageOperationDialogMessage::CancelCompleted {
            operation_id,
            result,
        } => {
            if state.operation_id.as_ref() == Some(&operation_id)
                && let Err(error) = result
            {
                state.cancel_requested = false;
                state.error = Some(error);
            }
        }
        ImageOperationDialogMessage::Progress(op_id, bytes, total, speed) => {
            if state.running && state.operation_id.as_deref() == Some(op_id.as_str()) {
                state.progress = Some((bytes, total, speed));
            }
        }
        ImageOperationDialogMessage::Complete {
            operation_id,
            result,
        } => {
            if !state.running || state.operation_id.as_ref() != Some(&operation_id) {
                return Task::none();
            }
            state.running = false;
            state.request_id = None;
            state.operation_id = None;
            state.progress = None;
            app.image_op_operation_id = None;
            let cleanup = Task::perform(
                async move { client.forget_operation(&operation_id).await },
                |result| {
                    if let Err(error) = result {
                        tracing::warn!(%error, "Image operation cleanup failed");
                    }
                    Message::None.into()
                },
            );
            match result {
                Ok(()) => {
                    app.dialog = Some(ShowDialog::Info {
                        title: fl!("app-title"),
                        body: fl!("ok"),
                    });
                    let refresh = Task::perform(
                        async move {
                            crate::models::load::load_all_drives_with_operations(operations).await
                        },
                        |result| match result {
                            Ok(drives) => Message::UpdateNav(drives, None).into(),
                            Err(error) => Message::Dialog(Box::new(ShowDialog::Info {
                                title: "Refresh failed".into(),
                                body: error.to_string(),
                            }))
                            .into(),
                        },
                    );
                    return Task::batch([cleanup, refresh]);
                }
                Err(error) => {
                    state.error = Some(if error.to_lowercase().contains("cancelled") {
                        fl!("operation-cancelled")
                    } else {
                        error
                    });
                    return cleanup;
                }
            }
        }
    }
    Task::none()
}

fn cancel_image(client: ImageClient, operation_id: String) -> Task<Message> {
    let target = operation_id.clone();
    Task::perform(
        async move {
            client
                .cancel_operation(&target)
                .await
                .map_err(|error| error.to_string())
        },
        move |result| {
            Message::ImageOperationDialog(ImageOperationDialogMessage::CancelCompleted {
                operation_id: operation_id.clone(),
                result,
            })
            .into()
        },
    )
}

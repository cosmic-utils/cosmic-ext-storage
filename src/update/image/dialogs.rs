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
    let Some(ShowDialog::ImageOperation(state)) = app.dialog.as_mut() else {
        return Task::none();
    };

    match msg {
        ImageOperationDialogMessage::CancelOperation => {
            if state.running {
                if let Some(operation_id) = state.operation_id.clone() {
                    return Task::perform(
                        async move {
                            let client = ImageClient::new().await?;
                            client.cancel_operation(&operation_id).await?;
                            Ok::<(), crate::operations::error::OperationError>(())
                        },
                        |_| Message::None.into(),
                    );
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
                let e = "Image path is required".to_string();
                tracing::warn!(%e, "image operation dialog validation error");
                state.error = Some(e);
                return Task::none();
            }

            let kind = state.kind;
            let drive = state.drive.clone();
            let partition = state.partition.clone();

            state.running = true;
            state.error = None;

            return Task::perform(
                async move { start_image_operation(kind, drive, partition, image_path).await },
                |res: anyhow::Result<String>| match res {
                    Ok(operation_id) => Message::ImageOperationStarted(operation_id).into(),
                    Err(e) => Message::ImageOperationDialog(ImageOperationDialogMessage::Complete(
                        Err(e.to_string()),
                    ))
                    .into(),
                },
            );
        }
        ImageOperationDialogMessage::Progress(op_id, bytes, total, speed) => {
            if state.operation_id.as_deref() == Some(op_id.as_str()) {
                state.progress = Some((bytes, total, speed));
            }
        }
        ImageOperationDialogMessage::Complete(res) => {
            state.running = false;
            state.operation_id = None;
            state.progress = None;
            app.image_op_operation_id = None;

            match res {
                Ok(()) => {
                    app.dialog = Some(ShowDialog::Info {
                        title: fl!("app-title"),
                        body: fl!("ok"),
                    });

                    return Task::perform(async { load_all_drives().await.ok() }, |drives| {
                        match drives {
                            None => Message::None.into(),
                            Some(drives) => Message::UpdateNav(drives, None).into(),
                        }
                    });
                }
                Err(e) => {
                    tracing::error!(%e, "image operation dialog error");
                    let msg = if e.to_lowercase().contains("cancelled") {
                        fl!("operation-cancelled")
                    } else {
                        e
                    };
                    state.error = Some(msg);
                }
            }
        }
    }

    Task::none()
}

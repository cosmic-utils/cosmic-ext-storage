use crate::models::{UiDrive, load_all_drives};
use cosmic::Task;

use crate::app::Message;
use crate::errors::ui::{UiErrorContext, log_error_and_show_dialog};
use crate::fl;
use crate::message::dialogs::EditFilesystemLabelMessage;
use crate::operations::filesystems::FilesystemsClient;
use crate::state::dialogs::{
    ConfirmActionDialog, EditFilesystemLabelDialog, FilesystemTarget, ShowDialog,
};

use crate::message::volumes::VolumesControlMessage;
use crate::state::volumes::VolumesControl;

pub(super) fn open_edit_filesystem_label(
    control: &mut VolumesControl,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    if dialog.is_some() {
        return Task::none();
    }

    let target = if let Some(node) = control.selected_volume_node() {
        if !node.volume.has_filesystem {
            return Task::none();
        }

        FilesystemTarget::Node(node.clone())
    } else {
        let Some(segment) = control.segments.get(control.selected_segment) else {
            return Task::none();
        };
        let Some(volume) = segment.volume.clone() else {
            return Task::none();
        };

        if !volume.has_filesystem {
            return Task::none();
        }

        FilesystemTarget::Volume(volume)
    };

    *dialog = Some(ShowDialog::EditFilesystemLabel(EditFilesystemLabelDialog {
        target,
        label: String::new(),
        running: false,
    }));

    Task::none()
}

pub(super) fn edit_filesystem_label_message(
    _control: &mut VolumesControl,
    msg: EditFilesystemLabelMessage,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    let Some(ShowDialog::EditFilesystemLabel(state)) = dialog.as_mut() else {
        return Task::none();
    };

    match msg {
        EditFilesystemLabelMessage::LabelUpdate(label) => state.label = label,
        EditFilesystemLabelMessage::Cancel => {
            return Task::done(Message::CloseDialog.into());
        }
        EditFilesystemLabelMessage::Confirm => {
            if state.running {
                return Task::none();
            }

            state.running = true;
            let target = state.target.clone();
            let label = state.label.clone();

            return Task::perform(
                async move {
                    let fs_client = FilesystemsClient::new().await.map_err(|e| {
                        anyhow::anyhow!("Failed to create filesystems client: {}", e)
                    })?;
                    let device = match &target {
                        FilesystemTarget::Volume(v) => v
                            .device_path
                            .as_ref()
                            .ok_or_else(|| anyhow::anyhow!("Volume has no device path"))?,
                        FilesystemTarget::Node(n) => n
                            .device_path
                            .as_ref()
                            .ok_or_else(|| anyhow::anyhow!("Volume has no device path"))?,
                    };
                    fs_client
                        .set_label(device, &label)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to set label: {}", e))?;
                    load_all_drives().await.map_err(|e| e.into())
                },
                |result: Result<Vec<UiDrive>, anyhow::Error>| match result {
                    Ok(drives) => Message::UpdateNav(drives, None).into(),
                    Err(e) => {
                        let ctx = UiErrorContext::new("edit_filesystem");
                        log_error_and_show_dialog(fl!("edit-filesystem").to_string(), e, ctx).into()
                    }
                },
            );
        }
    }

    Task::none()
}

pub(super) fn open_check_filesystem(
    control: &mut VolumesControl,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    if dialog.is_some() {
        return Task::none();
    }

    let target = if let Some(node) = control.selected_volume_node() {
        if !node.volume.has_filesystem {
            return Task::none();
        }
        FilesystemTarget::Node(node.clone())
    } else {
        let Some(segment) = control.segments.get(control.selected_segment) else {
            return Task::none();
        };
        let Some(volume) = segment.volume.clone() else {
            return Task::none();
        };
        if !volume.has_filesystem {
            return Task::none();
        }
        FilesystemTarget::Volume(volume)
    };

    *dialog = Some(ShowDialog::ConfirmAction(ConfirmActionDialog {
        title: fl!("check-filesystem").to_string(),
        body: fl!("check-filesystem-warning").to_string(),
        target,
        ok_message: VolumesControlMessage::CheckFilesystemConfirm.into(),
        running: false,
    }));

    Task::none()
}

pub(super) fn check_filesystem_confirm(
    _control: &mut VolumesControl,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    let Some(ShowDialog::ConfirmAction(state)) = dialog.as_mut() else {
        return Task::none();
    };

    if state.running {
        return Task::none();
    }
    state.running = true;

    let target = state.target.clone();
    Task::perform(
        async move {
            let fs_client = FilesystemsClient::new()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create filesystems client: {}", e))?;
            let device = match &target {
                FilesystemTarget::Volume(v) => v
                    .device_path
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Volume has no device path"))?,
                FilesystemTarget::Node(n) => n
                    .device_path
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Volume has no device path"))?,
            };
            let _output = fs_client
                .check(device, false)
                .await
                .map_err(|e| anyhow::anyhow!("Filesystem check failed: {}", e))?;
            load_all_drives().await.map_err(|e| e.into())
        },
        |result: Result<Vec<UiDrive>, anyhow::Error>| match result {
            Ok(drives) => Message::UpdateNav(drives, None).into(),
            Err(e) => {
                let ctx = UiErrorContext::new("check_filesystem");
                log_error_and_show_dialog(fl!("check-filesystem").to_string(), e, ctx).into()
            }
        },
    )
}

pub(super) fn open_repair_filesystem(
    control: &mut VolumesControl,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    if dialog.is_some() {
        return Task::none();
    }

    let target = if let Some(node) = control.selected_volume_node() {
        if !node.volume.has_filesystem {
            return Task::none();
        }
        FilesystemTarget::Node(node.clone())
    } else {
        let Some(segment) = control.segments.get(control.selected_segment) else {
            return Task::none();
        };
        let Some(volume) = segment.volume.clone() else {
            return Task::none();
        };

        if !volume.has_filesystem {
            return Task::none();
        }

        FilesystemTarget::Volume(volume)
    };

    *dialog = Some(ShowDialog::ConfirmAction(ConfirmActionDialog {
        title: fl!("repair-filesystem").to_string(),
        body: fl!("repair-filesystem-warning").to_string(),
        target,
        ok_message: VolumesControlMessage::RepairFilesystemConfirm.into(),
        running: false,
    }));

    Task::none()
}

pub(super) fn repair_filesystem_confirm(
    _control: &mut VolumesControl,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    let Some(ShowDialog::ConfirmAction(state)) = dialog.as_mut() else {
        return Task::none();
    };

    if state.running {
        return Task::none();
    }
    state.running = true;

    let target = state.target.clone();
    Task::perform(
        async move {
            let fs_client = FilesystemsClient::new()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create filesystems client: {}", e))?;
            let device = match &target {
                FilesystemTarget::Volume(v) => v
                    .device_path
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Volume has no device path"))?,
                FilesystemTarget::Node(n) => n
                    .device_path
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Volume has no device path"))?,
            };
            let _output = fs_client
                .check(device, true)
                .await
                .map_err(|e| anyhow::anyhow!("Filesystem repair failed: {}", e))?;
            load_all_drives().await.map_err(|e| e.into())
        },
        |result: Result<Vec<UiDrive>, anyhow::Error>| match result {
            Ok(drives) => Message::UpdateNav(drives, None).into(),
            Err(e) => {
                let ctx = UiErrorContext::new("repair_filesystem");
                log_error_and_show_dialog(fl!("repair-filesystem").to_string(), e, ctx).into()
            }
        },
    )
}

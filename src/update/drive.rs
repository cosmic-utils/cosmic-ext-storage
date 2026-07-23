use crate::errors::ui::{UiErrorContext, log_error_and_show_dialog};
use crate::fl;
use crate::message::dialogs::FormatDiskMessage;
use crate::models::{UiDrive, load_all_drives};
use crate::operations::{DisksClient, PartitionsClient};
use crate::state::dialogs::{FormatDiskDialog, ShowDialog, SmartDataDialog};
use cosmic::app::Task;

use crate::message::app::Message;
use crate::state::app::AppModel;

pub(super) fn format_disk(app: &mut AppModel, msg: FormatDiskMessage) -> Task<Message> {
    let Some(ShowDialog::FormatDisk(state)) = app.dialog.as_mut() else {
        return Task::none();
    };

    match msg {
        FormatDiskMessage::EraseUpdate(v) => state.erase_index = v,
        FormatDiskMessage::PartitioningUpdate(v) => state.partitioning_index = v,
        FormatDiskMessage::Cancel => {
            app.dialog = None;
        }
        FormatDiskMessage::Confirm => {
            if state.running {
                return Task::none();
            }

            state.running = true;

            let drive = state.drive.clone();
            let block_path = drive.device().to_string();
            let drive_path = drive.device().to_string();
            let block_path_for_closure = block_path.clone();
            let drive_path_for_closure = drive_path.clone();
            let _erase = state.erase_index == 1;
            let format_type = match state.partitioning_index {
                0 => "dos",
                1 => "gpt",
                _ => "empty",
            };

            return Task::perform(
                async move {
                    let partitions_client = PartitionsClient::new().await.map_err(|e| {
                        anyhow::anyhow!("Failed to create partitions client: {}", e)
                    })?;
                    partitions_client
                        .create_partition_table(&block_path, format_type)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to format disk: {}", e))?;
                    load_all_drives().await.map_err(|e| e.into())
                },
                move |res: Result<Vec<UiDrive>, anyhow::Error>| match res {
                    Ok(drives) => {
                        Message::UpdateNav(drives, Some(block_path_for_closure.clone())).into()
                    }
                    Err(e) => {
                        let ctx = UiErrorContext {
                            operation: "format_disk",
                            device_path: Some(drive_path_for_closure.as_str()),
                            device: Some(block_path_for_closure.as_str()),
                            drive_path: Some(drive_path_for_closure.as_str()),
                        };
                        log_error_and_show_dialog(fl!("format-disk-failed"), e, ctx).into()
                    }
                },
            );
        }
    };

    Task::none()
}

pub(super) fn eject(app: &mut AppModel) -> Task<Message> {
    let Some(drive) = app.nav.active_data::<UiDrive>().cloned() else {
        return Task::none();
    };

    eject_drive(drive)
}

pub(super) fn eject_drive(drive: UiDrive) -> Task<Message> {
    let drive_path = drive.device().to_string();
    let block_path = drive.device().to_string();
    let block_path_for_closure = block_path.clone();
    let drive_path_for_closure = drive_path.clone();

    Task::perform(
        async move {
            let disks_client = DisksClient::new()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create disks client: {}", e));
            let res = match disks_client {
                Ok(client) => client
                    .remove(&block_path)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to remove: {}", e)),
                Err(e) => Err(e),
            };
            let drives = load_all_drives().await.ok();
            (res, drives)
        },
        move |(res, drives)| match res {
            Ok(()) => match drives {
                Some(drives) => Message::UpdateNav(drives, None).into(),
                None => Message::None.into(),
            },
            Err(e) => {
                let ctx = UiErrorContext {
                    operation: "eject_or_remove",
                    device_path: Some(drive_path_for_closure.as_str()),
                    device: Some(block_path_for_closure.as_str()),
                    drive_path: Some(drive_path_for_closure.as_str()),
                };
                log_error_and_show_dialog(fl!("eject-failed"), e, ctx).into()
            }
        },
    )
}

pub(super) fn power_off(app: &mut AppModel) -> Task<Message> {
    let Some(drive) = app.nav.active_data::<UiDrive>().cloned() else {
        return Task::none();
    };

    power_off_drive(drive)
}

pub(super) fn power_off_drive(drive: UiDrive) -> Task<Message> {
    let drive_path = drive.device().to_string();
    let block_path = drive.device().to_string();
    let block_path_for_closure = block_path.clone();
    let drive_path_for_closure = drive_path.clone();

    Task::perform(
        async move {
            let disks_client = DisksClient::new()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create disks client: {}", e));
            let res = match disks_client {
                Ok(client) => client
                    .power_off(&block_path)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to power off: {}", e)),
                Err(e) => Err(e),
            };
            let drives = load_all_drives().await.ok();
            (res, drives)
        },
        move |(res, drives)| match res {
            Ok(()) => match drives {
                Some(drives) => Message::UpdateNav(drives, None).into(),
                None => Message::None.into(),
            },
            Err(e) => {
                let ctx = UiErrorContext {
                    operation: "power_off",
                    device_path: Some(drive_path_for_closure.as_str()),
                    device: Some(block_path_for_closure.as_str()),
                    drive_path: Some(drive_path_for_closure.as_str()),
                };
                log_error_and_show_dialog(fl!("power-off-failed"), e, ctx).into()
            }
        },
    )
}

pub(super) fn format(app: &mut AppModel) {
    let Some(drive) = app.nav.active_data::<UiDrive>().cloned() else {
        return;
    };

    format_for(app, drive)
}

pub(super) fn format_for(app: &mut AppModel, drive: UiDrive) {
    let partitioning_index = match drive.partition_table_type.as_deref() {
        Some("dos") => 0,
        Some("gpt") => 1,
        _ => 2,
    };

    app.dialog = Some(ShowDialog::FormatDisk(FormatDiskDialog {
        drive,
        erase_index: 0,
        partitioning_index,
        running: false,
    }));
}

pub(super) fn smart_data(app: &mut AppModel) -> Task<Message> {
    let Some(drive) = app.nav.active_data::<UiDrive>().cloned() else {
        return Task::none();
    };

    smart_data_for(app, drive)
}

pub(super) fn smart_data_for(app: &mut AppModel, drive: UiDrive) -> Task<Message> {
    app.dialog = Some(ShowDialog::SmartData(SmartDataDialog {
        drive: drive.clone(),
        running: true,
        info: None,
        error: None,
    }));

    Task::perform(
        async move {
            let disks_client = DisksClient::new()
                .await
                .map_err(|e| format!("Failed to create disks client: {}", e))?;
            let status = disks_client
                .get_smart_status(drive.device())
                .await
                .map_err(|e| format!("Failed to get SMART status: {}", e))?;
            let attributes = disks_client
                .get_smart_attributes(drive.device())
                .await
                .map_err(|e| format!("Failed to get SMART attributes: {}", e))?;
            Ok((status, attributes))
        },
        |res| Message::SmartDialog(crate::message::dialogs::SmartDialogMessage::Loaded(res)).into(),
    )
}

pub(super) fn standby_now(app: &mut AppModel) -> Task<Message> {
    let Some(drive) = app.nav.active_data::<UiDrive>().cloned() else {
        return Task::none();
    };

    standby_now_drive(drive)
}

pub(super) fn standby_now_drive(drive: UiDrive) -> Task<Message> {
    let drive_path = drive.device().to_string();
    let device = drive.device().to_string();
    let device_for_closure = device.clone();
    let drive_path_for_closure = drive_path.clone();

    Task::perform(
        async move {
            DisksClient::new()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create disks client: {}", e))?
                .standby_now(&device)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to standby: {}", e))
        },
        move |res| match res {
            Ok(()) => Message::Dialog(Box::new(ShowDialog::Info {
                title: fl!("app-title"),
                body: "Standby requested.".to_string(),
            }))
            .into(),
            Err(e) => {
                let ctx = UiErrorContext {
                    operation: "standby_now",
                    device_path: Some(drive_path_for_closure.as_str()),
                    device: Some(device_for_closure.as_str()),
                    drive_path: Some(drive_path_for_closure.as_str()),
                };
                log_error_and_show_dialog(fl!("standby-failed"), e, ctx).into()
            }
        },
    )
}

pub(super) fn wakeup(app: &mut AppModel) -> Task<Message> {
    let Some(drive) = app.nav.active_data::<UiDrive>().cloned() else {
        return Task::none();
    };

    wakeup_drive(drive)
}

pub(super) fn wakeup_drive(drive: UiDrive) -> Task<Message> {
    let drive_path = drive.device().to_string();
    let device = drive.device().to_string();
    let device_for_closure = device.clone();
    let drive_path_for_closure = drive_path.clone();

    Task::perform(
        async move {
            DisksClient::new()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create disks client: {}", e))?
                .wakeup(&device)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to wakeup: {}", e))
        },
        move |res| match res {
            Ok(()) => Message::Dialog(Box::new(ShowDialog::Info {
                title: fl!("app-title"),
                body: "Wake-up requested.".to_string(),
            }))
            .into(),
            Err(e) => {
                let ctx = UiErrorContext {
                    operation: "wakeup",
                    device_path: Some(drive_path_for_closure.as_str()),
                    device: Some(device_for_closure.as_str()),
                    drive_path: Some(drive_path_for_closure.as_str()),
                };
                log_error_and_show_dialog(fl!("wake-up-failed"), e, ctx).into()
            }
        },
    )
}

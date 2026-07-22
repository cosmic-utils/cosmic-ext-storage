mod dialogs;
mod ops;

use crate::fl;
use crate::models::UiDrive;
use crate::state::dialogs::{ImageOperationDialog, ImageOperationKind, ShowDialog};
use crate::state::volumes::VolumesControl;
use cosmic::app::Task;

use crate::message::app::Message;
use crate::state::app::AppModel;

pub(super) fn new_disk_image(app: &mut AppModel) {
    dialogs::new_disk_image(app);
}

pub(super) fn attach_disk(app: &mut AppModel) {
    dialogs::attach_disk(app);
}

pub(super) fn new_disk_image_dialog(
    app: &mut AppModel,
    msg: crate::message::dialogs::NewDiskImageDialogMessage,
) -> Task<Message> {
    dialogs::new_disk_image_dialog(app, msg)
}

pub(super) fn attach_disk_image_dialog(
    app: &mut AppModel,
    msg: crate::message::dialogs::AttachDiskImageDialogMessage,
) -> Task<Message> {
    dialogs::attach_disk_image_dialog(app, msg)
}

pub(super) fn image_operation_dialog(
    app: &mut AppModel,
    msg: crate::message::dialogs::ImageOperationDialogMessage,
) -> Task<Message> {
    dialogs::image_operation_dialog(app, msg)
}

pub(super) fn create_disk_from(app: &mut AppModel) -> Task<Message> {
    let Some(drive) = app.nav.active_data::<UiDrive>().cloned() else {
        return Task::none();
    };

    app.dialog = Some(ShowDialog::ImageOperation(
        ImageOperationDialog {
            kind: ImageOperationKind::CreateFromDrive,
            drive,
            partition: None,
            image_path: String::new(),
            running: false,
            operation_id: None,
            progress: None,
            error: None,
        }
        .into(),
    ));

    Task::none()
}

pub(super) fn restore_image_to(app: &mut AppModel) -> Task<Message> {
    let Some(drive) = app.nav.active_data::<UiDrive>().cloned() else {
        return Task::none();
    };

    app.dialog = Some(ShowDialog::ImageOperation(
        ImageOperationDialog {
            kind: ImageOperationKind::RestoreToDrive,
            drive,
            partition: None,
            image_path: String::new(),
            running: false,
            operation_id: None,
            progress: None,
            error: None,
        }
        .into(),
    ));

    Task::none()
}

pub(super) fn create_disk_from_partition(app: &mut AppModel) -> Task<Message> {
    let Some(drive) = app.nav.active_data::<UiDrive>().cloned() else {
        return Task::none();
    };

    let Some(volumes_control) = app.nav.active_data::<VolumesControl>() else {
        app.dialog = Some(ShowDialog::Info {
            title: fl!("app-title"),
            body: fl!("no-disk-selected"),
        });
        return Task::none();
    };

    let partition = volumes_control
        .segments
        .get(volumes_control.selected_segment)
        .and_then(|s| s.volume.clone());

    let Some(partition) = partition else {
        app.dialog = Some(ShowDialog::Info {
            title: fl!("app-title"),
            body: "Select a partition to create an image from.".to_string(),
        });
        return Task::none();
    };

    app.dialog = Some(ShowDialog::ImageOperation(
        ImageOperationDialog {
            kind: ImageOperationKind::CreateFromPartition,
            drive,
            partition: Some(partition),
            image_path: String::new(),
            running: false,
            operation_id: None,
            progress: None,
            error: None,
        }
        .into(),
    ));

    Task::none()
}

pub(super) fn restore_image_to_partition(app: &mut AppModel) -> Task<Message> {
    let Some(drive) = app.nav.active_data::<UiDrive>().cloned() else {
        return Task::none();
    };

    let Some(volumes_control) = app.nav.active_data::<VolumesControl>() else {
        app.dialog = Some(ShowDialog::Info {
            title: fl!("app-title"),
            body: fl!("no-disk-selected"),
        });
        return Task::none();
    };

    let partition = volumes_control
        .segments
        .get(volumes_control.selected_segment)
        .and_then(|s| s.volume.clone());

    let Some(partition) = partition else {
        app.dialog = Some(ShowDialog::Info {
            title: fl!("app-title"),
            body: "Select a partition to restore an image to.".to_string(),
        });
        return Task::none();
    };

    app.dialog = Some(ShowDialog::ImageOperation(
        ImageOperationDialog {
            kind: ImageOperationKind::RestoreToPartition,
            drive,
            partition: Some(partition),
            image_path: String::new(),
            running: false,
            operation_id: None,
            progress: None,
            error: None,
        }
        .into(),
    ));

    Task::none()
}

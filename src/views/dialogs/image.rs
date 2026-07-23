use crate::app::Message;
use crate::controls::fields::labelled_spinner;
use crate::controls::wizard::{wizard_action_row, wizard_shell};
use crate::fl;
use crate::message::app::ImagePathPickerKind;
use crate::message::dialogs::{
    AttachDiskImageDialogMessage, ImageOperationDialogMessage, NewDiskImageDialogMessage,
};
use crate::state::dialogs::{
    AttachDiskImageDialog, ImageOperationDialog, ImageOperationKind, NewDiskImageDialog,
};
use cosmic::iced::widget as iced_widget;
use cosmic::{
    Element,
    iced::{Alignment, Length},
    widget::button,
    widget::text::caption,
};
use storage_types::bytes_to_pretty;

pub fn new_disk_image<'a>(state: NewDiskImageDialog) -> Element<'a, Message> {
    let size_pretty = bytes_to_pretty(&state.size_bytes, false);
    let step = storage_types::get_step(&state.size_bytes);

    let path_label = if state.path.trim().is_empty() {
        fl!("no-file-selected")
    } else {
        state.path.clone()
    };

    let path_row = iced_widget::row![
        caption(path_label).width(Length::Fill),
        button::standard(fl!("choose-path")).on_press(Message::OpenImagePathPicker(
            ImagePathPickerKind::NewDiskImage
        ))
    ]
    .align_y(Alignment::Center)
    .spacing(12);

    let mut content = iced_widget::column![
        caption(fl!("image-destination-path")),
        path_row,
        labelled_spinner(
            fl!("image-size"),
            size_pretty,
            state.size_bytes as f64,
            step,
            (1024 * 1024) as f64,
            (1024_u64.pow(5)) as f64,
            |v| NewDiskImageDialogMessage::SizeUpdate(v as u64).into(),
        ),
    ]
    .spacing(12);

    if let Some(err) = state.error.as_ref() {
        content = content.push(caption(err.clone()));
    }

    if state.running {
        content = content.push(caption(fl!("working")));
    }

    let mut create_button = button::destructive(fl!("create-image"));
    if !state.running {
        create_button = create_button.on_press(NewDiskImageDialogMessage::Create.into());
    }

    // while running, still allow closing (it will not cancel the file create, which is fast)
    let cancel_msg = NewDiskImageDialogMessage::Cancel;

    let footer = wizard_action_row(
        vec![],
        vec![
            button::standard(fl!("cancel"))
                .on_press(cancel_msg.into())
                .into(),
            create_button.into(),
        ],
    );

    wizard_shell(
        caption(fl!("new-disk-image")).into(),
        content.into(),
        footer,
    )
}

pub fn attach_disk_image<'a>(state: AttachDiskImageDialog) -> Element<'a, Message> {
    let path_label = if state.path.trim().is_empty() {
        fl!("no-file-selected")
    } else {
        state.path.clone()
    };

    let path_row = iced_widget::row![
        caption(path_label).width(Length::Fill),
        button::standard(fl!("choose-path")).on_press(Message::OpenImagePathPicker(
            ImagePathPickerKind::AttachDiskImage
        ))
    ]
    .align_y(Alignment::Center)
    .spacing(12);

    let mut content = iced_widget::column![caption(fl!("image-file-path")), path_row,].spacing(12);

    if let Some(err) = state.error.as_ref() {
        content = content.push(caption(err.clone()));
    }

    if state.running {
        content = content.push(caption(fl!("working")));
    }

    let mut attach_button = button::destructive(fl!("attach"));
    if !state.running {
        attach_button = attach_button.on_press(AttachDiskImageDialogMessage::Attach.into());
    }

    let footer = wizard_action_row(
        vec![],
        vec![
            button::standard(fl!("cancel"))
                .on_press(AttachDiskImageDialogMessage::Cancel.into())
                .into(),
            attach_button.into(),
        ],
    );

    wizard_shell(
        caption(fl!("attach-disk-image")).into(),
        content.into(),
        footer,
    )
}

pub fn image_operation<'a>(state: ImageOperationDialog) -> Element<'a, Message> {
    let title = match state.kind {
        ImageOperationKind::CreateFromDrive => fl!("create-disk-from-drive"),
        ImageOperationKind::RestoreToDrive => fl!("restore-image-to-drive"),
        ImageOperationKind::CreateFromPartition => fl!("create-disk-from-partition"),
        ImageOperationKind::RestoreToPartition => fl!("restore-image-to-partition"),
    };

    let path_label = match state.kind {
        ImageOperationKind::CreateFromDrive | ImageOperationKind::CreateFromPartition => {
            fl!("image-destination-path")
        }
        ImageOperationKind::RestoreToDrive | ImageOperationKind::RestoreToPartition => {
            fl!("image-source-path")
        }
    };

    let mut content = iced_widget::column![caption(format!(
        "{}: {}",
        fl!("device"),
        state.drive.name()
    ))]
    .spacing(12);

    if matches!(
        state.kind,
        ImageOperationKind::CreateFromPartition | ImageOperationKind::RestoreToPartition
    ) {
        match state.partition.as_ref() {
            Some(partition) => {
                let partition_name = if partition.name().trim().is_empty() {
                    fl!("untitled")
                } else {
                    partition.name()
                };
                let partition_path = partition
                    .device_path
                    .clone()
                    .unwrap_or_else(|| fl!("unknown"));

                content = content
                    .push(caption(format!("{}: {}", fl!("partition"), partition_name)))
                    .push(caption(format!("{}: {}", fl!("path"), partition_path)));
            }
            None => {
                content =
                    content.push(caption(format!("{}: {}", fl!("partition"), fl!("unknown"))));
            }
        }
    }

    if matches!(
        state.kind,
        ImageOperationKind::RestoreToDrive | ImageOperationKind::RestoreToPartition
    ) {
        content = content.push(caption(fl!("restore-warning")));
    }

    let path_text = if state.image_path.trim().is_empty() {
        fl!("no-file-selected")
    } else {
        state.image_path.clone()
    };

    let picker_kind = match state.kind {
        ImageOperationKind::CreateFromDrive | ImageOperationKind::CreateFromPartition => {
            ImagePathPickerKind::ImageOperationCreate
        }
        ImageOperationKind::RestoreToDrive | ImageOperationKind::RestoreToPartition => {
            ImagePathPickerKind::ImageOperationRestore
        }
    };

    let path_row = iced_widget::row![
        caption(path_text).width(Length::Fill),
        button::standard(fl!("choose-path")).on_press(Message::OpenImagePathPicker(picker_kind))
    ]
    .align_y(Alignment::Center)
    .spacing(12);

    content = content.push(caption(path_label)).push(path_row);

    if let Some(err) = state.error.as_ref() {
        content = content.push(caption(err.clone()));
    }

    if state.running {
        content = content.push(caption(fl!("working")));
        if let Some((bytes_completed, total_bytes, speed_bytes_per_sec)) = state.progress {
            let fraction = if total_bytes > 0 {
                (bytes_completed as f64 / total_bytes as f64).min(1.0) as f32
            } else {
                0.0_f32
            };
            content =
                content.push(iced_widget::progress_bar(0.0..=1.0, fraction).length(Length::Fill));
            if total_bytes > 0 {
                let done = storage_types::bytes_to_pretty(&bytes_completed, false);
                let total = storage_types::bytes_to_pretty(&total_bytes, false);
                let speed = storage_types::bytes_to_pretty(&speed_bytes_per_sec, false);
                content = content.push(caption(format!("{} / {} · {}/s", done, total, speed)));
            }
        }
    }

    let primary_label = match state.kind {
        ImageOperationKind::CreateFromDrive | ImageOperationKind::CreateFromPartition => {
            fl!("create-image")
        }
        ImageOperationKind::RestoreToDrive | ImageOperationKind::RestoreToPartition => {
            fl!("restore-image")
        }
    };

    let mut start_button = button::destructive(primary_label);
    if !state.running {
        start_button = start_button.on_press(ImageOperationDialogMessage::Start.into());
    }

    let cancel_msg = ImageOperationDialogMessage::CancelOperation;

    let footer = wizard_action_row(
        vec![],
        vec![
            button::standard(fl!("cancel"))
                .on_press(cancel_msg.into())
                .into(),
            start_button.into(),
        ],
    );

    wizard_shell(caption(title.clone()).into(), content.into(), footer)
}

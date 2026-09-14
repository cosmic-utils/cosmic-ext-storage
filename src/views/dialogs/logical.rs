use crate::{
    app::Message,
    message::dialogs::LogicalActionFormMessage,
    state::dialogs::{
        LogicalActionConfirmationDialog, LogicalActionForm, LogicalActionFormDialog,
        LogicalDevicePickerDialog,
    },
};
use cosmic::{
    Element,
    widget::{button, checkbox, dialog, text, text_input},
};

/// Renders a reviewable native logical mutation; the contained action is fully
/// typed before this dialog is opened.
pub fn confirmation(state: LogicalActionConfirmationDialog) -> Element<'static, Message> {
    super::common::confirmation(
        state.title,
        state.body,
        Message::LogicalActionConfirmed(state.confirmed),
        Some(Message::LogicalActionCancelled),
        state.running,
    )
}

/// Renders only semantic fields.  Fresh device/subvolume identity is already
/// present in the typed form and is not editable in the dialog.
pub fn form<'a>(state: LogicalActionFormDialog) -> Element<'a, Message> {
    let title = state.form.title();
    let submit_label = state.form.submit_label();
    let mut content = match state.form {
        LogicalActionForm::CreateLvmLogicalVolume {
            name, size_bytes, ..
        } => cosmic::iced::widget::column![
            text_input("Name", name)
                .label("Name")
                .on_input(|text| LogicalActionFormMessage::PrimaryTextUpdate(text).into()),
            text_input("Size in bytes", size_bytes)
                .label("Size in bytes")
                .on_input(|text| LogicalActionFormMessage::SizeUpdate(text).into()),
        ],
        LogicalActionForm::ResizeLvmLogicalVolume { size_bytes, .. }
        | LogicalActionForm::ResizeBtrfsFilesystem { size_bytes, .. } => {
            cosmic::iced::widget::column![
                text_input("Size in bytes", size_bytes)
                    .label("Size in bytes")
                    .on_input(|text| LogicalActionFormMessage::SizeUpdate(text).into())
            ]
        }
        LogicalActionForm::SetBtrfsLabel { label, .. } => cosmic::iced::widget::column![
            text_input("Label", label)
                .label("Label")
                .on_input(|text| LogicalActionFormMessage::PrimaryTextUpdate(text).into()),
        ],
        LogicalActionForm::CreateBtrfsSubvolume { name, .. } => cosmic::iced::widget::column![
            text_input("Relative subvolume name", name)
                .label("Relative subvolume name")
                .on_input(|text| LogicalActionFormMessage::PrimaryTextUpdate(text).into()),
        ],
        LogicalActionForm::CreateBtrfsSnapshot {
            source,
            destination,
            readonly,
            ..
        } => cosmic::iced::widget::column![
            text(format!("Source: {}", source.expected_relative_path)),
            text_input("Relative snapshot name", destination)
                .label("Relative snapshot name")
                .on_input(|text| LogicalActionFormMessage::PrimaryTextUpdate(text).into()),
            checkbox(readonly)
                .label("Read-only snapshot")
                .on_toggle(|value| LogicalActionFormMessage::ReadOnlyUpdate(value).into()),
        ],
    }
    .spacing(12);

    if let Some(error) = state.error {
        content = content.push(text(error).size(11));
    }

    dialog::dialog()
        .title(title)
        .control(content)
        .primary_action(
            button::suggested(submit_label).on_press(LogicalActionFormMessage::Submit.into()),
        )
        .secondary_action(
            button::standard("Cancel").on_press(LogicalActionFormMessage::Cancel.into()),
        )
        .into()
}

pub fn device_picker<'a>(state: LogicalDevicePickerDialog) -> Element<'a, Message> {
    let mut rows = Vec::with_capacity(state.candidates.len());
    for candidate in state.candidates {
        match candidate {
            storage_contracts::LogicalDeviceCandidate::Ready { device, display } => {
                rows.push(
                    button::standard(candidate_label(&display))
                        .on_press(Message::LogicalDevicePickerSelected(device))
                        .into(),
                );
            }
            storage_contracts::LogicalDeviceCandidate::Blocked { display, reason } => {
                rows.push(
                    text(format!(
                        "{} — {}",
                        candidate_label(&display),
                        candidate_reason(&reason)
                    ))
                    .size(11)
                    .into(),
                );
            }
        }
    }
    if rows.is_empty() {
        rows.push(text("No current device candidates are available.").into());
    }
    dialog::dialog()
        .title(state.picker.title())
        .control(cosmic::iced::widget::Column::with_children(rows).spacing(8))
        .secondary_action(
            button::standard("Cancel").on_press(Message::LogicalDevicePickerCancelled),
        )
        .into()
}

fn candidate_label(display: &storage_contracts::LogicalCandidateDisplay) -> String {
    let label = display
        .label
        .as_known()
        .cloned()
        .unwrap_or_else(|| "Unknown device".into());
    let path = display
        .path
        .as_known()
        .cloned()
        .unwrap_or_else(|| "Unknown path".into());
    let size = display
        .size
        .as_known()
        .map(|size| storage_types::bytes_to_pretty(size, false))
        .unwrap_or_else(|| "Unknown size".into());
    format!("{label} — {path} ({size})")
}

fn candidate_reason(reason: &storage_contracts::CandidateBlockReason) -> String {
    match reason {
        storage_contracts::CandidateBlockReason::NoStrongIdentity => {
            "A strong device identity is unavailable".into()
        }
        storage_contracts::CandidateBlockReason::AlreadyTargetMember => {
            "Already a member of this filesystem".into()
        }
        storage_contracts::CandidateBlockReason::StructuredDataSignature { signature } => {
            format!("Contains {signature}")
        }
        storage_contracts::CandidateBlockReason::SourceUnavailable { source, reason } => {
            format!("{source}: {reason}")
        }
        storage_contracts::CandidateBlockReason::Ineligible { reason } => reason.clone(),
    }
}

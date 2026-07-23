use crate::{app::Message, state::dialogs::LogicalActionConfirmationDialog};
use cosmic::Element;

/// Renders a reviewable native logical mutation; the contained action is fully
/// typed before this dialog is opened.
pub fn confirmation(state: LogicalActionConfirmationDialog) -> Element<'static, Message> {
    super::common::confirmation(
        state.title,
        state.body,
        Message::LogicalActionConfirmed(state.action),
        Some(Message::LogicalActionCancelled),
        state.running,
    )
}

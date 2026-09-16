use super::*;
use cosmic::iced::keyboard::Modifiers;

#[test]
fn usage_row_selection_message_defaults_to_single_click_selection() {
    let message = usage_row_selection_message("/tmp/a".to_string(), 3, Modifiers::empty());

    match message {
        Message::UsageSelectionSingle { path, index } => {
            assert_eq!(path, "/tmp/a");
            assert_eq!(index, 3);
        }
        other => panic!("unexpected message: {other:?}"),
    }
}

#[test]
fn usage_row_selection_message_uses_ctrl_for_toggle() {
    let message = usage_row_selection_message("/tmp/b".to_string(), 5, Modifiers::CTRL);

    match message {
        Message::UsageSelectionCtrl { path, index } => {
            assert_eq!(path, "/tmp/b");
            assert_eq!(index, 5);
        }
        other => panic!("unexpected message: {other:?}"),
    }
}

#[test]
fn usage_row_selection_message_uses_shift_for_range_selection() {
    let message = usage_row_selection_message("/tmp/c".to_string(), 7, Modifiers::SHIFT);

    match message {
        Message::UsageSelectionShift { index } => {
            assert_eq!(index, 7);
        }
        other => panic!("unexpected message: {other:?}"),
    }
}

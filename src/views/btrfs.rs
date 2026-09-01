//! Compatibility hand-off from the physical volume page to Logical Storage.

use crate::app::Message;
use crate::state::btrfs::BtrfsState;
use cosmic::{Element, widget};
use storage_types::VolumeInfo;

/// Physical-volume Btrfs management intentionally owns no mutation controls.
/// The logical page captures a fresh native identity before offering forms or
/// confirmation, while this older surface has only a display path.
pub fn btrfs_management_section<'a>(
    volume: &'a VolumeInfo,
    _state: &'a BtrfsState,
) -> Element<'a, Message> {
    let mut content = widget::Column::with_children(vec![
        widget::text::title3("Btrfs management").into(),
        widget::text::body(
            "Manage subvolumes and devices from Logical Storage, where the current filesystem can be reviewed safely.",
        )
        .into(),
    ])
    .spacing(12);

    if let Some(device_path) = volume.device_path.clone() {
        content = content.push(widget::button::text("Open Logical Storage").on_press(
            Message::LogicalViewRequested {
                device_path: Some(device_path),
            },
        ));
    } else {
        content = content.push(widget::text::caption(
            "The selected Btrfs device no longer has a readable path.",
        ));
    }

    content.into()
}

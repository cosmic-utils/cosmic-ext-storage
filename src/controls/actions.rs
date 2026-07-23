// SPDX-License-Identifier: GPL-3.0-only

use cosmic::iced::Alignment;
use cosmic::widget::{self, icon};
use cosmic::{Apply, Element};

pub(crate) fn icon_tooltip_action<Message: Clone + 'static>(
    icon_name: &'static str,
    label: &'static str,
    message: Option<Message>,
    enabled: bool,
) -> Element<'static, Message> {
    let mut button = widget::button::icon(icon::from_name(icon_name).size(16));
    if enabled && let Some(message) = message {
        button = button.on_press(message);
    }

    widget::tooltip(
        button,
        widget::text(label),
        widget::tooltip::Position::Bottom,
    )
    .into()
}

pub(crate) fn trailing_actions_row<Message: 'static>(
    actions: Vec<Element<'static, Message>>,
) -> Element<'static, Message> {
    widget::Row::from_vec(actions)
        .spacing(4)
        .align_y(Alignment::Center)
        .apply(widget::container)
        .padding([0, 10, 0, 0])
        .into()
}

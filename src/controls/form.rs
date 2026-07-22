// SPDX-License-Identifier: GPL-3.0-only

use cosmic::iced::Length;
use cosmic::{Element, widget};

pub(crate) fn bounded_form<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    max_width: u16,
) -> Element<'a, Message> {
    widget::container(content)
        .width(Length::Fill)
        .max_width(max_width)
        .into()
}

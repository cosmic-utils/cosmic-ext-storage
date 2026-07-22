// SPDX-License-Identifier: GPL-3.0-only

use std::borrow::Cow;
use storage_types::pretty_to_bytes;

use cosmic::iced::widget as iced_widget;
use cosmic::{
    Element,
    cosmic_theme::Spacing,
    iced::{Alignment, Length, alignment},
    widget::{self, button, container},
};
use iced_widget::row;

pub fn input_spinner<'a, Message: 'static + Clone>(
    value_string: impl Into<Cow<'a, str>>,
    value: f64,
    step: f64,
    min: f64,
    max: f64,
    on_edit: impl Fn(f64) -> Message + 'static + Clone,
) -> Element<'a, Message> {
    let text_edit = on_edit.clone();
    container(row![
        button::text("-").on_press((on_edit.clone())(value - step)),
        widget::text_input("", value_string.into())
            .width(Length::Fill)
            .on_input(move |v| {
                match pretty_to_bytes(&v) {
                    Ok(v) => (text_edit)((v as f64).clamp(min, max)),
                    Err(_) => (text_edit)(value),
                }
            }),
        button::text("+").on_press((on_edit)(value + step)),
    ])
    .into()
}

pub fn labelled_spinner<'a, Message: 'static + Clone>(
    label: impl Into<Cow<'a, str>>,
    value_string: impl Into<Cow<'a, str>>,
    value: f64,
    step: f64,
    min: f64,
    max: f64,
    on_press: impl Fn(f64) -> Message + 'static + Clone,
) -> Element<'a, Message> {
    iced_widget::row![
        widget::text(label.into())
            .align_x(Alignment::End)
            .width(Length::FillPortion(1)),
        container(input_spinner(value_string, value, step, min, max, on_press))
            .width(Length::FillPortion(3)),
    ]
    .align_y(alignment::Vertical::Center)
    .spacing(Spacing::default().space_s)
    .into()
}

#[allow(dead_code)]
pub fn labelled_info<'a, Message: 'static + Clone>(
    label: impl Into<String>,
    info: impl Into<String>,
) -> Element<'a, Message> {
    iced_widget::row![
        widget::text(label.into())
            .align_x(Alignment::End)
            .width(Length::FillPortion(1)),
        widget::text(info.into()).width(Length::FillPortion(3)),
    ]
    .spacing(Spacing::default().space_s)
    .into()
}

#[allow(dead_code)]
pub fn link_info<'a, Message: 'static + Clone>(
    label: impl Into<String>,
    info: impl Into<String>,
    message: Message,
) -> Element<'a, Message> {
    iced_widget::row![
        widget::text(label.into())
            .align_x(Alignment::End)
            .width(Length::FillPortion(1)),
        container(
            cosmic::widget::button::link(info.into())
                .width(Length::Shrink)
                .padding(0)
                .on_press(message)
        )
        .width(Length::FillPortion(3)),
    ]
    .spacing(Spacing::default().space_s)
    .into()
}

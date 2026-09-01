// SPDX-License-Identifier: GPL-3.0-only

use cosmic::cosmic_theme::palette::WithAlpha;
use cosmic::iced::Alignment;
use cosmic::widget::{self, icon};
use cosmic::{Apply, Element};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IconActionTone {
    Neutral,
    Accent,
    Success,
    Destructive,
}

pub(crate) fn icon_tooltip_action<Message: Clone + 'static>(
    icon_name: &'static str,
    label: &'static str,
    message: Option<Message>,
    enabled: bool,
) -> Element<'static, Message> {
    icon_tooltip_action_toned(icon_name, label, message, enabled, IconActionTone::Neutral)
}

pub(crate) fn icon_tooltip_action_toned<Message: Clone + 'static>(
    icon_name: &'static str,
    label: &'static str,
    message: Option<Message>,
    enabled: bool,
    tone: IconActionTone,
) -> Element<'static, Message> {
    let mut button =
        widget::button::icon(icon::from_name(icon_name).size(16)).class(icon_action_class(tone));
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

fn icon_action_class(tone: IconActionTone) -> cosmic::theme::Button {
    cosmic::theme::Button::Custom {
        active: Box::new(move |_focused, theme| icon_action_style(tone, false, theme)),
        disabled: Box::new(move |theme| icon_action_style(tone, true, theme)),
        hovered: Box::new(move |_focused, theme| icon_action_style(tone, false, theme)),
        pressed: Box::new(move |_focused, theme| icon_action_style(tone, false, theme)),
    }
}

fn icon_action_style(
    tone: IconActionTone,
    disabled: bool,
    theme: &cosmic::theme::Theme,
) -> cosmic::widget::button::Style {
    let cosmic = theme.cosmic();
    let mut color = match tone {
        IconActionTone::Neutral => cosmic.background(false).component.on,
        IconActionTone::Accent => cosmic.accent_color(),
        IconActionTone::Success => cosmic.success_color(),
        IconActionTone::Destructive => cosmic.destructive_color(),
    };
    if disabled {
        color = color.with_alpha(0.35);
    }
    cosmic::widget::button::Style {
        shadow_offset: Default::default(),
        background: None,
        overlay: None,
        border_radius: cosmic.corner_radii.radius_xs.into(),
        border_width: 0.0,
        border_color: cosmic
            .background(false)
            .component
            .base
            .with_alpha(0.0)
            .into(),
        outline_width: 0.0,
        outline_color: cosmic
            .background(false)
            .component
            .base
            .with_alpha(0.0)
            .into(),
        icon_color: Some(color.into()),
        text_color: Some(color.into()),
    }
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

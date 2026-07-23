// SPDX-License-Identifier: GPL-3.0-only

use cosmic::{
    Theme,
    iced::{self, Background, Border, Color, Shadow},
    widget,
};

#[allow(dead_code)]
fn alert<'a, Message: 'static + Clone>(
    message: impl Into<String>,
    on_close: Message,
    style: fn(&Theme) -> widget::container::Style,
) -> widget::Container<'a, Message, Theme> {
    widget::warning(message.into())
        .on_close(on_close)
        .into_widget()
        .style(style)
}

#[allow(dead_code)]
pub fn warning<'a, Message: 'static + Clone>(
    message: impl Into<String>,
    on_close: Message,
) -> widget::Container<'a, Message, Theme> {
    alert(message, on_close, warning_style)
}

#[allow(dead_code)]
pub fn error<'a, Message: 'static + Clone>(
    message: impl Into<String>,
    on_close: Message,
) -> widget::Container<'a, Message, Theme> {
    alert(message, on_close, error_style)
}

#[allow(dead_code)]
pub fn success<'a, Message: 'static + Clone>(
    message: impl Into<String>,
    on_close: Message,
) -> widget::Container<'a, Message, Theme> {
    alert(message, on_close, success_style)
}

#[allow(dead_code)]
pub fn info<'a, Message: 'static + Clone>(
    message: impl Into<String>,
    on_close: Message,
) -> widget::Container<'a, Message, Theme> {
    alert(message, on_close, info_style)
}

#[allow(dead_code)]
pub fn warning_style(theme: &Theme) -> widget::container::Style {
    let cosmic = theme.cosmic();
    widget::container::Style {
        icon_color: Some(theme.cosmic().warning.on.into()),
        text_color: Some(theme.cosmic().warning.on.into()),
        background: Some(Background::Color(theme.cosmic().warning_color().into())),
        border: Border {
            color: Color::TRANSPARENT,
            width: 1.0,
            radius: cosmic.corner_radii.radius_0.into(),
        },
        shadow: Shadow {
            color: Color::TRANSPARENT,
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 0.0,
        },
        snap: false,
    }
}

#[allow(dead_code)]
pub fn error_style(theme: &Theme) -> widget::container::Style {
    let cosmic = theme.cosmic();
    widget::container::Style {
        icon_color: Some(theme.cosmic().destructive.on.into()),
        text_color: Some(theme.cosmic().destructive.on.into()),
        background: Some(Background::Color(theme.cosmic().destructive_color().into())),
        border: Border {
            color: Color::TRANSPARENT,
            width: 1.0,
            radius: cosmic.corner_radii.radius_0.into(),
        },
        shadow: Shadow {
            color: Color::TRANSPARENT,
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 0.0,
        },
        snap: false,
    }
}

#[allow(dead_code)]
pub fn success_style(theme: &Theme) -> widget::container::Style {
    let cosmic = theme.cosmic();
    widget::container::Style {
        icon_color: Some(theme.cosmic().success.on.into()),
        text_color: Some(theme.cosmic().success.on.into()),
        background: Some(Background::Color(theme.cosmic().success_color().into())),
        border: Border {
            color: Color::TRANSPARENT,
            width: 1.0,
            radius: cosmic.corner_radii.radius_0.into(),
        },
        shadow: Shadow {
            color: Color::TRANSPARENT,
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 0.0,
        },
        snap: false,
    }
}

#[allow(dead_code)]
pub fn info_style(theme: &Theme) -> widget::container::Style {
    let cosmic = theme.cosmic();
    widget::container::Style {
        icon_color: Some(theme.cosmic().accent.on.into()),
        text_color: Some(theme.cosmic().accent.on.into()),
        background: Some(Background::Color(theme.cosmic().accent_color().into())),
        border: Border {
            color: Color::TRANSPARENT,
            width: 1.0,
            radius: cosmic.corner_radii.radius_0.into(),
        },
        shadow: Shadow {
            color: Color::TRANSPARENT,
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 0.0,
        },
        snap: false,
    }
}

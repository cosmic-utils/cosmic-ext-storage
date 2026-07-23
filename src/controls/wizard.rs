// SPDX-License-Identifier: GPL-3.0-only

use cosmic::Element;
use cosmic::cosmic_theme::palette::WithAlpha;
use cosmic::iced::Alignment;
use cosmic::iced::Length;
use cosmic::iced::widget as iced_widget;
use cosmic::widget::{self, button};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WizardBreadcrumbStatus {
    Current,
    Completed,
    Upcoming,
}

pub(crate) struct WizardBreadcrumbStep<Message> {
    pub(crate) label: String,
    pub(crate) status: WizardBreadcrumbStatus,
    pub(crate) on_press: Option<Message>,
}

pub(crate) fn wizard_step_is_clickable(target_index: usize, current_index: usize) -> bool {
    target_index < current_index
}

pub(crate) fn wizard_shell<'a, Message: Clone + 'static>(
    header: Element<'a, Message>,
    content: Element<'a, Message>,
    footer: Element<'a, Message>,
) -> Element<'a, Message> {
    let content = widget::scrollable(
        widget::container(content)
            .padding([8, 0])
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill);

    let layout = iced_widget::column![header, content, footer]
        .spacing(12)
        .width(Length::Fill)
        .height(Length::Fill);

    widget::container(layout)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub(crate) fn wizard_action_row<'a, Message: Clone + 'static>(
    left_actions: Vec<Element<'a, Message>>,
    right_actions: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut row = iced_widget::row![]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    for action in left_actions {
        row = row.push(action);
    }

    row = row.push(widget::Space::new().width(Length::Fill));

    for action in right_actions {
        row = row.push(action);
    }

    row.into()
}

pub(crate) fn wizard_step_nav<'a, Message: Clone + 'static>(
    cancel_message: Message,
    back_message: Option<Message>,
    primary_label: String,
    primary_message: Option<Message>,
) -> Element<'a, Message> {
    let mut right_actions: Vec<Element<'a, Message>> = Vec::new();

    if let Some(back_message) = back_message {
        right_actions.push(button::standard("Back").on_press(back_message).into());
    }

    let mut primary_button = button::suggested(primary_label);
    if let Some(primary_message) = primary_message {
        primary_button = primary_button.on_press(primary_message);
    }
    right_actions.push(primary_button.into());

    wizard_action_row(
        vec![button::standard("Cancel").on_press(cancel_message).into()],
        right_actions,
    )
}

pub(crate) fn wizard_breadcrumb<'a, Message: Clone + 'static>(
    steps: Vec<WizardBreadcrumbStep<Message>>,
) -> Element<'a, Message> {
    let mut children: Vec<Element<'a, Message>> = Vec::new();

    for (index, step) in steps.into_iter().enumerate() {
        let is_current = step.status == WizardBreadcrumbStatus::Current;
        let is_done = step.status == WizardBreadcrumbStatus::Completed;

        let text = if is_current {
            widget::text::body(step.label).font(cosmic::font::semibold())
        } else {
            widget::text::body(step.label)
        };

        let styled_text: Element<'a, Message> = widget::container(text)
            .style(move |theme| {
                let color = if is_current {
                    theme.cosmic().accent_color()
                } else if is_done {
                    theme.cosmic().background(false).component.on
                } else {
                    theme
                        .cosmic()
                        .background(false)
                        .component
                        .on
                        .with_alpha(0.4)
                };

                cosmic::iced::widget::container::Style {
                    text_color: Some(color.into()),
                    ..Default::default()
                }
            })
            .into();

        if let Some(message) = step.on_press {
            children.push(
                button::custom(styled_text)
                    .class(cosmic::theme::Button::Link)
                    .padding(0)
                    .on_press(message)
                    .into(),
            );
        } else {
            children.push(styled_text);
        }

        if index > 0 {
            let separator =
                widget::container(widget::text::caption("  >  ".to_string())).style(|theme| {
                    cosmic::iced::widget::container::Style {
                        text_color: Some(
                            theme
                                .cosmic()
                                .background(false)
                                .component
                                .on
                                .with_alpha(0.3)
                                .into(),
                        ),
                        ..Default::default()
                    }
                });
            children.insert(children.len() - 1, separator.into());
        }
    }

    widget::Row::from_vec(children)
        .align_y(cosmic::iced::Alignment::Center)
        .into()
}

pub(crate) fn wizard_step_shell<'a, Message: Clone + 'static>(
    title: Element<'a, Message>,
    breadcrumb: Element<'a, Message>,
    content: Element<'a, Message>,
    footer: Element<'a, Message>,
) -> Element<'a, Message> {
    let header = iced_widget::column![title, breadcrumb]
        .spacing(8)
        .width(Length::Fill);

    wizard_shell(header.into(), content, footer)
}

pub(crate) fn option_tile_grid<'a, Message: Clone + 'static>(
    tiles: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    widget::flex_row(tiles)
        .row_spacing(12)
        .column_spacing(12)
        .into()
}

pub(crate) fn selectable_tile<'a, Message: Clone + 'static>(
    content: Element<'a, Message>,
    selected: bool,
    on_press: Option<Message>,
    width: Length,
    height: Length,
) -> Element<'a, Message> {
    let mut tile = button::custom(
        widget::container(content)
            .padding(16)
            .width(width)
            .height(height)
            .align_x(cosmic::iced::alignment::Horizontal::Center)
            .align_y(cosmic::iced::alignment::Vertical::Center),
    )
    .class(if selected {
        cosmic::theme::Button::Suggested
    } else {
        cosmic::theme::Button::Standard
    });

    if let Some(message) = on_press {
        tile = tile.on_press(message);
    }

    tile.into()
}

#[cfg(test)]
mod tests {
    use super::wizard_step_is_clickable;

    #[test]
    fn breadcrumb_previous_step_is_clickable() {
        assert!(wizard_step_is_clickable(1, 3));
    }

    #[test]
    fn breadcrumb_current_step_is_not_clickable() {
        assert!(!wizard_step_is_clickable(2, 2));
    }

    #[test]
    fn breadcrumb_future_step_is_not_clickable() {
        assert!(!wizard_step_is_clickable(3, 2));
    }
}

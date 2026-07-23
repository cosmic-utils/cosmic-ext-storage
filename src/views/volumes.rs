use cosmic::{
    Element,
    cosmic_theme::palette::WithAlpha,
    iced::widget::column,
    iced::{Alignment, Background, Length, Shadow},
    widget::{
        self, container,
        text::{caption, caption_heading},
    },
};

use crate::fl;
use crate::message::volumes::VolumesControlMessage;
use crate::state::volumes::{Segment, ToggleState, VolumesControl};
use crate::utils::DiskSegmentKind;

use crate::{app::Message, models::UiVolume};
use storage_types::{VolumeKind, bytes_to_pretty};

impl Segment {
    pub fn get_segment_control<'a>(&self) -> Element<'a, Message> {
        if self.kind == DiskSegmentKind::FreeSpace {
            container(
                column![
                    caption_heading(fl!("free-space-caption")).center(),
                    caption(bytes_to_pretty(&self.size, false)).center()
                ]
                .spacing(3)
                .width(Length::Fill)
                .align_x(Alignment::Center),
            )
            .padding(3)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
        } else if self.kind == DiskSegmentKind::Reserved {
            container(
                column![
                    caption_heading(fl!("reserved-space-caption")).center(),
                    caption(bytes_to_pretty(&self.size, false)).center()
                ]
                .spacing(3)
                .width(Length::Fill)
                .align_x(Alignment::Center),
            )
            .padding(3)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
        } else {
            container(
                column![
                    caption_heading(self.name.clone()).center(),
                    caption(bytes_to_pretty(&self.size, false)).center()
                ]
                .spacing(3)
                .align_x(Alignment::Center),
            )
            .padding(3)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
        }
    }
}

impl VolumesControl {
    pub fn view(&self) -> Element<'_, Message> {
        const SEGMENT_BUTTON_HEIGHT: f32 = 97.5;

        let segment_buttons: Vec<Element<Message>> = self
            .segments
            .iter()
            .enumerate()
            .map(|(index, segment)| {
                let container_selected = segment.state && self.selected_volume.is_none();
                let active_state = ToggleState::active_or(&container_selected, ToggleState::Normal);
                let hovered_state =
                    ToggleState::active_or(&container_selected, ToggleState::Hovered);

                let container_volume = segment
                    .volume
                    .as_ref()
                    .and_then(|p| {
                        crate::state::volumes::find_volume_for_partition(&self.volumes, p)
                    })
                    .filter(|v| v.volume.kind == VolumeKind::CryptoContainer);

                if let Some(v) = container_volume {
                    let state_text = if v.volume.locked {
                        fl!("locked")
                    } else {
                        fl!("unlocked")
                    };

                    let top = cosmic::widget::button::custom(
                        container(
                            column![
                                caption_heading(segment.name.clone()).center(),
                                caption(bytes_to_pretty(&segment.size, false)).center(),
                                caption(state_text).center(),
                            ]
                            .spacing(4)
                            .width(Length::Fill)
                            .align_x(Alignment::Center),
                        )
                        .padding(6)
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center),
                    )
                    .on_press(Message::VolumesMessage(
                        VolumesControlMessage::SegmentSelected(index),
                    ))
                    .class(cosmic::theme::Button::Custom {
                        active: Box::new(move |_b, theme| get_button_style(active_state, theme)),
                        disabled: Box::new(|theme| get_button_style(ToggleState::Disabled, theme)),
                        hovered: Box::new(move |_, theme| get_button_style(hovered_state, theme)),
                        pressed: Box::new(|_, theme| get_button_style(ToggleState::Pressed, theme)),
                    })
                    .height(Length::FillPortion(1));

                    let bottom_content: Element<Message> = if v.volume.locked {
                        container(
                            column![caption(fl!("locked")).center()]
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .align_x(Alignment::Center),
                        )
                        .padding(6)
                        .into()
                    } else {
                        let direct = &v.children;
                        let mut col = column![].spacing(8);
                        col = col.width(Length::Fill).height(Length::Fill);

                        if direct.len() == 1 && !direct[0].children.is_empty() {
                            col = col.push(volume_row_compact(
                                index,
                                &direct[0],
                                &direct[0].children,
                                self.selected_volume.as_deref(),
                            ));
                        } else {
                            col = col.push(volume_row_compact(
                                index,
                                v,
                                direct,
                                self.selected_volume.as_deref(),
                            ));
                        }

                        col.into()
                    };

                    let bottom = container(bottom_content)
                        .padding(0)
                        .height(Length::FillPortion(1))
                        .width(Length::Fill);

                    return container(
                        column![top, bottom]
                            .spacing(6)
                            .height(Length::Fixed(SEGMENT_BUTTON_HEIGHT)),
                    )
                    .width(Length::FillPortion(segment.width))
                    .into();
                }

                cosmic::widget::button::custom(segment.get_segment_control())
                    .on_press(Message::VolumesMessage(
                        VolumesControlMessage::SegmentSelected(index),
                    ))
                    .class(cosmic::theme::Button::Custom {
                        active: Box::new(move |_b, theme| get_button_style(active_state, theme)),
                        disabled: Box::new(|theme| get_button_style(ToggleState::Disabled, theme)),
                        hovered: Box::new(move |_, theme| get_button_style(hovered_state, theme)),
                        pressed: Box::new(|_, theme| get_button_style(ToggleState::Pressed, theme)),
                    })
                    .height(Length::Fixed(SEGMENT_BUTTON_HEIGHT))
                    .width(Length::FillPortion(segment.width))
                    .into()
            })
            .collect();

        let _selected = match self.segments.get(self.selected_segment).cloned() {
            Some(segment) => segment,
            None => {
                return container(
                    cosmic::widget::Column::from_vec(vec![
                        cosmic::widget::Row::from_vec(vec![])
                            .spacing(10)
                            .width(Length::Fill)
                            .into(),
                        widget::Row::from_vec(vec![]).width(Length::Fill).into(),
                    ])
                    .spacing(10),
                )
                .width(Length::Fill)
                .padding(10)
                .class(cosmic::style::Container::Card)
                .into();
            }
        };

        let root = cosmic::widget::Row::from_vec(segment_buttons)
            .spacing(10)
            .width(Length::Fill);

        container(root)
            .width(Length::Fill)
            .padding(10)
            .class(cosmic::style::Container::Card)
            .into()
    }
}

fn volume_row_compact<'a>(
    segment_index: usize,
    parent: &UiVolume,
    children: &'a [UiVolume],
    selected_volume: Option<&str>,
) -> Element<'a, Message> {
    let total = parent.size.max(1);
    let mut buttons: Vec<Element<Message>> = Vec::new();

    for child in children {
        let child_device_path = child.device_path().unwrap_or_default();
        let denom = total;
        let width = (((child.size as f64 / denom as f64) * 1000.).log10().ceil() as u16).max(1);

        let col = column![cosmic::widget::text::caption_heading(child.label.clone()).center(),]
            .spacing(4)
            .width(Length::Fill)
            .align_x(Alignment::Center);

        let is_selected = selected_volume.is_some_and(|p| p == child_device_path);
        let active_state = if is_selected {
            ToggleState::Active
        } else {
            ToggleState::Normal
        };
        let hovered_state = if is_selected {
            ToggleState::Active
        } else {
            ToggleState::Hovered
        };

        let b = cosmic::widget::button::custom(container(col).padding(6))
            .on_press(
                VolumesControlMessage::SelectVolume {
                    segment_index,
                    device_path: child_device_path,
                }
                .into(),
            )
            .class(cosmic::theme::Button::Custom {
                active: Box::new(move |_b, theme| get_button_style(active_state, theme)),
                disabled: Box::new(|theme| get_button_style(ToggleState::Disabled, theme)),
                hovered: Box::new(move |_, theme| get_button_style(hovered_state, theme)),
                pressed: Box::new(|_, theme| get_button_style(ToggleState::Pressed, theme)),
            })
            .height(Length::Fill)
            .width(Length::FillPortion(width));

        buttons.push(b.into());
    }

    cosmic::widget::Row::from_vec(buttons)
        .spacing(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn get_button_style(
    state: ToggleState,
    theme: &cosmic::theme::Theme,
) -> cosmic::widget::button::Style {
    let mut base = cosmic::widget::button::Style {
        shadow_offset: Shadow::default().offset,
        background: Some(cosmic::iced::Background::Color(
            theme.cosmic().primary(false).base.into(),
        )),
        overlay: None,
        border_radius: (theme.cosmic().corner_radii.radius_xs).into(),
        border_width: 0.,
        border_color: theme.cosmic().primary(false).base.into(),
        outline_width: 2.,
        outline_color: theme.cosmic().primary(false).base.into(),
        icon_color: None,
        text_color: None,
    };

    match state {
        ToggleState::Normal => {}
        ToggleState::Active => {
            base.border_color = theme.cosmic().accent_color().into();
            base.outline_color = theme.cosmic().accent_color().into();
            base.background = Some(Background::Color(
                theme.cosmic().accent_color().with_alpha(0.2).into(),
            ));
        }
        ToggleState::Disabled => {
            base.border_color = theme.cosmic().primary(false).base.with_alpha(0.35).into();
            base.outline_color = theme.cosmic().primary(false).base.with_alpha(0.35).into();
            base.background = Some(Background::Color(
                theme.cosmic().primary(false).base.with_alpha(0.08).into(),
            ));
        }
        ToggleState::Hovered => {
            base.text_color = Some(theme.cosmic().accent_button.base.into());
            base.background = Some(Background::Color(theme.cosmic().button.hover.into()));
        }
        ToggleState::Pressed => {
            base.border_color = theme.cosmic().accent_color().into();
            base.outline_color = theme.cosmic().accent_color().into();
        }
    }

    base
}

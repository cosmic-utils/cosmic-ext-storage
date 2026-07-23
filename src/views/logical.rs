//! Logical-storage sidebar and detail rendering.

use crate::{
    app::Message,
    controls::logical::{operation_label, operation_status},
    state::logical::{LogicalDetailTab, LogicalState},
};
use cosmic::iced::Length;
use cosmic::widget::{self, icon};
use cosmic::{Apply, Element};

pub(crate) fn sidebar_section(
    state: &LogicalState,
    controls_enabled: bool,
) -> Vec<Element<'static, Message>> {
    let mut rows: Vec<Element<'static, Message>> = vec![
        widget::text::caption_heading("Logical")
            .apply(widget::container)
            .padding([8, 12, 4, 12])
            .into(),
    ];
    if state.loading {
        rows.push(
            widget::container(widget::text::body("Loading logical storage…"))
                .padding([2, 12])
                .into(),
        );
    }
    if let Some(error) = &state.last_refresh_error {
        rows.push(
            widget::container(widget::text::caption(error.clone()))
                .padding([2, 12])
                .into(),
        );
    }
    for entity in &state.entities {
        let selected = state.selected.as_ref() == Some(&entity.id);
        let title = if entity.name.is_empty() {
            entity.id.to_string()
        } else {
            entity.name.clone()
        };
        let indent = if entity.parent_id.is_some() { 28 } else { 12 };
        let icon_name = match entity.kind {
            storage_types::LogicalEntityKind::LvmVolumeGroup => "drive-harddisk-symbolic",
            storage_types::LogicalEntityKind::LvmLogicalVolume => "folder-symbolic",
            storage_types::LogicalEntityKind::LvmPhysicalVolume => "drive-harddisk-symbolic",
            storage_types::LogicalEntityKind::MdRaidArray => "drive-harddisk-symbolic",
            storage_types::LogicalEntityKind::MdRaidMember => "drive-harddisk-symbolic",
            storage_types::LogicalEntityKind::BtrfsFilesystem => "folder-symbolic",
            storage_types::LogicalEntityKind::BtrfsDevice => "drive-harddisk-symbolic",
            storage_types::LogicalEntityKind::BtrfsSubvolume => "folder-symbolic",
        };
        let mut button = widget::button::custom(
            widget::Row::with_children(vec![
                icon::from_name(icon_name).size(16).into(),
                widget::text::body(title).into(),
            ])
            .spacing(8)
            .width(Length::Fill),
        )
        .padding([4, 8])
        .width(Length::Fill)
        .class(if selected {
            cosmic::theme::Button::Suggested
        } else {
            cosmic::theme::Button::Link
        });
        if controls_enabled {
            button = button.on_press(Message::LogicalSelectionChanged(Some(entity.id.clone())));
        }
        rows.push(
            widget::Row::with_children(vec![
                widget::Space::new().width(indent).into(),
                button.into(),
            ])
            .width(Length::Fill)
            .into(),
        );
    }
    rows
}

pub(crate) fn detail<'a>(state: &'a LogicalState) -> Element<'a, Message> {
    let Some(entity) = state.selected_entity() else {
        return widget::text::title1("Select a logical storage item")
            .apply(widget::container)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
    };
    let tabs = [
        (LogicalDetailTab::Overview, "Overview"),
        (LogicalDetailTab::Members, "Members"),
        (LogicalDetailTab::Operations, "Operations"),
        (LogicalDetailTab::Btrfs, "Btrfs"),
    ]
    .into_iter()
    .map(|(tab, label)| {
        let mut button = widget::button::text(label);
        if state.selected_tab != tab {
            button = button.on_press(Message::LogicalDetailTabSelected(tab));
        }
        button.into()
    })
    .collect::<Vec<Element<'static, Message>>>();
    let body: Element<'static, Message> = match state.selected_tab {
        LogicalDetailTab::Overview => widget::Column::with_children(vec![
            widget::text::title2(entity.name.clone()).into(),
            widget::text::body(format!("ID: {}", entity.id)).into(),
            widget::text::body(format!("Size: {} bytes", entity.size_bytes)).into(),
            entity
                .health_status
                .as_ref()
                .map(|status| widget::text::body(format!("Status: {status}")).into())
                .unwrap_or_else(|| widget::Space::new().into()),
        ])
        .spacing(8)
        .into(),
        LogicalDetailTab::Members => widget::Column::with_children(
            entity
                .members
                .iter()
                .map(|member| {
                    widget::text::body(format!(
                        "{}{}",
                        member.name,
                        member
                            .state
                            .as_deref()
                            .map(|state| format!(" — {state}"))
                            .unwrap_or_default()
                    ))
                    .into()
                })
                .collect::<Vec<Element<'static, Message>>>(),
        )
        .spacing(6)
        .into(),
        LogicalDetailTab::Operations => {
            let mut rows: Vec<Element<'static, Message>> = Vec::new();
            for operation in storage_types::all_logical_operations() {
                let label = operation_label(operation);
                if let Err(reason) = operation_status(&entity.capabilities, operation) {
                    rows.push(widget::text::body(format!("{label} — {reason}")).into());
                } else {
                    // A source-visible action remains visible.  Typed action
                    // construction occurs only after its dedicated dialog has
                    // resolved current block references.
                    rows.push(widget::button::text(label).into());
                }
            }
            widget::Column::with_children(rows).spacing(6).into()
        }
        LogicalDetailTab::Btrfs => widget::Column::with_children(
            entity
                .metadata
                .iter()
                .map(|(key, value)| widget::text::body(format!("{key}: {value}")).into())
                .collect::<Vec<Element<'static, Message>>>(),
        )
        .spacing(6)
        .into(),
    };
    let mut children: Vec<Element<'static, Message>> =
        vec![widget::Row::with_children(tabs).spacing(8).into(), body];
    if let Some(pending) = &state.pending {
        children.push(
            widget::text::body(format!(
                "{:?}{}",
                pending.action.operation(),
                pending
                    .entity
                    .as_ref()
                    .map(|entity| format!(" — {entity}"))
                    .unwrap_or_default()
            ))
            .into(),
        );
    }
    widget::container(widget::Column::with_children(children).spacing(16))
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

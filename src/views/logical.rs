//! Logical-storage sidebar and detail rendering.

use crate::{
    app::Message,
    controls::logical::{operation_label, operation_status},
    state::logical::{LogicalDetailTab, LogicalState},
};
use cosmic::iced::Length;
use cosmic::widget;
use cosmic::{Apply, Element};
use storage_contracts::{
    ConfigurationCleanupPolicy, LogicalAction, LvmWipePolicy, MdRaidSyncAction,
};
use storage_types::{
    ConfirmedDestructiveScope, LogicalEntity, LogicalEntityKind, LogicalOperation,
};

fn reviewed_scope(
    entity_ids: Vec<storage_types::LogicalEntityId>,
    device_refs: Vec<storage_types::BlockDeviceRef>,
) -> Result<ConfirmedDestructiveScope, String> {
    ConfirmedDestructiveScope::new(entity_ids, device_refs).map_err(|error| error.to_string())
}

/// Construct only actions whose complete input is already represented by the
/// selected, freshly discovered topology.  Input-taking operations remain
/// visibly blocked until their dedicated typed form can resolve a device ref.
fn immediate_action(
    state: &LogicalState,
    entity: &LogicalEntity,
    operation: LogicalOperation,
) -> Result<LogicalAction, String> {
    match (entity.kind, operation) {
        (LogicalEntityKind::LvmVolumeGroup, LogicalOperation::Delete) => {
            let mut descendants = state
                .entities
                .iter()
                .filter(|candidate| {
                    candidate.parent_id.as_ref() == Some(&entity.id)
                        && candidate.kind == LogicalEntityKind::LvmLogicalVolume
                })
                .map(|candidate| candidate.id.clone())
                .collect::<Vec<_>>();
            descendants.sort();
            Ok(LogicalAction::DeleteLvmVolumeGroup {
                volume_group: entity.id.clone(),
                pv_label_policy: LvmWipePolicy::Preserve,
                configuration_policy: ConfigurationCleanupPolicy::Preserve,
                confirmed_scope: reviewed_scope(descendants, Vec::new())?,
            })
        }
        (LogicalEntityKind::LvmLogicalVolume, LogicalOperation::Delete) => {
            Ok(LogicalAction::DeleteLvmLogicalVolume {
                logical_volume: entity.id.clone(),
                configuration_policy: ConfigurationCleanupPolicy::Preserve,
                confirmed_scope: reviewed_scope(Vec::new(), Vec::new())?,
            })
        }
        (LogicalEntityKind::LvmLogicalVolume, LogicalOperation::Activate) => {
            Ok(LogicalAction::ActivateLvmLogicalVolume {
                logical_volume: entity.id.clone(),
            })
        }
        (LogicalEntityKind::LvmLogicalVolume, LogicalOperation::Deactivate) => {
            Ok(LogicalAction::DeactivateLvmLogicalVolume {
                logical_volume: entity.id.clone(),
            })
        }
        (LogicalEntityKind::MdRaidArray, LogicalOperation::Delete) => {
            let mut members = entity
                .members
                .iter()
                .filter_map(|member| member.device_ref.clone())
                .collect::<Vec<_>>();
            members.sort();
            Ok(LogicalAction::DeleteMdRaidArray {
                array: entity.id.clone(),
                configuration_policy: ConfigurationCleanupPolicy::Preserve,
                confirmed_scope: reviewed_scope(Vec::new(), members)?,
            })
        }
        (LogicalEntityKind::MdRaidArray, LogicalOperation::Start) => {
            Ok(LogicalAction::StartMdRaidArray {
                array: entity.id.clone(),
            })
        }
        (LogicalEntityKind::MdRaidArray, LogicalOperation::Stop) => {
            Ok(LogicalAction::StopMdRaidArray {
                array: entity.id.clone(),
            })
        }
        (LogicalEntityKind::MdRaidArray, LogicalOperation::Check) => {
            Ok(LogicalAction::RequestMdRaidSync {
                array: entity.id.clone(),
                action: MdRaidSyncAction::Check,
            })
        }
        (LogicalEntityKind::MdRaidArray, LogicalOperation::Repair) => {
            Ok(LogicalAction::RequestMdRaidSync {
                array: entity.id.clone(),
                action: MdRaidSyncAction::Repair,
            })
        }
        _ => Err("This operation requires additional typed input.".into()),
    }
}

pub(crate) fn detail<'a>(state: &'a LogicalState) -> Element<'a, Message> {
    if state.loading {
        return widget::text::title1("Loading logical storage…")
            .apply(widget::container)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
    }

    if let Some(error) = &state.last_refresh_error {
        return widget::container(
            widget::Column::with_children(vec![
                widget::text::title1("Logical storage could not be loaded").into(),
                widget::text::body(error.clone()).into(),
                widget::button::text("Retry")
                    .on_press(Message::LogicalViewRequested { device_path: None })
                    .into(),
            ])
            .spacing(12),
        )
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    }

    let Some(entity) = state.selected_entity() else {
        return widget::text::title1("No logical storage was found")
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
                    match immediate_action(state, entity, operation) {
                        Ok(action) => rows.push(
                            widget::button::text(label)
                                .on_press(Message::LogicalActionPrompted(action))
                                .into(),
                        ),
                        Err(reason) => {
                            rows.push(widget::text::body(format!("{label} — {reason}")).into())
                        }
                    }
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

//! Flattened logical-storage detail rendering.

use crate::{
    app::Message,
    controls::{
        actions::{
            IconActionTone, icon_tooltip_action, icon_tooltip_action_toned, trailing_actions_row,
        },
        layout::row_container,
        logical::{operation_label, operation_status},
        usage_pie,
    },
    state::{
        dialogs::LogicalActionForm,
        logical::{LogicalDevicePickerAction, LogicalState},
        sidebar::{SidebarNodeKey, SidebarState},
    },
    views::sidebar::{TreeNode, tree_node_row},
};
use cosmic::iced::{Alignment, Length, alignment::Horizontal};
use cosmic::widget;
use cosmic::{Apply, Element};
use std::collections::BTreeSet;
use storage_contracts::{
    ConfigurationCleanupPolicy, LogicalAction, LvmWipePolicy, MdRaidMemberWipePolicy,
    MdRaidSyncAction,
};
use storage_types::{
    BtrfsMember, BtrfsMemberState, BtrfsPrimaryMember, BtrfsSubvolumeDetails,
    BtrfsSubvolumeHierarchy, BtrfsSubvolumeRef, BtrfsTopologyDiagnostic, ConfirmedDestructiveScope,
    LogicalDisplay, LogicalEntity, LogicalEntityDetails, LogicalEntityKind, LogicalOperation,
    LvmActivationState, LvmPhysicalVolumeState, MdRaidHealth, MdRaidMemberRole,
};

fn reviewed_scope(
    entity_ids: Vec<storage_types::LogicalEntityId>,
    device_refs: Vec<storage_types::BlockDeviceRef>,
) -> Result<ConfirmedDestructiveScope, String> {
    ConfirmedDestructiveScope::new(entity_ids, device_refs).map_err(|error| error.to_string())
}

/// Actions that require no new user input can start a current preflight
/// directly.  Every input-taking operation remains visible with an exact
/// reason until its typed draft supplies a fresh identity.
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
            let LogicalEntityDetails::MdRaidArray(details) = &entity.details else {
                return Err("The MD RAID member details are unavailable.".into());
            };
            let mut members = details
                .members
                .iter()
                .filter_map(|member| member.block.clone())
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
        _ => Err("This operation needs additional values from its typed form.".into()),
    }
}

pub(crate) fn detail<'a>(
    state: &'a LogicalState,
    sidebar: &'a SidebarState,
    controls_enabled: bool,
) -> Element<'a, Message> {
    if state.loading && state.entities.is_empty() {
        return centered("Loading logical storage…");
    }
    if let Some(storage_types::LogicalCandidateResolution::Unavailable { reason, .. }) =
        &state.candidate_resolution
    {
        return unavailable_page(reason);
    }
    if let Some(error) = &state.last_refresh_error
        && state.entities.is_empty()
    {
        return unavailable_page(error);
    }
    let Some(entity) = state.selected_entity() else {
        if state.selected_candidate.is_some() {
            return centered("This logical item is no longer present");
        }
        return centered("No logical storage was found");
    };

    if let LogicalEntityDetails::BtrfsFilesystem(details) = &entity.details {
        return btrfs_detail_page(state, sidebar, entity, details, controls_enabled);
    }

    let mut children: Vec<Element<'static, Message>> = vec![header(state, entity)];
    if let Some(status) = &state.action_status {
        children.push(widget::text::body(status.clone()).into());
    }
    if let Some(pending) = &state.pending {
        let target = pending
            .entity
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "new logical storage".into());
        children.push(
            widget::text::caption(format!(
                "{} in progress for {target}",
                operation_label(pending.action.operation())
            ))
            .into(),
        );
    }
    if let Some(error) = &state.last_refresh_error {
        children.push(widget::text::body(format!("Refresh warning: {error}")).into());
    }
    children.push(summary(entity));
    match &entity.details {
        LogicalEntityDetails::LvmVolumeGroup(details) => {
            children.push(lvm_members(entity, details));
        }
        LogicalEntityDetails::MdRaidArray(details) => {
            children.push(mdraid_members(entity, details));
        }
        _ => {}
    }
    children.push(manage(state, entity));
    widget::container(widget::scrollable(
        widget::Column::with_children(children).spacing(16),
    ))
    .padding(20)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn centered<'a>(label: &'static str) -> Element<'a, Message> {
    widget::text::title1(label)
        .apply(widget::container)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn unavailable_page<'a>(reason: &str) -> Element<'a, Message> {
    widget::container(
        widget::Column::with_children(vec![
            widget::text::title1("Logical storage is unavailable").into(),
            widget::text::body(reason.to_string()).into(),
            widget::button::text("Retry")
                .on_press(Message::LogicalViewRequested { device_path: None })
                .into(),
        ])
        .spacing(12),
    )
    .padding(20)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn header(state: &LogicalState, entity: &LogicalEntity) -> Element<'static, Message> {
    let kind = match entity.kind {
        LogicalEntityKind::LvmVolumeGroup => "LVM volume group",
        LogicalEntityKind::LvmLogicalVolume => "LVM logical volume",
        LogicalEntityKind::LvmPhysicalVolume => "LVM physical volume",
        LogicalEntityKind::MdRaidArray => "MD RAID array",
        LogicalEntityKind::MdRaidMember => "MD RAID member",
        LogicalEntityKind::BtrfsFilesystem => "Btrfs filesystem",
        LogicalEntityKind::BtrfsDevice => "Btrfs device",
        LogicalEntityKind::BtrfsSubvolume => "Btrfs subvolume",
    };
    let refresh = icon_tooltip_action(
        "view-refresh-symbolic",
        "Refresh logical storage",
        Some(Message::LogicalViewRequested { device_path: None }),
        !state.loading,
    );
    widget::Row::with_children(vec![
        widget::Column::with_children(vec![
            widget::text::title1(entity.display_name().to_string()).into(),
            widget::text::caption(kind).into(),
        ])
        .width(Length::Fill)
        .into(),
        trailing_actions_row(vec![refresh]),
    ])
    .align_y(Alignment::Center)
    .into()
}

fn summary(entity: &LogicalEntity) -> Element<'static, Message> {
    let lines = match &entity.details {
        LogicalEntityDetails::LvmVolumeGroup(details) => vec![
            line("Total", byte_display(&details.size)),
            line("Used", byte_display(&details.used)),
            line("Free", byte_display(&details.free)),
            line("Logical volumes", details.logical_volumes.len().to_string()),
            line(
                "Physical volumes",
                details.physical_volumes.len().to_string(),
            ),
            line("UUID", display_value(&details.uuid)),
        ],
        LogicalEntityDetails::LvmLogicalVolume(details) => vec![
            line("Size", byte_display(&details.size)),
            line("State", activation_display(&details.activation)),
            line("Volume group", details.volume_group.to_string()),
            line("Device", display_value(&details.device_path)),
        ],
        LogicalEntityDetails::LvmPhysicalVolume(details) => vec![
            line("Device", display_value(&details.display_path)),
            line("Size", byte_display(&details.size)),
            line("State", physical_volume_state(&details.state)),
        ],
        LogicalEntityDetails::MdRaidArray(details) => vec![
            line("Level", level_display(&details.level)),
            line("Size", byte_display(&details.size)),
            line("Status", health_display(&details.health)),
            line("Running", display_value(&details.running)),
            line("Sync", progress_display(&details.sync_progress)),
        ],
        LogicalEntityDetails::MdRaidMember(details) => vec![
            line("Device", display_value(&details.display_path)),
            line("Size", byte_display(&details.size)),
            line("Role", role_display(&details.role)),
        ],
        LogicalEntityDetails::BtrfsFilesystem(details) => {
            let mut lines = vec![
                line("Filesystem UUID", details.filesystem_uuid.to_string()),
                line("Label", display_value(&details.label)),
                line(
                    "Default subvolume",
                    btrfs_default_subvolume_display(details),
                ),
            ];
            if let Some(usage) = &details.mount_usage {
                lines.push(line(
                    "Mount-point usage",
                    format!(
                        "{} used of {}",
                        storage_types::bytes_to_pretty(&usage.used, false),
                        storage_types::bytes_to_pretty(&usage.total, false)
                    ),
                ));
            }
            lines
        }
        LogicalEntityDetails::BtrfsDevice(details) => vec![
            line("Filesystem", details.filesystem.to_string()),
            line("Device", display_value(&details.member.display_path)),
            line("State", btrfs_state(&details.member.state)),
        ],
        LogicalEntityDetails::BtrfsSubvolume(details) => vec![
            line("Path", details.subvolume.relative_path.to_string()),
            line("Subvolume ID", details.subvolume.id.to_string()),
            line("Filesystem", details.filesystem.to_string()),
        ],
    };
    widget::Column::with_children(lines).spacing(6).into()
}

/// Btrfs follows the approved flat-page mock: a compact identity/usage header,
/// then clean device and subvolume sections rather than a generic diagnostics
/// list. All buttons still emit the same typed actions as the other roots.
fn btrfs_detail_page(
    state: &LogicalState,
    sidebar: &SidebarState,
    entity: &LogicalEntity,
    details: &storage_types::BtrfsFilesystemDetails,
    controls_enabled: bool,
) -> Element<'static, Message> {
    let mut children = vec![btrfs_header(state, entity, details)];
    if let Some(status) = &state.action_status {
        children.push(widget::text::caption(status.clone()).into());
    }
    if let Some(pending) = &state.pending {
        children.push(
            widget::text::caption(format!(
                "{} in progress",
                operation_label(pending.action.operation())
            ))
            .into(),
        );
    }
    if let Some(error) = &state.last_refresh_error {
        children.push(widget::text::caption(format!("Refresh warning: {error}")).into());
    }
    children.push(widget::divider::horizontal::default().into());
    children.push(btrfs_devices(entity, &details.members));
    children.push(widget::divider::horizontal::default().into());
    children.push(btrfs_subvolumes(
        state,
        sidebar,
        entity,
        details,
        controls_enabled,
    ));

    let content = widget::Column::with_children(children)
        .spacing(22)
        .width(Length::Fill)
        .max_width(1_120);
    widget::scrollable(
        widget::container(content)
            .padding(20)
            .width(Length::Fill)
            .align_x(Horizontal::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn btrfs_header(
    state: &LogicalState,
    entity: &LogicalEntity,
    details: &storage_types::BtrfsFilesystemDetails,
) -> Element<'static, Message> {
    let title = details
        .label
        .as_known()
        .filter(|label| !label.trim().is_empty())
        .cloned()
        .or_else(|| {
            details
                .members
                .iter()
                .find_map(|member| member.display_path.as_known().cloned())
        })
        .unwrap_or_else(|| "Btrfs filesystem".into());
    let mut metadata: Vec<Element<'static, Message>> = vec![
        widget::text::body(title)
            .size(20)
            .font(cosmic::font::semibold())
            .into(),
        widget::text::caption("Btrfs filesystem").into(),
        widget::text::caption(format!("UUID: {}", details.filesystem_uuid)).into(),
    ];
    if let Some(usage) = &details.mount_usage {
        metadata.push(widget::text::caption(format!("Mounted at: {}", usage.mount_point)).into());
    }
    metadata.push(
        widget::text::caption(format!(
            "Default subvolume: {}",
            btrfs_default_subvolume_display(details)
        ))
        .into(),
    );

    let set_label = entity
        .capabilities
        .is_allowed(LogicalOperation::SetLabel)
        .then(|| form_for_operation(entity, LogicalOperation::SetLabel))
        .flatten()
        .map(Message::LogicalActionFormRequested);
    let add_device = entity
        .capabilities
        .is_allowed(LogicalOperation::AddMember)
        .then(|| picker_for_operation(entity, LogicalOperation::AddMember))
        .flatten()
        .map(Message::LogicalDevicePickerRequested);
    let resize = entity
        .capabilities
        .is_allowed(LogicalOperation::Resize)
        .then(|| form_for_operation(entity, LogicalOperation::Resize))
        .flatten()
        .map(Message::LogicalActionFormRequested);
    let actions = widget::Row::with_children(vec![
        icon_tooltip_action_toned(
            "view-refresh-symbolic",
            "Refresh logical storage",
            Some(Message::LogicalViewRequested { device_path: None }),
            !state.loading,
            IconActionTone::Accent,
        ),
        icon_tooltip_action_toned(
            "tag-symbolic",
            "Change filesystem label",
            set_label,
            true,
            IconActionTone::Accent,
        ),
        icon_tooltip_action_toned(
            "list-add-symbolic",
            "Add Btrfs device",
            add_device,
            true,
            IconActionTone::Accent,
        ),
        icon_tooltip_action_toned(
            "view-fullscreen-symbolic",
            "Resize filesystem",
            resize,
            true,
            IconActionTone::Accent,
        ),
    ])
    .spacing(4)
    .padding([8, 0, 0, 0]);

    let left = widget::Column::with_children(vec![
        widget::Column::with_children(metadata).spacing(3).into(),
        actions.into(),
    ])
    .spacing(4)
    .width(Length::Fill);
    widget::Row::with_children(vec![
        left.into(),
        widget::container(btrfs_usage(details))
            .width(Length::Fixed(244.0))
            .align_x(Horizontal::Right)
            .into(),
    ])
    .spacing(28)
    .align_y(Alignment::Start)
    .into()
}

fn btrfs_usage(details: &storage_types::BtrfsFilesystemDetails) -> Element<'static, Message> {
    let Some(usage) = &details.mount_usage else {
        return widget::text::caption("Mount-point usage unavailable").into();
    };
    let pie = usage_pie::filesystem_usage_pie(usage.total, usage.used);
    let legend = widget::Column::with_children(vec![
        btrfs_usage_legend(
            "Used",
            storage_types::bytes_to_pretty(&usage.used, false),
            true,
        ),
        btrfs_usage_legend(
            "Available",
            storage_types::bytes_to_pretty(&usage.free, false),
            false,
        ),
    ])
    .spacing(8);
    widget::Row::with_children(vec![pie, legend.into()])
        .spacing(12)
        .align_y(Alignment::Center)
        .into()
}

fn btrfs_usage_legend(label: &'static str, value: String, used: bool) -> Element<'static, Message> {
    widget::Row::with_children(vec![
        btrfs_usage_swatch(used),
        widget::Column::with_children(vec![
            widget::text::caption(label).into(),
            widget::text::body(value).into(),
        ])
        .spacing(1)
        .into(),
    ])
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn btrfs_usage_swatch(used: bool) -> Element<'static, Message> {
    widget::container(widget::Space::new().width(10.0).height(10.0))
        .width(Length::Fixed(10.0))
        .height(Length::Fixed(10.0))
        .style(move |theme: &cosmic::Theme| {
            let color: cosmic::iced::Color = if used {
                usage_pie::segment_color(1)
            } else {
                theme.cosmic().background(false).component.divider.into()
            };
            widget::container::Style {
                background: Some(cosmic::iced::Background::Color(color)),
                border: cosmic::iced::Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn btrfs_section_header(
    title: &'static str,
    description: &'static str,
    actions: Vec<Element<'static, Message>>,
) -> Element<'static, Message> {
    widget::Row::with_children(vec![
        widget::Column::with_children(vec![
            widget::text::title3(title).into(),
            widget::text::caption(description).into(),
        ])
        .spacing(3)
        .width(Length::Fill)
        .into(),
        widget::Row::with_children(actions)
            .spacing(4)
            .align_y(Alignment::Center)
            .into(),
    ])
    .align_y(Alignment::Start)
    .into()
}

fn btrfs_devices(entity: &LogicalEntity, members: &[BtrfsMember]) -> Element<'static, Message> {
    let mut rows = vec![btrfs_section_header(
        "Devices",
        "Filesystem member devices. The state identifies whether the member is writable.",
        Vec::new(),
    )];
    rows.push(widget::Space::new().height(6.0).into());
    for (index, member) in members.iter().enumerate() {
        let path = display_value(&member.display_path);
        let state = btrfs_state(&member.state);
        let size = byte_display(&member.size);
        let remove = entity
            .capabilities
            .is_allowed(LogicalOperation::RemoveMember)
            .then(|| member.block.clone())
            .flatten()
            .map(|device| LogicalAction::RemoveBtrfsDevice {
                filesystem: entity.id.clone(),
                device,
            })
            .map(Message::LogicalActionPrompted);
        rows.push(
            widget::Row::with_children(vec![
                btrfs_member_led(&member.state),
                widget::Column::with_children(vec![
                    widget::text::body(path)
                        .font(cosmic::font::semibold())
                        .into(),
                    widget::text::caption(format!("{state} · {size}")).into(),
                ])
                .spacing(2)
                .width(Length::Fill)
                .into(),
                trailing_actions_row(vec![icon_tooltip_action_toned(
                    "edit-delete-symbolic",
                    "Remove device",
                    remove,
                    true,
                    IconActionTone::Destructive,
                )]),
            ])
            .spacing(10)
            .align_y(Alignment::Center)
            .apply(widget::container)
            .padding([8, 0])
            .into(),
        );
        if index + 1 < members.len() {
            rows.push(widget::divider::horizontal::default().into());
        }
    }
    if members.is_empty() {
        rows.push(widget::text::body("No Btrfs devices were reported.").into());
    }
    widget::Column::with_children(rows).spacing(0).into()
}

fn btrfs_subvolumes(
    state: &LogicalState,
    sidebar: &SidebarState,
    entity: &LogicalEntity,
    details: &storage_types::BtrfsFilesystemDetails,
    controls_enabled: bool,
) -> Element<'static, Message> {
    let create = entity
        .capabilities
        .is_allowed(LogicalOperation::Create)
        .then(|| form_for_operation(entity, LogicalOperation::Create))
        .flatten()
        .map(Message::LogicalActionFormRequested);
    let mut rows = vec![btrfs_section_header(
        "Subvolumes",
        "Nested subvolumes are shown in their filesystem hierarchy.",
        vec![icon_tooltip_action_toned(
            "list-add-symbolic",
            "Create subvolume",
            create,
            true,
            IconActionTone::Accent,
        )],
    )];
    rows.push(widget::Space::new().height(6.0).into());
    let mut subvolumes = btrfs_subvolume_entities(state, &entity.id);
    sort_btrfs_subvolume_entities(&mut subvolumes);
    if subvolumes.is_empty() {
        rows.push(
            widget::text::body("No subvolumes were reported by the selected UDisks Btrfs member.")
                .into(),
        );
        return widget::Column::with_children(rows).spacing(0).into();
    }

    let roots = subvolumes
        .iter()
        .copied()
        .filter(|subvolume| subvolume.parent_id.as_ref() == Some(&entity.id))
        .collect::<Vec<_>>();
    let mut reachable = BTreeSet::new();
    for subvolume in &roots {
        mark_btrfs_subvolume_descendants(state, &entity.id, subvolume, &mut reachable);
    }

    let mut rendered = BTreeSet::new();
    for subvolume in roots {
        push_btrfs_subvolume_tree(
            &mut rows,
            state,
            sidebar,
            entity,
            details,
            subvolume,
            0,
            controls_enabled,
            &mut rendered,
        );
    }
    // The UDisks source validates hierarchy before building entities, but a
    // stale or malformed topology must remain inspectable rather than making
    // a node disappear from the UI.
    for subvolume in subvolumes {
        if !reachable.contains(&subvolume.id) {
            push_btrfs_subvolume_tree(
                &mut rows,
                state,
                sidebar,
                entity,
                details,
                subvolume,
                0,
                controls_enabled,
                &mut rendered,
            );
        }
    }
    widget::Column::with_children(rows).spacing(0).into()
}

fn btrfs_subvolume_entities<'a>(
    state: &'a LogicalState,
    filesystem: &storage_types::LogicalEntityId,
) -> Vec<&'a LogicalEntity> {
    state
        .entities
        .iter()
        .filter(|candidate| {
            candidate.kind == LogicalEntityKind::BtrfsSubvolume
                && matches!(
                    &candidate.details,
                    LogicalEntityDetails::BtrfsSubvolume(details)
                        if &details.filesystem == filesystem
                )
        })
        .collect()
}

fn sort_btrfs_subvolume_entities(subvolumes: &mut Vec<&LogicalEntity>) {
    subvolumes.sort_by(|left, right| {
        left.display_name()
            .cmp(right.display_name())
            .then(left.id.cmp(&right.id))
    });
}

fn mark_btrfs_subvolume_descendants(
    state: &LogicalState,
    filesystem: &storage_types::LogicalEntityId,
    subvolume: &LogicalEntity,
    reachable: &mut BTreeSet<storage_types::LogicalEntityId>,
) {
    if !reachable.insert(subvolume.id.clone()) {
        return;
    }
    for child in btrfs_subvolume_entities(state, filesystem)
        .into_iter()
        .filter(|candidate| candidate.parent_id.as_ref() == Some(&subvolume.id))
    {
        mark_btrfs_subvolume_descendants(state, filesystem, child, reachable);
    }
}

#[allow(clippy::too_many_arguments)]
fn push_btrfs_subvolume_tree(
    out: &mut Vec<Element<'static, Message>>,
    state: &LogicalState,
    sidebar: &SidebarState,
    filesystem: &LogicalEntity,
    filesystem_details: &storage_types::BtrfsFilesystemDetails,
    subvolume_entity: &LogicalEntity,
    depth: u16,
    controls_enabled: bool,
    visited: &mut BTreeSet<storage_types::LogicalEntityId>,
) {
    if !visited.insert(subvolume_entity.id.clone()) {
        return;
    }
    let LogicalEntityDetails::BtrfsSubvolume(subvolume_details) = &subvolume_entity.details else {
        return;
    };
    let subvolume = &subvolume_details.subvolume;
    let primary_available = matches!(
        filesystem_details.primary_member,
        BtrfsPrimaryMember::Selected { .. }
    );
    let mut children = btrfs_subvolume_entities(state, &filesystem.id)
        .into_iter()
        .filter(|candidate| candidate.parent_id.as_ref() == Some(&subvolume_entity.id))
        .collect();
    sort_btrfs_subvolume_entities(&mut children);
    let reference = btrfs_subvolume_ref(filesystem, filesystem_details, subvolume);
    let set_default = reference
        .clone()
        .filter(|_| {
            !subvolume.is_default
                && filesystem
                    .capabilities
                    .is_allowed(LogicalOperation::SetDefaultSubvolume)
        })
        .map(|subvolume| LogicalAction::SetBtrfsDefaultSubvolume {
            filesystem: filesystem.id.clone(),
            subvolume,
        })
        .map(Message::LogicalActionPrompted);
    let snapshot = reference
        .clone()
        .filter(|_| {
            primary_available && filesystem.capabilities.is_allowed(LogicalOperation::Create)
        })
        .map(|source| {
            Message::LogicalActionFormRequested(LogicalActionForm::CreateBtrfsSnapshot {
                filesystem: filesystem.id.clone(),
                source,
                destination: String::new(),
                readonly: false,
            })
        });
    let delete = reference
        .filter(|_| filesystem.capabilities.is_allowed(LogicalOperation::Delete))
        .map(|subvolume| LogicalAction::DeleteBtrfsSubvolume {
            filesystem: filesystem.id.clone(),
            subvolume,
        })
        .map(Message::LogicalActionPrompted);

    let mut title = vec![
        widget::text::body(subvolume.relative_path.to_string())
            .font(cosmic::font::semibold())
            .into(),
    ];
    let subtitle = if subvolume.is_default {
        format!("Default subvolume · ID {}", subvolume.id)
    } else {
        format!("ID {}", subvolume.id)
    };
    title.push(widget::text::caption(subtitle).into());

    let mut actions = Vec::new();
    if let Some(message) = snapshot {
        actions.push(icon_tooltip_action_toned(
            "camera-photo-symbolic",
            "Create snapshot",
            Some(message),
            controls_enabled,
            IconActionTone::Accent,
        ));
    }
    if let Some(message) = set_default {
        actions.push(icon_tooltip_action_toned(
            "emblem-ok-symbolic",
            "Make default subvolume",
            Some(message),
            controls_enabled,
            IconActionTone::Success,
        ));
    }
    if let Some(message) = delete {
        actions.push(icon_tooltip_action_toned(
            "edit-delete-symbolic",
            "Delete subvolume",
            Some(message),
            controls_enabled,
            IconActionTone::Destructive,
        ));
    }

    let key = SidebarNodeKey::LogicalEntity(subvolume_entity.id.to_string());
    let expanded = sidebar.is_expanded(&key);
    out.push(tree_node_row(
        sidebar,
        TreeNode {
            key,
            selected: state.selected.as_ref() == Some(&subvolume_entity.id),
            has_children: !children.is_empty(),
            icon_name: "folder-symbolic",
            title: widget::Column::with_children(title).spacing(2).into(),
            depth,
            select_message: None,
            actions,
        },
        controls_enabled,
    ));

    if expanded {
        for child in children {
            push_btrfs_subvolume_tree(
                out,
                state,
                sidebar,
                filesystem,
                filesystem_details,
                child,
                depth.saturating_add(1),
                controls_enabled,
                visited,
            );
        }
    }
}

fn btrfs_member_led(state: &BtrfsMemberState) -> Element<'static, Message> {
    let kind = match state {
        BtrfsMemberState::Writable => 0_u8,
        BtrfsMemberState::ReadOnly => 1,
        BtrfsMemberState::Unknown { .. } => 2,
    };
    widget::container(widget::Space::new().width(9.0).height(9.0))
        .width(Length::Fixed(9.0))
        .height(Length::Fixed(9.0))
        .style(move |theme: &cosmic::Theme| {
            let cosmic = theme.cosmic();
            let color = match kind {
                0 => cosmic.success_color(),
                1 => cosmic.warning_color(),
                _ => cosmic.background(false).component.on,
            };
            widget::container::Style {
                background: Some(cosmic::iced::Background::Color(color.into())),
                border: cosmic::iced::Border {
                    radius: 9.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

/// A Btrfs subvolume action carries the selected row's native identity plus
/// the object-manager epoch of the deterministic primary member.  We never
/// reconstruct this reference from the rendered relative path.
fn btrfs_subvolume_ref(
    filesystem: &LogicalEntity,
    details: &storage_types::BtrfsFilesystemDetails,
    subvolume: &BtrfsSubvolumeDetails,
) -> Option<BtrfsSubvolumeRef> {
    if !btrfs_subvolume_target_is_unambiguous(subvolume) {
        return None;
    }
    let BtrfsPrimaryMember::Selected { member_id } = &details.primary_member else {
        return None;
    };
    let observed_topology_epoch = details
        .members
        .iter()
        .find(|member| &member.member_id == member_id)
        .and_then(|member| member.block.as_ref())
        .map(|block| block.observed_generation)?;
    Some(BtrfsSubvolumeRef {
        filesystem: filesystem.id.clone(),
        id: subvolume.id,
        expected_relative_path: subvolume.relative_path.clone(),
        expected_parent_id: subvolume.parent_id,
        observed_topology_epoch,
    })
}

/// A missing parent row (notably Btrfs's implicit root, ID 5) does not make
/// this subvolume ambiguous: the backend still revalidates its native ID,
/// path, and parent ID before every mutation. All topology conflicts that can
/// make a row ambiguous remain action-blocking.
fn btrfs_subvolume_target_is_unambiguous(subvolume: &BtrfsSubvolumeDetails) -> bool {
    match &subvolume.hierarchy {
        BtrfsSubvolumeHierarchy::Attached => true,
        BtrfsSubvolumeHierarchy::Unparented { diagnostics } => {
            diagnostics.iter().all(|diagnostic| {
                matches!(
                    diagnostic,
                    BtrfsTopologyDiagnostic::MissingParent { row, parent_id }
                        if row == &subvolume.row_key && Some(*parent_id) == subvolume.parent_id
                )
            })
        }
    }
}

fn lvm_members(
    entity: &LogicalEntity,
    details: &storage_types::LvmVolumeGroupDetails,
) -> Element<'static, Message> {
    let mut rows = vec![widget::text::title3("Members").into()];
    for physical_volume in &details.physical_volumes {
        let remove = entity
            .capabilities
            .is_allowed(LogicalOperation::RemoveMember)
            .then(|| physical_volume.member.block.clone())
            .flatten()
            .map(|device| LogicalAction::RemoveLvmPhysicalVolume {
                volume_group: entity.id.clone(),
                device,
                pv_label_policy: LvmWipePolicy::Preserve,
            });
        rows.push(row_container(
            widget::Row::with_children(vec![
                widget::text::body(display_value(&physical_volume.member.display_path))
                    .width(Length::Fill)
                    .into(),
                trailing_actions_row(vec![icon_tooltip_action(
                    "edit-delete-symbolic",
                    "Remove physical volume",
                    remove.map(Message::LogicalActionPrompted),
                    true,
                )]),
            ]),
            false,
            true,
        ));
    }
    for logical_volume in &details.logical_volumes {
        rows.push(row_container(
            widget::text::body(logical_volume.name.clone()),
            false,
            true,
        ));
    }
    widget::Column::with_children(rows).spacing(8).into()
}

fn mdraid_members(
    entity: &LogicalEntity,
    details: &storage_types::MdRaidArrayDetails,
) -> Element<'static, Message> {
    let mut rows = vec![widget::text::title3("Members").into()];
    for member in &details.members {
        let remove = entity
            .capabilities
            .is_allowed(LogicalOperation::RemoveMember)
            .then(|| member.block.clone())
            .flatten()
            .map(|device| LogicalAction::RemoveMdRaidMember {
                array: entity.id.clone(),
                device,
                member_signature_policy: MdRaidMemberWipePolicy::Preserve,
            });
        rows.push(row_container(
            widget::Row::with_children(vec![
                widget::text::body(display_value(&member.display_path))
                    .width(Length::Fill)
                    .into(),
                trailing_actions_row(vec![icon_tooltip_action(
                    "edit-delete-symbolic",
                    "Remove RAID member",
                    remove.map(Message::LogicalActionPrompted),
                    true,
                )]),
            ]),
            false,
            true,
        ));
    }
    widget::Column::with_children(rows).spacing(8).into()
}

fn manage(state: &LogicalState, entity: &LogicalEntity) -> Element<'static, Message> {
    let mut rows: Vec<Element<'static, Message>> = vec![widget::text::title3("Manage").into()];
    for operation in storage_types::all_logical_operations() {
        if !entity.capabilities.is_supported(operation)
            && entity.capabilities.blocked_reason(operation).is_none()
        {
            continue;
        }
        let label = operation_label(operation);
        let row: Element<'static, Message> = match operation_status(&entity.capabilities, operation)
        {
            Err(reason) => widget::text::body(format!("{label} — {reason}")).into(),
            Ok(()) => match immediate_action(state, entity, operation) {
                Ok(action) => widget::button::text(label)
                    .on_press(Message::LogicalActionPrompted(action))
                    .into(),
                Err(reason) => match form_for_operation(entity, operation) {
                    Some(form) => widget::button::text(label)
                        .on_press(Message::LogicalActionFormRequested(form))
                        .into(),
                    None => match picker_for_operation(entity, operation) {
                        Some(picker) => widget::button::text(label)
                            .on_press(Message::LogicalDevicePickerRequested(picker))
                            .into(),
                        None => widget::text::body(format!("{label} — {reason}")).into(),
                    },
                },
            },
        };
        rows.push(row);
    }
    widget::Column::with_children(rows).spacing(6).into()
}

fn picker_for_operation(
    entity: &LogicalEntity,
    operation: LogicalOperation,
) -> Option<LogicalDevicePickerAction> {
    match (entity.kind, operation) {
        (LogicalEntityKind::LvmVolumeGroup, LogicalOperation::AddMember) => {
            Some(LogicalDevicePickerAction::LvmPhysicalVolume {
                volume_group: entity.id.clone(),
            })
        }
        (LogicalEntityKind::MdRaidArray, LogicalOperation::AddMember) => {
            Some(LogicalDevicePickerAction::MdRaidMember {
                array: entity.id.clone(),
            })
        }
        (LogicalEntityKind::BtrfsFilesystem, LogicalOperation::AddMember) => {
            Some(LogicalDevicePickerAction::BtrfsDevice {
                filesystem: entity.id.clone(),
            })
        }
        _ => None,
    }
}

fn form_for_operation(
    entity: &LogicalEntity,
    operation: LogicalOperation,
) -> Option<LogicalActionForm> {
    match (entity.kind, operation) {
        (LogicalEntityKind::LvmVolumeGroup, LogicalOperation::Create) => {
            Some(LogicalActionForm::CreateLvmLogicalVolume {
                volume_group: entity.id.clone(),
                name: String::new(),
                size_bytes: String::new(),
            })
        }
        (LogicalEntityKind::LvmLogicalVolume, LogicalOperation::Resize) => {
            Some(LogicalActionForm::ResizeLvmLogicalVolume {
                logical_volume: entity.id.clone(),
                size_bytes: String::new(),
            })
        }
        (LogicalEntityKind::BtrfsFilesystem, LogicalOperation::Resize) => {
            Some(LogicalActionForm::ResizeBtrfsFilesystem {
                filesystem: entity.id.clone(),
                size_bytes: String::new(),
            })
        }
        (LogicalEntityKind::BtrfsFilesystem, LogicalOperation::SetLabel) => {
            let label = match &entity.details {
                LogicalEntityDetails::BtrfsFilesystem(details) => {
                    details.label.as_known().cloned().unwrap_or_default()
                }
                _ => String::new(),
            };
            Some(LogicalActionForm::SetBtrfsLabel {
                filesystem: entity.id.clone(),
                label,
            })
        }
        (LogicalEntityKind::BtrfsFilesystem, LogicalOperation::Create) => {
            Some(LogicalActionForm::CreateBtrfsSubvolume {
                filesystem: entity.id.clone(),
                name: String::new(),
            })
        }
        _ => None,
    }
}

fn line(label: &str, value: String) -> Element<'static, Message> {
    widget::text::body(format!("{label}: {value}")).into()
}

fn byte_display(display: &LogicalDisplay<u64>) -> String {
    display
        .as_known()
        .map(|value| storage_types::bytes_to_pretty(value, false))
        .unwrap_or_else(|| "—".into())
}

fn display_value<T: std::fmt::Display>(display: &LogicalDisplay<T>) -> String {
    display
        .as_known()
        .map(ToString::to_string)
        .unwrap_or_else(|| "Unknown".into())
}

fn health_display(display: &LogicalDisplay<MdRaidHealth>) -> String {
    match display {
        LogicalDisplay::Known(MdRaidHealth::Healthy) => "Healthy".into(),
        LogicalDisplay::Known(MdRaidHealth::Degraded) => "Degraded".into(),
        LogicalDisplay::Unknown { .. } => "Unknown".into(),
    }
}

fn activation_display(display: &LogicalDisplay<LvmActivationState>) -> String {
    match display {
        LogicalDisplay::Known(LvmActivationState::Active) => "Active".into(),
        LogicalDisplay::Known(LvmActivationState::Inactive) => "Inactive".into(),
        LogicalDisplay::Unknown { .. } => "Unknown".into(),
    }
}

fn physical_volume_state(display: &LogicalDisplay<LvmPhysicalVolumeState>) -> String {
    match display {
        LogicalDisplay::Known(LvmPhysicalVolumeState::Available) => "Available".into(),
        LogicalDisplay::Unknown { .. } => "Unknown".into(),
    }
}

fn level_display(display: &LogicalDisplay<storage_types::MdRaidLevelName>) -> String {
    display
        .as_known()
        .map(|level| level.as_str().to_string())
        .unwrap_or_else(|| "Unknown".into())
}

fn progress_display(display: &LogicalDisplay<storage_types::ProgressRatio>) -> String {
    display
        .as_known()
        .map(|progress| format!("{:.0}%", progress.as_fraction() * 100.0))
        .unwrap_or_else(|| "Unknown".into())
}

fn role_display(display: &LogicalDisplay<MdRaidMemberRole>) -> String {
    match display {
        LogicalDisplay::Known(MdRaidMemberRole::Active) => "Active".into(),
        LogicalDisplay::Known(MdRaidMemberRole::Spare) => "Spare".into(),
        LogicalDisplay::Unknown { .. } => "Unknown".into(),
    }
}

fn btrfs_default_subvolume_display(details: &storage_types::BtrfsFilesystemDetails) -> String {
    let id = match &details.default_subvolume {
        LogicalDisplay::Known(Some(id)) => *id,
        LogicalDisplay::Known(None) => return "None".into(),
        LogicalDisplay::Unknown { .. } => return "Unknown".into(),
    };

    let matches = details
        .subvolumes
        .iter()
        .filter(|subvolume| subvolume.id == id)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [subvolume] => format!("{} (ID {id})", subvolume.relative_path),
        [] if id.get() == 5 => format!("/ (ID {id})"),
        [] => format!("Unreported subvolume (ID {id})"),
        _ => format!("Ambiguous subvolume (ID {id})"),
    }
}

fn btrfs_state(state: &BtrfsMemberState) -> String {
    match state {
        BtrfsMemberState::Writable => "Writable".into(),
        BtrfsMemberState::ReadOnly => "Read-only".into(),
        BtrfsMemberState::Unknown { .. } => "Unknown".into(),
    }
}

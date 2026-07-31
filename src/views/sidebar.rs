use crate::app::Message;
use crate::controls::layout::{row_container, transparent_button_class};
use crate::models::{UiDrive, UiVolume};
use crate::state::logical::LogicalState;
use crate::state::network::NetworkState;
use crate::state::sidebar::{SidebarNodeKey, SidebarState};
use crate::views::network::network_section;
use cosmic::cosmic_theme::palette::WithAlpha;
use cosmic::iced::Length;
use cosmic::widget::{self, icon};
use cosmic::{Apply, Element};
use std::collections::HashSet;
use storage_types::{
    LogicalEntity, LogicalEntityDetails, LogicalEntityId, LogicalEntityKind, VolumeKind,
};

/// Fixed width for expander button (icon 16px + padding 2px * 2)
const EXPANDER_WIDTH: u16 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    Logical,
    Internal,
    External,
    Images,
}

impl Section {
    fn label(&self) -> String {
        match self {
            Section::Logical => "Logical".to_string(),
            Section::Internal => "Internal".to_string(),
            Section::External => "External".to_string(),
            Section::Images => "Images".to_string(),
        }
    }
}

fn section_for_drive(drive: &UiDrive) -> Section {
    if drive.disk.is_loop || drive.disk.backing_file.is_some() {
        return Section::Images;
    }

    if drive.disk.removable {
        return Section::External;
    }

    Section::Internal
}

fn volume_icon(kind: &VolumeKind) -> &'static str {
    match kind {
        VolumeKind::CryptoContainer => "dialog-password-symbolic",
        VolumeKind::Filesystem => "folder-symbolic",
        VolumeKind::LvmPhysicalVolume => "folder-symbolic",
        VolumeKind::LvmLogicalVolume => "folder-symbolic",
        VolumeKind::Partition => "drive-harddisk-symbolic",
        VolumeKind::Block => "drive-harddisk-symbolic",
    }
}

fn expander_icon(expanded: bool) -> &'static str {
    if expanded {
        "go-down-symbolic"
    } else {
        "go-next-symbolic"
    }
}

fn drive_title(drive: &UiDrive) -> String {
    if let Some(path) = drive.disk.backing_file.as_deref()
        && !path.trim().is_empty()
        && let Some(name) = path.rsplit('/').next()
        && !name.trim().is_empty()
    {
        return name.to_string();
    }

    let vendor = drive.disk.vendor.trim();
    let model = drive.disk.model.trim();

    if vendor.is_empty() && model.is_empty() {
        return drive.disk.display_name();
    }

    if vendor.is_empty() {
        return model.to_string();
    }

    if model.is_empty() {
        return vendor.to_string();
    }

    if model.to_lowercase().starts_with(&vendor.to_lowercase()) {
        model.to_string()
    } else {
        format!("{vendor} {model}")
    }
}

fn section_header(label: String) -> Element<'static, Message> {
    widget::text::caption_heading(label)
        .apply(widget::container)
        .padding([8, 12, 4, 12])
        .into()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LogicalCandidateKind {
    Btrfs,
    LvmPhysicalVolume,
    RaidMember,
}

impl LogicalCandidateKind {
    fn label(self) -> &'static str {
        match self {
            Self::Btrfs => "Btrfs",
            Self::LvmPhysicalVolume => "LVM physical volume",
            Self::RaidMember => "RAID member",
        }
    }

    fn icon_name(self) -> &'static str {
        match self {
            Self::Btrfs => "folder-symbolic",
            Self::LvmPhysicalVolume | Self::RaidMember => "drive-harddisk-symbolic",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LogicalCandidate {
    device_path: String,
    kind: LogicalCandidateKind,
}

fn logical_candidate_kind(volume: &UiVolume) -> Option<LogicalCandidateKind> {
    let id_type = volume.volume.id_type.trim();
    if id_type.eq_ignore_ascii_case("btrfs") {
        return Some(LogicalCandidateKind::Btrfs);
    }
    if volume.volume.kind == VolumeKind::LvmPhysicalVolume
        || id_type.eq_ignore_ascii_case("lvm2_member")
    {
        return Some(LogicalCandidateKind::LvmPhysicalVolume);
    }
    if id_type.eq_ignore_ascii_case("linux_raid_member") {
        return Some(LogicalCandidateKind::RaidMember);
    }
    None
}

fn collect_logical_candidates(volume: &UiVolume, candidates: &mut Vec<LogicalCandidate>) {
    if let (Some(kind), Some(device_path)) = (
        logical_candidate_kind(volume),
        volume.volume.device_path.as_deref(),
    ) {
        candidates.push(LogicalCandidate {
            device_path: device_path.to_string(),
            kind,
        });
    }

    for child in &volume.children {
        collect_logical_candidates(child, candidates);
    }
}

fn logical_candidates(drives: &[UiDrive]) -> Vec<LogicalCandidate> {
    let mut candidates = Vec::new();
    for drive in drives {
        for volume in &drive.volumes {
            collect_logical_candidates(volume, &mut candidates);
        }
    }
    candidates.sort_by(|left, right| left.device_path.cmp(&right.device_path));
    candidates.dedup_by(|left, right| left.device_path == right.device_path);
    candidates
}

fn dimmed_text(text: impl Into<String>) -> Element<'static, Message> {
    widget::container(widget::text::body(text.into()))
        .style(|theme| cosmic::iced::widget::container::Style {
            text_color: Some(
                theme
                    .cosmic()
                    .background(false)
                    .component
                    .on
                    .with_alpha(0.55)
                    .into(),
            ),
            ..Default::default()
        })
        .into()
}

fn logical_candidate_title(candidate: &LogicalCandidate) -> Element<'static, Message> {
    let (directory, file_name) = candidate
        .device_path
        .rsplit_once('/')
        .map(|(directory, file_name)| (format!("{directory}/"), file_name.to_string()))
        .unwrap_or_default();

    widget::Row::with_children(vec![
        dimmed_text(directory),
        widget::text::body(file_name)
            .font(cosmic::font::semibold())
            .into(),
        dimmed_text(" — "),
        widget::text::body(candidate.kind.label())
            .font(cosmic::font::semibold())
            .into(),
    ])
    .spacing(0)
    .align_y(cosmic::iced::Alignment::Center)
    .into()
}

fn image_section_header(controls_enabled: bool) -> Element<'static, Message> {
    let mut children: Vec<Element<'static, Message>> = vec![
        widget::text::caption_heading(Section::Images.label()).into(),
        widget::Space::new().width(Length::Fill).into(),
    ];

    if controls_enabled {
        let new_image_button = widget::tooltip(
            widget::button::custom(icon::from_name("list-add-symbolic").size(20))
                .padding(4)
                .class(cosmic::theme::Button::Link)
                .on_press(Message::NewDiskImage),
            widget::text(crate::fl!("new-disk-image")),
            widget::tooltip::Position::Bottom,
        );

        let attach_image_button = widget::tooltip(
            widget::button::custom(icon::from_name("document-open-symbolic").size(20))
                .padding(4)
                .class(cosmic::theme::Button::Link)
                .on_press(Message::AttachDisk),
            widget::text(crate::fl!("attach-disk-image")),
            widget::tooltip::Position::Bottom,
        );

        children.push(new_image_button.into());
        children.push(attach_image_button.into());
    }

    widget::Row::with_children(children)
        .padding([8, 12, 4, 12])
        .spacing(6)
        .align_y(cosmic::iced::Alignment::Center)
        .into()
}

/// A single row in the app's shared expandable tree control.
///
/// Logical detail views use this directly as well, so hierarchy has the same
/// indentation, expander affordance, selection treatment, and action layout
/// everywhere in the application.
pub(crate) struct TreeNode {
    pub(crate) key: SidebarNodeKey,
    pub(crate) selected: bool,
    pub(crate) has_children: bool,
    pub(crate) icon_name: &'static str,
    pub(crate) title: Element<'static, Message>,
    pub(crate) depth: u16,
    /// A tree row may be structural: its expander and trailing actions still
    /// work, while clicking its label has no separate destination.
    pub(crate) select_message: Option<Message>,
    pub(crate) actions: Vec<Element<'static, Message>>,
}

pub(crate) fn tree_node_row(
    sidebar: &SidebarState,
    node: TreeNode,
    controls_enabled: bool,
) -> Element<'static, Message> {
    let TreeNode {
        key,
        selected,
        has_children,
        icon_name,
        title,
        depth,
        select_message,
        actions,
    } = node;
    let expanded = sidebar.is_expanded(&key);
    let expander = if has_children {
        let mut button =
            widget::button::custom(icon::from_name(expander_icon(expanded)).size(16)).padding(2);
        button = button.class(transparent_button_class(selected));
        if controls_enabled {
            button = button.on_press(Message::SidebarToggleExpanded(key.clone()));
        }
        button.into()
    } else {
        widget::Space::new()
            .width(EXPANDER_WIDTH)
            .height(EXPANDER_WIDTH)
            .into()
    };
    let select_content =
        widget::Row::with_children(vec![icon::from_name(icon_name).size(16).into(), title])
            .spacing(8)
            .align_y(cosmic::iced::Alignment::Center)
            .width(Length::Fill);
    let select: Element<'static, Message> = if let Some(select_message) = select_message {
        let mut select_button = widget::button::custom(select_content)
            .padding(0)
            .width(Length::Fill)
            .class(transparent_button_class(selected));
        if controls_enabled {
            select_button = select_button.on_press(select_message);
        }
        select_button.into()
    } else {
        // Structural rows must not be represented by a disabled button: COSMIC
        // correctly dims disabled content, but a Btrfs subvolume is readable
        // even when selecting it has no separate detail page.
        widget::container(select_content).width(Length::Fill).into()
    };
    let row = widget::Row::with_children(vec![
        expander,
        select,
        widget::Row::with_children(actions).spacing(4).into(),
    ])
    .spacing(8)
    .align_y(cosmic::iced::Alignment::Center)
    .width(Length::Fill);

    let item = row_container(row, selected, controls_enabled);
    const ROW_SPACING: u16 = 8;
    let indent = depth * (EXPANDER_WIDTH + ROW_SPACING);
    if indent > 0 {
        widget::Row::with_children(vec![widget::Space::new().width(indent).into(), item])
            .spacing(0)
            .align_y(cosmic::iced::Alignment::Center)
            .width(Length::Fill)
            .into()
    } else {
        item
    }
}

fn drive_row(
    sidebar: &SidebarState,
    drive: &UiDrive,
    active_drive: Option<&str>,
    controls_enabled: bool,
) -> Element<'static, Message> {
    let key = SidebarNodeKey::Drive(drive.device().to_string());
    let selected = active_drive.is_some_and(|active| active == drive.device());
    let icon_name = if drive.disk.removable {
        "drive-removable-media-symbolic"
    } else {
        "disks-symbolic"
    };
    let mut actions: Vec<Element<'static, Message>> = Vec::new();
    if drive.disk.is_loop || drive.disk.removable || drive.disk.ejectable {
        let mut eject_button =
            widget::button::custom(icon::from_name("media-eject-symbolic").size(16)).padding(4);
        eject_button = eject_button.class(transparent_button_class(selected));
        if controls_enabled {
            eject_button = eject_button.on_press(Message::SidebarDriveEject {
                device_path: drive.device().to_string(),
            });
        }
        actions.push(eject_button.into());
    }

    tree_node_row(
        sidebar,
        TreeNode {
            key,
            selected,
            has_children: !drive.volumes.is_empty(),
            icon_name,
            title: widget::text::body(drive_title(drive))
                .font(cosmic::font::semibold())
                .into(),
            depth: 0,
            select_message: Some(Message::SidebarSelectDrive {
                device_path: drive.device().to_string(),
            }),
            actions,
        },
        controls_enabled,
    )
}

fn volume_row(
    sidebar: &SidebarState,
    drive_block_path: &str,
    node: &UiVolume,
    depth: u16,
    controls_enabled: bool,
) -> Element<'static, Message> {
    let key = SidebarNodeKey::Volume(node.device_path().unwrap_or_default());
    let selected = sidebar.selected_child.as_ref() == Some(&key);

    let title_text = if node.volume.label.trim().is_empty() {
        match node.volume.device_path.as_deref() {
            Some(p) => p.to_string(),
            None => node.device_path().unwrap_or_default(),
        }
    } else {
        node.volume.label.clone()
    };

    let select_msg = Message::SidebarSelectChild {
        device_path: node.device_path().unwrap_or_default(),
    };

    let mut actions: Vec<Element<'static, Message>> = Vec::new();

    if node.is_mounted() {
        let mut unmount_btn =
            widget::button::custom(icon::from_name("media-eject-symbolic").size(16)).padding(4);
        unmount_btn = unmount_btn.class(transparent_button_class(selected));
        if controls_enabled {
            unmount_btn = unmount_btn.on_press(Message::SidebarVolumeUnmount {
                drive: drive_block_path.to_string(),
                device_path: node.device_path().unwrap_or_default(),
            });
        }
        actions.push(unmount_btn.into());
    }

    tree_node_row(
        sidebar,
        TreeNode {
            key,
            selected,
            has_children: !node.children.is_empty(),
            icon_name: volume_icon(&node.volume.kind),
            title: widget::text::body(title_text)
                .font(cosmic::font::semibold())
                .into(),
            depth,
            select_message: Some(select_msg),
            actions,
        },
        controls_enabled,
    )
}

fn push_volume_tree(
    out: &mut Vec<Element<'static, Message>>,
    sidebar: &SidebarState,
    drive_block_path: &str,
    node: &UiVolume,
    depth: u16,
    controls_enabled: bool,
) {
    out.push(volume_row(
        sidebar,
        drive_block_path,
        node,
        depth,
        controls_enabled,
    ));

    let key = SidebarNodeKey::Volume(node.device_path().unwrap_or_default());
    let expanded = sidebar.is_expanded(&key);

    if expanded {
        // Sort children by device_path to maintain disk offset order
        let mut sorted_children: Vec<&UiVolume> = node.children.iter().collect();
        sorted_children.sort_by(|a, b| {
            a.device_path()
                .as_deref()
                .unwrap_or("")
                .cmp(b.device_path().as_deref().unwrap_or(""))
        });

        for child in sorted_children {
            push_volume_tree(
                out,
                sidebar,
                drive_block_path,
                child,
                depth + 1,
                controls_enabled,
            );
        }
    }
}

fn logical_entity_icon(kind: LogicalEntityKind) -> &'static str {
    match kind {
        LogicalEntityKind::LvmVolumeGroup
        | LogicalEntityKind::LvmPhysicalVolume
        | LogicalEntityKind::MdRaidArray
        | LogicalEntityKind::MdRaidMember
        | LogicalEntityKind::BtrfsDevice => "drive-harddisk-symbolic",
        LogicalEntityKind::LvmLogicalVolume
        | LogicalEntityKind::BtrfsFilesystem
        | LogicalEntityKind::BtrfsSubvolume => "folder-symbolic",
    }
}

fn logical_entity_title(entity: &LogicalEntity) -> String {
    if let LogicalEntityDetails::BtrfsFilesystem(details) = &entity.details
        && let Some(path) = details
            .members
            .iter()
            .find_map(|member| member.display_path.as_known())
    {
        return format!("{path} — Btrfs");
    }
    entity.display_name().to_string()
}

fn logical_entity_matches_candidate(entity: &LogicalEntity, candidate: &LogicalCandidate) -> bool {
    match candidate.kind {
        LogicalCandidateKind::Btrfs => {
            entity.kind == LogicalEntityKind::BtrfsFilesystem
                && matches!(
                    &entity.details,
                    storage_types::LogicalEntityDetails::BtrfsFilesystem(details)
                        if details.members.iter().any(|member| {
                            member.display_path.as_known() == Some(&candidate.device_path)
                        })
                )
        }
        LogicalCandidateKind::LvmPhysicalVolume => {
            entity.kind == LogicalEntityKind::LvmVolumeGroup
                && matches!(
                    &entity.details,
                    storage_types::LogicalEntityDetails::LvmVolumeGroup(details)
                        if details.physical_volumes.iter().any(|member| {
                            member.member.display_path.as_known() == Some(&candidate.device_path)
                        })
                )
        }
        LogicalCandidateKind::RaidMember => {
            entity.kind == LogicalEntityKind::MdRaidArray
                && matches!(
                    &entity.details,
                    storage_types::LogicalEntityDetails::MdRaidArray(details)
                        if details.members.iter().any(|member| {
                            member.display_path.as_known() == Some(&candidate.device_path)
                        })
                )
        }
    }
}

fn logical_roots_for_candidate<'a>(
    entities: &'a [LogicalEntity],
    candidate: &LogicalCandidate,
    assigned_roots: &mut HashSet<LogicalEntityId>,
) -> Vec<&'a LogicalEntity> {
    let mut roots = entities
        .iter()
        .filter(|entity| {
            entity.parent_id.is_none() && logical_entity_matches_candidate(entity, candidate)
        })
        .collect::<Vec<_>>();
    roots.sort_by(|left, right| {
        left.display_name()
            .cmp(right.display_name())
            .then(left.id.cmp(&right.id))
    });
    roots.retain(|entity| assigned_roots.insert(entity.id.clone()));
    roots
}

fn logical_filesystem_row(
    sidebar: &SidebarState,
    state: &LogicalState,
    entity: &LogicalEntity,
    controls_enabled: bool,
) -> Element<'static, Message> {
    let key = SidebarNodeKey::LogicalEntity(entity.id.to_string());
    let selected = state.selected.as_ref() == Some(&entity.id);

    tree_node_row(
        sidebar,
        TreeNode {
            key,
            selected,
            has_children: false,
            icon_name: logical_entity_icon(entity.kind),
            title: widget::text::body(logical_entity_title(entity))
                .font(cosmic::font::semibold())
                .into(),
            depth: 0,
            select_message: Some(Message::LogicalSelectionChanged(Some(entity.id.clone()))),
            actions: Vec::new(),
        },
        controls_enabled,
    )
}

fn logical_section(
    sidebar: &SidebarState,
    state: &LogicalState,
    controls_enabled: bool,
) -> Vec<Element<'static, Message>> {
    let mut rows = vec![section_header(Section::Logical.label())];
    let mut assigned_roots = HashSet::new();

    for candidate in logical_candidates(&sidebar.drives) {
        let roots = logical_roots_for_candidate(&state.entities, &candidate, &mut assigned_roots);
        if !roots.is_empty() {
            for root in roots {
                rows.push(logical_filesystem_row(
                    sidebar,
                    state,
                    root,
                    controls_enabled,
                ));
            }
            continue;
        }

        // A second physical member can resolve to a filesystem already shown
        // above. Do not duplicate it, but retain candidates with no loaded
        // logical root so another filesystem can still be opened.
        let resolves_to_shown_root = state.entities.iter().any(|entity| {
            entity.parent_id.is_none() && logical_entity_matches_candidate(entity, &candidate)
        });
        if !resolves_to_shown_root {
            let key = SidebarNodeKey::LogicalCandidate(candidate.device_path.clone());
            let selected = state.selected.is_none()
                && state.selected_device.as_deref() == Some(candidate.device_path.as_str());
            rows.push(tree_node_row(
                sidebar,
                TreeNode {
                    key,
                    selected,
                    has_children: false,
                    icon_name: candidate.kind.icon_name(),
                    title: logical_candidate_title(&candidate),
                    depth: 0,
                    select_message: Some(Message::LogicalViewRequested {
                        device_path: Some(candidate.device_path.clone()),
                    }),
                    actions: Vec::new(),
                },
                controls_enabled,
            ));
        }
    }

    rows
}

pub(crate) fn sidebar(
    app_nav: &cosmic::widget::nav_bar::Model,
    sidebar: &SidebarState,
    network: &NetworkState,
    logical_state: &LogicalState,
    controls_enabled: bool,
) -> Element<'static, Message> {
    let active_drive = sidebar.active_drive_block_path(app_nav);

    let mut internal: Vec<&UiDrive> = Vec::new();
    let mut external: Vec<&UiDrive> = Vec::new();
    let mut images: Vec<&UiDrive> = Vec::new();

    for d in &sidebar.drives {
        match section_for_drive(d) {
            Section::Logical => {}
            Section::Internal => internal.push(d),
            Section::External => external.push(d),
            Section::Images => images.push(d),
        }
    }

    let mut rows: Vec<Element<'static, Message>> = Vec::new();
    rows.extend(logical_section(sidebar, logical_state, controls_enabled));
    if sidebar.drives_loading {
        rows.push(
            widget::container(widget::text::caption("Loading drives…"))
                .padding([4, 12])
                .into(),
        );
    }

    let add_section =
        |rows: &mut Vec<Element<'static, Message>>, section: Section, drives: Vec<&UiDrive>| {
            if section != Section::Images && drives.is_empty() {
                return;
            }
            if section == Section::Images {
                rows.push(image_section_header(controls_enabled));
            } else {
                rows.push(section_header(section.label()));
            }

            for drive in drives {
                rows.push(drive_row(
                    sidebar,
                    drive,
                    active_drive.as_deref(),
                    controls_enabled,
                ));

                let drive_key = SidebarNodeKey::Drive(drive.device().to_string());
                if sidebar.is_expanded(&drive_key) {
                    // Sort volumes by offset to maintain disk order
                    let mut sorted_volumes: Vec<&UiVolume> = drive.volumes.iter().collect();

                    // To get offset, we need to look up the corresponding PartitionInfo by matching device_path with device
                    sorted_volumes.sort_by(|a, b| {
                        // Find offset for each volume by matching device_path with partitions
                        let offset_a = a
                            .volume
                            .device_path
                            .as_ref()
                            .and_then(|dev| drive.partitions.iter().find(|p| &p.device == dev))
                            .map(|p| p.offset)
                            .unwrap_or(0);
                        let offset_b = b
                            .volume
                            .device_path
                            .as_ref()
                            .and_then(|dev| drive.partitions.iter().find(|p| &p.device == dev))
                            .map(|p| p.offset)
                            .unwrap_or(0);
                        offset_a.cmp(&offset_b)
                    });

                    for v in sorted_volumes {
                        push_volume_tree(rows, sidebar, drive.device(), v, 1, controls_enabled);
                    }
                }
            }
        };

    add_section(&mut rows, Section::Internal, internal);
    add_section(&mut rows, Section::External, external);

    // Network section (RClone, Samba, FTP)
    rows.push(network_section(network, controls_enabled).map(Message::Network));

    // Images must remain the bottom-most section.
    add_section(&mut rows, Section::Images, images);

    widget::container::Container::new(
        widget::scrollable(widget::Column::with_children(rows).spacing(2)).height(Length::Fill),
    )
    .class(cosmic::style::Container::Card)
    .height(Length::Fill)
    .into()
}

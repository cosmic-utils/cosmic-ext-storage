use crate::app::Message;
use crate::controls::layout::{row_container, transparent_button_class};
use crate::models::{UiDrive, UiVolume};
use crate::state::network::NetworkState;
use crate::state::sidebar::{SidebarNodeKey, SidebarState};
use crate::views::network::network_section;
use cosmic::iced::Length;
use cosmic::widget::{self, icon};
use cosmic::{Apply, Element};
use storage_types::VolumeKind;

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

fn drive_row(
    sidebar: &SidebarState,
    drive: &UiDrive,
    active_drive: Option<&str>,
    controls_enabled: bool,
) -> Element<'static, Message> {
    let key = SidebarNodeKey::Drive(drive.device().to_string());
    let selected = active_drive.is_some_and(|a| a == drive.device());

    let expanded = sidebar.is_expanded(&key);
    let has_children = !drive.volumes.is_empty();

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

    let drive_icon_name = if drive.disk.removable {
        "drive-removable-media-symbolic"
    } else {
        "disks-symbolic"
    };

    let title = drive_title(drive);

    let mut select_button = widget::button::custom(
        widget::Row::with_children(vec![
            icon::from_name(drive_icon_name).size(16).into(),
            widget::text::body(title)
                .font(cosmic::font::semibold())
                .into(),
        ])
        .spacing(8)
        .align_y(cosmic::iced::Alignment::Center)
        .width(Length::Fill),
    )
    .padding(0)
    .width(Length::Fill)
    .class(transparent_button_class(selected));
    if controls_enabled {
        select_button = select_button.on_press(Message::SidebarSelectDrive {
            device_path: drive.device().to_string(),
        });
    }

    let mut actions: Vec<Element<'static, Message>> = Vec::new();

    // Primary action: eject/remove for removable drives and loop-backed images.
    if drive.disk.is_loop || drive.disk.removable || drive.disk.ejectable {
        let mut eject_btn =
            widget::button::custom(icon::from_name("media-eject-symbolic").size(16)).padding(4);
        eject_btn = eject_btn.class(transparent_button_class(selected));
        if controls_enabled {
            eject_btn = eject_btn.on_press(Message::SidebarDriveEject {
                device_path: drive.device().to_string(),
            });
        }
        actions.push(eject_btn.into());
    }

    let row = widget::Row::with_children(vec![
        expander,
        select_button.into(),
        widget::Row::with_children(actions).spacing(4).into(),
    ])
    .spacing(8)
    .align_y(cosmic::iced::Alignment::Center)
    .width(Length::Fill);

    row_container(row, selected, controls_enabled)
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

    let expanded = sidebar.is_expanded(&key);
    let has_children = !node.children.is_empty();

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

    let mut select_button = widget::button::custom(
        widget::Row::with_children(vec![
            icon::from_name(volume_icon(&node.volume.kind))
                .size(16)
                .into(),
            widget::text::body(title_text)
                .font(cosmic::font::semibold())
                .into(),
        ])
        .spacing(8)
        .align_y(cosmic::iced::Alignment::Center)
        .width(Length::Fill),
    )
    .padding(0)
    .width(Length::Fill)
    .class(transparent_button_class(selected));
    if controls_enabled {
        select_button = select_button.on_press(select_msg.clone());
    }

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

    // Indent accounts for expander width + spacing between elements
    // Each level indents by one expander width (20px) + row spacing (8px)
    const ROW_SPACING: u16 = 8;
    let indent = depth * (EXPANDER_WIDTH + ROW_SPACING);

    let row = widget::Row::with_children(vec![
        expander,
        select_button.into(),
        widget::Row::with_children(actions).spacing(4).into(),
    ])
    .spacing(ROW_SPACING)
    .align_y(cosmic::iced::Alignment::Center)
    .width(Length::Fill);

    let item = row_container(row, selected, controls_enabled);

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

pub(crate) fn sidebar(
    app_nav: &cosmic::widget::nav_bar::Model,
    sidebar: &SidebarState,
    network: &NetworkState,
    controls_enabled: bool,
) -> Element<'static, Message> {
    let active_drive = sidebar.active_drive_block_path(app_nav);

    let mut logical: Vec<&UiDrive> = Vec::new();
    let mut internal: Vec<&UiDrive> = Vec::new();
    let mut external: Vec<&UiDrive> = Vec::new();
    let mut images: Vec<&UiDrive> = Vec::new();

    for d in &sidebar.drives {
        match section_for_drive(d) {
            Section::Logical => logical.push(d),
            Section::Internal => internal.push(d),
            Section::External => external.push(d),
            Section::Images => images.push(d),
        }
    }

    let mut rows: Vec<Element<'static, Message>> = Vec::new();

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

    add_section(&mut rows, Section::Logical, logical);
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
    .into()
}

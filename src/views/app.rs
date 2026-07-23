use crate::controls::wizard::{option_tile_grid, selectable_tile, wizard_action_row, wizard_shell};
use crate::fl;
use crate::message::app::Message;
use crate::message::network::NetworkMessage;
use crate::message::volumes::VolumesControlMessage;
use crate::models::{UiDrive, UiVolume};
use crate::state::app::{AppModel, ContextPage};
use crate::state::dialogs::{DeletePartitionDialog, ShowDialog};
use crate::state::volumes::{DetailTab, Segment, VolumesControl};
use crate::utils::DiskSegmentKind;
use crate::views::btrfs::btrfs_management_section;
use crate::views::dialogs;
use crate::views::disk as disk_header;
use crate::views::network::network_main_view;
use crate::views::settings::{settings, settings_footer};
use crate::views::sidebar;
use cosmic::app::context_drawer as cosmic_context_drawer;
use cosmic::cosmic_theme::palette::WithAlpha;
use cosmic::iced::Color;
use cosmic::iced::Length;
use cosmic::iced::alignment::{Alignment, Horizontal, Vertical};
use cosmic::iced::mouse;
use cosmic::iced::widget as iced_widget;
use cosmic::widget::{self, Space, icon, text_input};
use cosmic::{Apply, Element};
use storage_types::{UsageCategory, VolumeInfo, VolumeKind, bytes_to_pretty};

/// Custom button style for header tabs with accent color background.
fn tab_button_class(active: bool) -> cosmic::theme::Button {
    cosmic::theme::Button::Custom {
        active: Box::new(move |_focused, theme| tab_button_style(active, theme)),
        disabled: Box::new(move |theme| tab_button_style(active, theme)),
        hovered: Box::new(move |_focused, theme| tab_button_style(active, theme)),
        pressed: Box::new(move |_focused, theme| tab_button_style(active, theme)),
    }
}

fn tab_button_style(active: bool, theme: &cosmic::theme::Theme) -> cosmic::widget::button::Style {
    let cosmic = theme.cosmic();

    let (background, text_color) = if active {
        // Active tab: accent background with white text
        (
            Some(cosmic::iced::Background::Color(
                cosmic.accent_color().into(),
            )),
            cosmic.on_accent_color().into(),
        )
    } else {
        // Inactive tab: transparent with accent color text
        (None, cosmic.accent_color().into())
    };

    cosmic::widget::button::Style {
        shadow_offset: Default::default(),
        background,
        overlay: None,
        border_radius: cosmic.corner_radii.radius_s.into(),
        border_width: 0.0,
        border_color: cosmic::iced::Color::TRANSPARENT,
        outline_width: 0.0,
        outline_color: cosmic::iced::Color::TRANSPARENT,
        icon_color: Some(text_color),
        text_color: Some(text_color),
    }
}

/// Elements to pack at the start of the header bar.
pub(crate) fn header_start(_app: &AppModel) -> Vec<Element<'_, Message>> {
    vec![]
}

/// Elements to pack at the end of the header bar.
pub(crate) fn header_end(_app: &AppModel) -> Vec<Element<'_, Message>> {
    vec![
        widget::button::icon(icon::from_name("preferences-system-symbolic"))
            .on_press(Message::ToggleContextPage(ContextPage::Settings))
            .into(),
    ]
}

/// Elements to pack at the center of the header bar.
pub(crate) fn header_center(app: &AppModel) -> Vec<Element<'_, Message>> {
    let mut elements = vec![];

    // Add BTRFS tabs if applicable
    if let Some(volumes_control) = app.nav.active_data::<VolumesControl>()
        && let Some(segment) = volumes_control
            .segments
            .get(volumes_control.selected_segment)
            .or_else(|| volumes_control.segments.first())
    {
        // Determine if this segment contains BTRFS
        let selected_volume_node = segment.volume.as_ref().and_then(|p| {
            crate::state::volumes::find_volume_for_partition(&volumes_control.volumes, p)
        });

        let selected_volume = segment.volume.as_ref().and_then(|p| {
            crate::state::volumes::find_volume_for_partition(&volumes_control.volumes, p)
        });

        let has_btrfs = if let Some(v) = selected_volume_node {
            crate::state::btrfs::detect_btrfs_in_node(v).is_some()
        } else if let Some(v) = selected_volume {
            crate::state::btrfs::detect_btrfs_in_node(v).is_some()
        } else {
            false
        };

        let active_tab = volumes_control.detail_tab;

        // Volume tab
        let is_active = active_tab == DetailTab::VolumeInfo;
        let mut volume_tab = widget::button::text(fl!("volume")).class(tab_button_class(is_active));
        if !is_active {
            volume_tab = volume_tab.on_press(Message::VolumesMessage(
                VolumesControlMessage::SelectDetailTab(DetailTab::VolumeInfo),
            ));
        }

        // Usage tab
        let is_active = active_tab == DetailTab::Usage;
        let mut usage_tab = widget::button::text(fl!("usage")).class(tab_button_class(is_active));
        if !is_active {
            usage_tab = usage_tab.on_press(Message::VolumesMessage(
                VolumesControlMessage::SelectDetailTab(DetailTab::Usage),
            ));
        }

        elements.push(volume_tab.into());
        elements.push(usage_tab.into());

        if has_btrfs {
            let is_active = active_tab == DetailTab::BtrfsManagement;
            let mut btrfs_tab =
                widget::button::text(fl!("btrfs")).class(tab_button_class(is_active));
            if !is_active {
                btrfs_tab = btrfs_tab.on_press(Message::VolumesMessage(
                    VolumesControlMessage::SelectDetailTab(DetailTab::BtrfsManagement),
                ));
            }
            elements.push(btrfs_tab.into());
        }
    }

    elements
}

pub(crate) fn dialog(app: &AppModel) -> Option<Element<'_, Message>> {
    match app.dialog {
        Some(ref d) => match d {
            crate::state::dialogs::ShowDialog::AddPartition(_)
            | crate::state::dialogs::ShowDialog::FormatPartition(_)
            | crate::state::dialogs::ShowDialog::EditPartition(_)
            | crate::state::dialogs::ShowDialog::ResizePartition(_)
            | crate::state::dialogs::ShowDialog::EditMountOptions(_)
            | crate::state::dialogs::ShowDialog::ChangePassphrase(_)
            | crate::state::dialogs::ShowDialog::EditEncryptionOptions(_)
            | crate::state::dialogs::ShowDialog::FormatDisk(_)
            | crate::state::dialogs::ShowDialog::NewDiskImage(_)
            | crate::state::dialogs::ShowDialog::AttachDiskImage(_)
            | crate::state::dialogs::ShowDialog::ImageOperation(_)
            | crate::state::dialogs::ShowDialog::BtrfsCreateSubvolume(_)
            | crate::state::dialogs::ShowDialog::BtrfsCreateSnapshot(_) => None,

            crate::state::dialogs::ShowDialog::DeletePartition(state) => {
                Some(dialogs::confirmation(
                    fl!("delete", name = state.name.clone()),
                    fl!("delete-confirmation", name = state.name.clone()),
                    VolumesControlMessage::Delete.into(),
                    Some(Message::CloseDialog),
                    state.running,
                ))
            }

            crate::state::dialogs::ShowDialog::EditFilesystemLabel(state) => {
                Some(dialogs::edit_filesystem_label(state.clone()))
            }

            crate::state::dialogs::ShowDialog::ConfirmAction(state) => Some(dialogs::confirmation(
                &state.title,
                &state.body,
                state.ok_message.clone(),
                Some(Message::CloseDialog),
                state.running,
            )),

            crate::state::dialogs::ShowDialog::TakeOwnership(state) => {
                Some(dialogs::take_ownership(state.clone()))
            }

            crate::state::dialogs::ShowDialog::UnlockEncrypted(state) => {
                Some(dialogs::unlock_encrypted(state.clone()))
            }

            crate::state::dialogs::ShowDialog::SmartData(state) => {
                Some(dialogs::smart_data(state.clone()))
            }

            crate::state::dialogs::ShowDialog::UnmountBusy(state) => {
                Some(dialogs::unmount_busy(state.clone()))
            }

            crate::state::dialogs::ShowDialog::Info { title, body } => {
                Some(dialogs::info(title, body, Message::CloseDialog))
            }

            crate::state::dialogs::ShowDialog::ConfirmDeleteRemote { name, scope } => {
                let body = format!(
                    "Are you sure you want to delete the remote '{}'? This action cannot be undone.",
                    name
                );
                Some(dialogs::confirmation(
                    "Delete Remote",
                    body,
                    Message::Network(NetworkMessage::ConfirmDeleteRemote {
                        name: name.clone(),
                        scope: *scope,
                    }),
                    Some(Message::CloseDialog),
                    false,
                ))
            }
        },
        None => None,
    }
}

fn full_page_wizard_view(dialog: &ShowDialog) -> Option<Element<'_, Message>> {
    match dialog {
        ShowDialog::AddPartition(state) => Some(dialogs::create_partition(state.clone())),
        ShowDialog::FormatPartition(state) => Some(dialogs::format_partition(state.clone())),
        ShowDialog::EditPartition(state) => Some(dialogs::edit_partition(state.clone())),
        ShowDialog::ResizePartition(state) => Some(dialogs::resize_partition(state.clone())),
        ShowDialog::EditMountOptions(state) => Some(dialogs::edit_mount_options(state.clone())),
        ShowDialog::ChangePassphrase(state) => Some(dialogs::change_passphrase(state.clone())),
        ShowDialog::EditEncryptionOptions(state) => {
            Some(dialogs::edit_encryption_options(state.clone()))
        }
        ShowDialog::FormatDisk(state) => Some(dialogs::format_disk(state.clone())),
        ShowDialog::NewDiskImage(state) => Some(dialogs::new_disk_image(state.as_ref().clone())),
        ShowDialog::AttachDiskImage(state) => {
            Some(dialogs::attach_disk_image(state.as_ref().clone()))
        }
        ShowDialog::ImageOperation(state) => Some(dialogs::image_operation(state.as_ref().clone())),
        ShowDialog::BtrfsCreateSubvolume(state) => Some(dialogs::create_subvolume(state.clone())),
        ShowDialog::BtrfsCreateSnapshot(state) => Some(dialogs::create_snapshot(state.clone())),
        _ => None,
    }
}

/// Allows overriding the default nav bar widget.
pub(crate) fn nav_bar(app: &AppModel) -> Option<Element<'_, cosmic::Action<Message>>> {
    if !app.core.nav_bar_active() {
        return None;
    }

    let controls_enabled = app.dialog.is_none();

    let mut nav = sidebar::sidebar(&app.nav, &app.sidebar, &app.network, controls_enabled)
        .map(Into::into)
        .apply(widget::container)
        .padding(8)
        .class(cosmic::style::Container::Background)
        // Both width and height must be Shrink for flex layout to respect the max_width constraint
        .width(cosmic::iced::Length::Shrink)
        .height(cosmic::iced::Length::Shrink);

    if !app.core.is_condensed() {
        nav = nav.max_width(280);
    }

    Some(Element::from(nav))
}

/// Enables the COSMIC application to create a nav bar with this model.
pub(crate) fn nav_model(app: &AppModel) -> Option<&cosmic::widget::nav_bar::Model> {
    Some(&app.nav)
}

/// Display a context drawer if the context page is requested.
pub(crate) fn context_drawer(
    app: &AppModel,
) -> Option<cosmic_context_drawer::ContextDrawer<'_, Message>> {
    if !app.core.window.show_context {
        return None;
    }

    Some(match app.context_page {
        ContextPage::Settings => cosmic_context_drawer::context_drawer(
            settings(&app.config),
            Message::ToggleContextPage(ContextPage::Settings),
        )
        .footer(settings_footer(&app.filesystem_tools))
        .title(fl!("settings")),
    })
}

/// Describes the interface based on the current state of the application model.
pub(crate) fn view(app: &AppModel) -> Element<'_, Message> {
    if let Some(active_dialog) = app.dialog.as_ref()
        && let Some(wizard_view) = full_page_wizard_view(active_dialog)
    {
        return widget::container(wizard_view)
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }

    if app.network.wizard.is_some()
        || app.network.editor.is_some()
        || app.network.selected.is_some()
    {
        let controls_enabled = app.dialog.is_none();
        return network_main_view(&app.network, controls_enabled).map(Message::Network);
    }

    match app.nav.active_data::<UiDrive>() {
        None => widget::text::title1(fl!("no-disk-selected"))
            .apply(widget::container)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .into(),

        Some(drive) => {
            let Some(volumes_control) = app.nav.active_data::<VolumesControl>() else {
                return widget::text::title1(fl!("working"))
                    .apply(widget::container)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
                    .into();
            };

            if volumes_control.detail_tab == DetailTab::Usage {
                return widget::container(usage_tab_view(volumes_control))
                    .padding(20)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into();
            }

            let Some(segment) = volumes_control
                .segments
                .get(volumes_control.selected_segment)
                .or_else(|| volumes_control.segments.first())
            else {
                return widget::text::title1(fl!("no-volumes"))
                    .apply(widget::container)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
                    .into();
            };

            // Calculate actual used space on the disk (sum of filesystem usage)
            // For LUKS containers, aggregate children's usage instead of container's (which is 0)
            let used: u64 = volumes_control
                .segments
                .iter()
                .filter_map(|s| s.volume.as_ref())
                .map(|volume_model| {
                    // Look up the corresponding UiVolume to check if it's a LUKS container
                    if let Some(volume_node) = crate::state::volumes::find_volume_for_partition(
                        &volumes_control.volumes,
                        volume_model,
                    ) {
                        if volume_node.volume.kind == storage_types::VolumeKind::CryptoContainer
                            && !volume_node.children.is_empty()
                        {
                            // Aggregate children's usage for LUKS containers
                            volume_node
                                .children
                                .iter()
                                .filter_map(|child| child.volume.usage.as_ref())
                                .map(|u| u.used)
                                .sum()
                        } else {
                            // Use volume's own usage
                            volume_model.usage.as_ref().map(|u| u.used).unwrap_or(0)
                        }
                    } else {
                        // Fallback to volume model's usage
                        volume_model.usage.as_ref().map(|u| u.used).unwrap_or(0)
                    }
                })
                .sum();

            // Top section: Disk header + volumes control
            let top_section = iced_widget::column![
                disk_header::disk_header(
                    drive,
                    used,
                    &volumes_control.segments,
                    &volumes_control.volumes
                ),
                Space::new().height(10),
                volumes_control.view(),
            ]
            .spacing(10)
            .width(Length::Fill);

            // Bottom section: Volume-specific detail view (2/3 of height)
            let bottom_section =
                volume_detail_view(volumes_control, segment, &app.filesystem_tools);

            // Full layout wrapped in a single scrollable
            widget::scrollable(
                iced_widget::column![
                    widget::container(top_section)
                        .padding(20)
                        .width(Length::Fill),
                    widget::container(bottom_section)
                        .padding(20)
                        .width(Length::Fill),
                ]
                .spacing(0)
                .width(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        }
    }
}

/// Renders the volume detail view for the selected volume with action buttons.
fn volume_detail_view<'a>(
    volumes_control: &'a VolumesControl,
    segment: &'a Segment,
    filesystem_tools: &'a [storage_types::FilesystemToolInfo],
) -> Element<'a, Message> {
    if segment.kind == DiskSegmentKind::Reserved {
        return widget::container(widget::Row::from_vec(vec![]))
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }

    let selected_volume_node = volumes_control.selected_volume_node();
    let selected_volume = segment.volume.as_ref().and_then(|p| {
        crate::state::volumes::find_volume_for_partition(&volumes_control.volumes, p)
    });

    // Determine if this segment contains a BTRFS filesystem (directly or inside LUKS)
    let has_btrfs = if let Some(v) = selected_volume_node {
        crate::state::btrfs::detect_btrfs_in_node(v).is_some()
    } else if let Some(v) = selected_volume {
        crate::state::btrfs::detect_btrfs_in_node(v).is_some()
    } else {
        false
    };

    // Build the tab content based on selected tab
    let tab_content: Element<'a, Message> = if volumes_control.detail_tab == DetailTab::Usage {
        usage_tab_view(volumes_control)
    } else if has_btrfs && volumes_control.detail_tab == DetailTab::BtrfsManagement {
        // BTRFS Management tab
        if let Some(btrfs_state) = &volumes_control.btrfs_state
            && let Some(volume) = &segment.volume
        {
            btrfs_management_section(volume, btrfs_state)
        } else if volumes_control.btrfs_state.is_some() {
            widget::text("No volume data available").into()
        } else {
            widget::text("Initializing BTRFS state...").into()
        }
    } else {
        // Volume Info tab (default)
        if let Some(v) = selected_volume_node {
            build_volume_node_info(v, volumes_control, segment, selected_volume)
        } else if let Some(ref p) = segment.volume {
            build_partition_info(p, selected_volume, volumes_control, segment)
        } else {
            build_free_space_info(segment, filesystem_tools)
        }
    };

    // Return the tab content directly (tabs are now in header)
    tab_content
}

fn usage_category_button_class(index: usize, active: bool) -> cosmic::theme::Button {
    cosmic::theme::Button::Custom {
        active: Box::new(move |_focused, theme| usage_category_button_style(index, active, theme)),
        disabled: Box::new(move |theme| usage_category_button_style(index, active, theme)),
        hovered: Box::new(move |_focused, theme| usage_category_button_style(index, active, theme)),
        pressed: Box::new(move |_focused, theme| usage_category_button_style(index, active, theme)),
    }
}

fn usage_category_button_style(
    index: usize,
    active: bool,
    theme: &cosmic::theme::Theme,
) -> cosmic::widget::button::Style {
    let base_color = crate::controls::usage_pie::segment_color(index);
    let text_color = if active { Color::WHITE } else { base_color };

    cosmic::widget::button::Style {
        shadow_offset: Default::default(),
        background: if active {
            Some(cosmic::iced::Background::Color(base_color))
        } else {
            None
        },
        overlay: None,
        border_radius: theme.cosmic().corner_radii.radius_s.into(),
        border_width: 1.0,
        border_color: base_color,
        outline_width: 0.0,
        outline_color: Color::TRANSPARENT,
        icon_color: Some(text_color),
        text_color: Some(text_color),
    }
}

fn usage_category_icon(category: UsageCategory) -> &'static str {
    match category {
        UsageCategory::Documents => "x-office-document-symbolic",
        UsageCategory::Images => "image-x-generic-symbolic",
        UsageCategory::Audio => "audio-x-generic-symbolic",
        UsageCategory::Video => "video-x-generic-symbolic",
        UsageCategory::Archives => "package-x-generic-symbolic",
        UsageCategory::Code => "text-x-script-symbolic",
        UsageCategory::Binaries => "application-x-executable-symbolic",
        UsageCategory::Packages => "package-x-generic-symbolic",
        UsageCategory::System => "computer-symbolic",
        UsageCategory::Other => "folder-symbolic",
    }
}

fn usage_category_label(category: UsageCategory) -> String {
    match category {
        UsageCategory::Documents => fl!("usage-category-documents"),
        UsageCategory::Images => fl!("usage-category-images"),
        UsageCategory::Audio => fl!("usage-category-audio"),
        UsageCategory::Video => fl!("usage-category-video"),
        UsageCategory::Archives => fl!("usage-category-archives"),
        UsageCategory::Code => fl!("usage-category-code"),
        UsageCategory::Binaries => fl!("usage-category-binaries"),
        UsageCategory::Packages => fl!("usage-category-packages"),
        UsageCategory::System => fl!("usage-category-system"),
        UsageCategory::Other => fl!("usage-category-other"),
    }
}

fn usage_tab_view<'a>(volumes_control: &'a VolumesControl) -> Element<'a, Message> {
    let usage_state = &volumes_control.usage_state;

    if usage_state.loading {
        let fraction = if usage_state.progress_estimated_total_bytes > 0 {
            (usage_state.progress_processed_bytes as f64
                / usage_state.progress_estimated_total_bytes as f64)
                .clamp(0.0, 1.0) as f32
        } else {
            0.0
        };

        let processed = bytes_to_pretty(&usage_state.progress_processed_bytes, false);
        let total = bytes_to_pretty(&usage_state.progress_estimated_total_bytes.max(1), false);

        let loading = iced_widget::column![
            iced_widget::row![
                widget::text(fl!("usage-scanning")).size(16),
                widget::Space::new().width(Length::Fill),
                widget::text::body(format!("{} / {}", processed, total)),
            ]
            .align_y(Alignment::Center)
            .width(Length::Fill),
            iced_widget::progress_bar(0.0..=1.0, fraction).length(Length::Fill),
        ]
        .spacing(10)
        .width(Length::Fill)
        .max_width(560);

        return widget::container(loading)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .into();
    }

    if usage_state.wizard_open {
        return usage_scan_wizard_view(usage_state);
    }

    if usage_state.result.is_none()
        && let Some(error) = &usage_state.error
    {
        return iced_widget::column![
            widget::text(fl!("usage-scan-failed")).size(16),
            widget::text::caption(error.clone()),
        ]
        .spacing(8)
        .into();
    }

    let Some(scan_result) = &usage_state.result else {
        return widget::text(fl!("usage-scan-not-started")).into();
    };

    let status_line: Option<String> = if let Some(error) = &usage_state.error {
        Some(error.clone())
    } else {
        usage_state.operation_status.clone()
    };

    let used_total_bytes: u64 = scan_result
        .categories
        .iter()
        .map(|category| category.bytes)
        .sum();
    let unused_bytes = scan_result.total_free_bytes;
    let total_bytes_for_bar = used_total_bytes.saturating_add(unused_bytes).max(1);
    let totals_line = format!(
        "{} / {} ({})",
        bytes_to_pretty(&used_total_bytes, false),
        bytes_to_pretty(&total_bytes_for_bar, false),
        bytes_to_pretty(&unused_bytes, false)
    );

    let non_zero_categories: Vec<_> = scan_result
        .categories
        .iter()
        .filter(|category| category.bytes > 0)
        .collect();

    let segmented_bar: Element<'a, Message> = if non_zero_categories.is_empty() && unused_bytes == 0
    {
        widget::container(
            widget::Space::new()
                .width(Length::Fill)
                .height(Length::Fixed(36.0)),
        )
        .class(cosmic::style::Container::List)
        .into()
    } else {
        let row_with_categories = non_zero_categories.into_iter().fold(
            iced_widget::row!().spacing(0).width(Length::Fill),
            |row, category| {
                let portion = ((category.bytes as f64 / total_bytes_for_bar as f64) * 1000.0)
                    .round()
                    .max(1.0) as u16;
                let color = UsageCategory::ALL
                    .iter()
                    .position(|candidate| *candidate == category.category)
                    .map(crate::controls::usage_pie::segment_color)
                    .unwrap_or(crate::controls::usage_pie::segment_color(0));
                row.push(
                    widget::container(
                        widget::Space::new()
                            .width(Length::Fill)
                            .height(Length::Fixed(36.0)),
                    )
                    .style(
                        move |_theme: &cosmic::Theme| iced_widget::container::Style {
                            background: Some(cosmic::iced::Background::Color(color)),
                            ..Default::default()
                        },
                    )
                    .width(Length::FillPortion(portion)),
                )
            },
        );

        if unused_bytes > 0 {
            let unused_portion = ((unused_bytes as f64 / total_bytes_for_bar as f64) * 1000.0)
                .round()
                .max(1.0) as u16;
            row_with_categories
                .push(
                    widget::container(
                        widget::Space::new()
                            .width(Length::Fill)
                            .height(Length::Fixed(36.0)),
                    )
                    .style(move |theme: &cosmic::Theme| iced_widget::container::Style {
                        background: Some(cosmic::iced::Background::Color(
                            theme.cosmic().background(false).component.divider.into(),
                        )),
                        ..Default::default()
                    })
                    .width(Length::FillPortion(unused_portion)),
                )
                .into()
        } else {
            row_with_categories.into()
        }
    };

    let selected_categories: std::collections::HashSet<UsageCategory> =
        usage_state.selected_categories.iter().copied().collect();

    let visible_categories: Vec<_> = scan_result
        .categories
        .iter()
        .filter(|entry| entry.bytes > 0)
        .collect();

    let category_buttons: Vec<Element<'a, Message>> = visible_categories
        .iter()
        .map(|entry| {
            let category = entry.category;
            let index = UsageCategory::ALL
                .iter()
                .position(|candidate| candidate == &category)
                .unwrap_or(0);
            let is_active = selected_categories.contains(&category);

            let tab_text = iced_widget::row![
                icon::from_name(usage_category_icon(category)).size(14),
                widget::text(format!(
                    "{} ({})",
                    usage_category_label(category),
                    bytes_to_pretty(&entry.bytes, false)
                ))
                .size(12),
            ]
            .spacing(6)
            .align_y(Alignment::Center);

            let mut button = widget::button::custom(tab_text)
                .class(usage_category_button_class(index, is_active));
            button = button.on_press(Message::UsageCategoryFilterToggled(category));
            button.into()
        })
        .collect();

    let category_tabs = widget::flex_row(category_buttons)
        .row_spacing(8)
        .column_spacing(8)
        .width(Length::Fill);

    let mut selected_files: Vec<(UsageCategory, &storage_types::UsageTopFileEntry)> = scan_result
        .categories
        .iter()
        .filter(|entry| entry.bytes > 0 && selected_categories.contains(&entry.category))
        .flat_map(|entry| {
            scan_result
                .top_files_by_category
                .iter()
                .find(|top| top.category == entry.category)
                .into_iter()
                .flat_map(move |top| top.files.iter().map(move |file| (entry.category, file)))
        })
        .collect();

    selected_files.sort_by(|(_, left_file), (_, right_file)| {
        right_file
            .bytes
            .cmp(&left_file.bytes)
            .then_with(|| left_file.path.cmp(&right_file.path))
    });

    let selected_path_set: std::collections::HashSet<&str> = usage_state
        .selected_paths
        .iter()
        .map(String::as_str)
        .collect();

    let selected_count = usage_state.selected_paths.len();

    let refresh_button: Element<'a, Message> = widget::tooltip(
        widget::button::icon(icon::from_name("view-refresh-symbolic").size(16))
            .on_press(Message::UsageRefreshRequested),
        widget::text(fl!("refresh")),
        widget::tooltip::Position::Bottom,
    )
    .into();

    let configure_button: Element<'a, Message> = widget::tooltip(
        widget::button::icon(icon::from_name("preferences-system-symbolic").size(16))
            .on_press(Message::UsageConfigureRequested),
        widget::text(fl!("usage-configure")),
        widget::tooltip::Position::Bottom,
    )
    .into();

    let mut clear_selection_icon =
        widget::button::icon(icon::from_name("edit-clear-symbolic").size(16));
    if selected_count > 0 {
        clear_selection_icon = clear_selection_icon.on_press(Message::UsageSelectionClear);
    }
    let clear_selection_button: Element<'a, Message> = widget::tooltip(
        clear_selection_icon,
        widget::text(fl!("usage-clear-selection")),
        widget::tooltip::Position::Bottom,
    )
    .into();

    let mut delete_icon = widget::button::icon(icon::from_name("edit-delete-symbolic").size(16));
    if selected_count > 0 && !usage_state.deleting {
        delete_icon = delete_icon.on_press(Message::UsageDeleteStart);
    }
    let delete_button: Element<'a, Message> = widget::tooltip(
        delete_icon,
        widget::text(fl!("delete-partition")),
        widget::tooltip::Position::Bottom,
    )
    .into();

    let top_files_input = text_input("1-1000", usage_state.top_files_per_category.to_string())
        .width(Length::Fixed(100.0))
        .on_input(|value| {
            value
                .parse::<u32>()
                .map(|parsed| Message::UsageTopFilesPerCategoryChanged(parsed.clamp(1, 1000)))
                .unwrap_or(Message::UsageTopFilesPerCategoryChanged(1))
        });

    let action_bar = iced_widget::row![
        widget::text::caption(fl!("usage-files-per-category")),
        top_files_input,
        refresh_button,
        configure_button,
        widget::Space::new().width(Length::Fill),
        widget::text::body(fl!("usage-selected-count", count = selected_count)),
        clear_selection_button,
        delete_button,
    ]
    .spacing(10)
    .width(Length::Fill)
    .align_y(Alignment::Center);

    let file_rows = selected_files.iter().enumerate().fold(
        iced_widget::column!().spacing(6),
        |column, (index, (category, file))| {
            let full_path = file.path.display().to_string();
            let selected = selected_path_set.contains(full_path.as_str());
            let filename = file
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_string())
                .unwrap_or_else(|| full_path.clone());

            let category_index = UsageCategory::ALL
                .iter()
                .position(|candidate| candidate == category)
                .unwrap_or(0);
            let category_color = crate::controls::usage_pie::segment_color(category_index);

            let row_content = widget::container(
                iced_widget::row![
                    widget::container(icon::from_name(usage_category_icon(*category)).size(16))
                        .style(
                            move |_theme: &cosmic::Theme| iced_widget::container::Style {
                                text_color: Some(category_color),
                                icon_color: Some(category_color),
                                ..Default::default()
                            }
                        )
                        .width(Length::Fixed(24.0)),
                    widget::container(widget::tooltip(
                        widget::text::body(filename).width(Length::Fill),
                        widget::text::caption(full_path.clone()),
                        widget::tooltip::Position::Bottom,
                    ))
                    .width(Length::Fill),
                    widget::text::body(bytes_to_pretty(&file.bytes, false)).width(Length::Shrink),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            )
            .padding([6, 8])
            .width(Length::Fill)
            .class(cosmic::style::Container::custom(move |theme| {
                use cosmic::iced::{Border, Shadow};

                let cosmic = theme.cosmic();
                let accent = cosmic.accent_color();

                cosmic::iced::widget::container::Style {
                    icon_color: if selected { Some(accent.into()) } else { None },
                    text_color: if selected { Some(accent.into()) } else { None },
                    background: if selected {
                        Some(cosmic::iced::Background::Color(
                            accent.with_alpha(0.14).into(),
                        ))
                    } else {
                        None
                    },
                    border: Border {
                        radius: cosmic.corner_radii.radius_s.into(),
                        width: if selected { 1.0 } else { 0.0 },
                        color: if selected {
                            accent.into()
                        } else {
                            Color::TRANSPARENT
                        },
                    },
                    shadow: Shadow::default(),
                    snap: false,
                }
            }));

            column.push(
                widget::mouse_area(row_content)
                    .interaction(mouse::Interaction::Pointer)
                    .on_press(usage_row_selection_message(
                        full_path,
                        index,
                        usage_state.selection_modifiers,
                    )),
            )
        },
    );

    let mut content = iced_widget::column![
        segmented_bar,
        widget::text::caption(totals_line),
        category_tabs,
        Space::new().height(4),
        action_bar,
        iced_widget::row![
            widget::text(" ").width(Length::Fixed(24.0)),
            widget::text(fl!("usage-filename"))
                .font(cosmic::font::semibold())
                .width(Length::Fill),
            widget::text(fl!("size")).font(cosmic::font::semibold()),
        ]
        .spacing(12),
        widget::scrollable(widget::container(file_rows).padding([4, 0])).height(Length::Fill),
    ]
    .spacing(12)
    .width(Length::Fill)
    .height(Length::Fill);

    if let Some(status) = status_line {
        content = content.push(widget::text::caption(status));
    }

    content.into()
}

fn usage_scan_wizard_view<'a>(
    usage_state: &'a crate::state::volumes::UsageTabState,
) -> Element<'a, Message> {
    let mut show_all_toggle =
        widget::checkbox(usage_state.wizard_show_all_files).label(fl!("usage-show-all-root-mode"));
    show_all_toggle = show_all_toggle.on_toggle(Message::UsageWizardShowAllFilesToggled);

    let parallelism_options = vec![
        fl!("usage-parallelism-low"),
        fl!("usage-parallelism-balanced"),
        fl!("usage-parallelism-high"),
    ];

    let selected_parallelism_index = usage_state.wizard_parallelism_preset.to_index();
    let parallelism_dropdown = widget::dropdown(
        parallelism_options,
        Some(selected_parallelism_index),
        Message::UsageWizardParallelismChanged,
    )
    .width(Length::Fill);

    let mount_tiles: Vec<Element<'a, Message>> = usage_state
        .wizard_mount_points
        .iter()
        .cloned()
        .map(|mount_point| {
            let selected = usage_state
                .wizard_selected_mount_points
                .iter()
                .any(|selected_mount| selected_mount == &mount_point);

            let tile_content = iced_widget::column![
                icon::from_name("drive-harddisk-symbolic").size(24),
                widget::text::body(mount_point.clone()).font(cosmic::font::semibold()),
                widget::text::caption(if selected {
                    fl!("usage-selected")
                } else {
                    fl!("usage-not-selected")
                }),
            ]
            .spacing(6)
            .align_x(Alignment::Center)
            .width(Length::Fill);

            selectable_tile(
                tile_content.into(),
                selected,
                Some(Message::UsageWizardMountToggled {
                    mount_point,
                    selected: !selected,
                }),
                Length::Fixed(150.0),
                Length::Fixed(120.0),
            )
        })
        .collect();

    let mut start_button = widget::button::standard(fl!("usage-start-scan"));
    if !usage_state.wizard_loading_mounts && !usage_state.wizard_selected_mount_points.is_empty() {
        start_button = widget::button::suggested(fl!("usage-start-scan"))
            .on_press(Message::UsageWizardStartScan);
    }

    let cancel_button =
        widget::button::standard(fl!("cancel")).on_press(Message::UsageWizardCancel);

    let mut wizard = iced_widget::column![
        widget::text::title3(fl!("usage-choose-mount-points")),
        widget::text::body(fl!("usage-choose-mount-points-desc")),
        widget::Space::new().height(8),
    ]
    .spacing(8)
    .width(Length::Fill);

    if usage_state.wizard_loading_mounts {
        wizard = wizard.push(widget::text::caption(fl!("usage-loading-mount-points")));
    } else if usage_state.wizard_mount_points.is_empty() {
        wizard = wizard.push(widget::text::caption(fl!("usage-no-mount-points")));
    } else {
        wizard = wizard.push(option_tile_grid(mount_tiles));
    }

    wizard = wizard
        .push(widget::Space::new().height(4))
        .push(show_all_toggle)
        .push(widget::text::caption(fl!("usage-parallelism")))
        .push(parallelism_dropdown)
        .width(Length::Fill)
        .max_width(640);

    if let Some(error) = &usage_state.wizard_error {
        wizard = wizard.push(widget::text::caption(error.clone()));
    }

    let header = iced_widget::column![
        widget::text::title2(fl!("usage-scan-setup")),
        widget::text::body(fl!("usage-choose-mount-points-desc")),
    ]
    .spacing(8)
    .width(Length::Fill);

    let footer = wizard_action_row(vec![cancel_button.into()], vec![start_button.into()]);

    wizard_shell(header.into(), wizard.into(), footer)
}

/// Aggregate children's used space for LUKS containers
fn aggregate_children_usage(node: &crate::models::UiVolume) -> u64 {
    node.children
        .iter()
        .filter_map(|child| child.volume.usage.as_ref())
        .map(|u| u.used)
        .sum()
}

fn usage_row_selection_message(
    path: String,
    index: usize,
    modifiers: cosmic::iced::keyboard::Modifiers,
) -> Message {
    if modifiers.shift() {
        Message::UsageSelectionShift { index }
    } else if modifiers.command() {
        Message::UsageSelectionCtrl { path, index }
    } else {
        Message::UsageSelectionSingle { path, index }
    }
}

/// Build info display for a volume (child filesystem/LV) - mirrors disk header layout
fn build_volume_node_info<'a>(
    v: &'a UiVolume,
    _volumes_control: &'a VolumesControl,
    _segment: &'a Segment,
    _selected_volume: Option<&'a UiVolume>,
) -> Element<'a, Message> {
    use crate::controls::usage_pie;

    // Pie chart showing usage (right side, matching disk header layout)
    // For LUKS containers, aggregate children's usage
    let used = if v.volume.kind == VolumeKind::CryptoContainer {
        if !v.children.is_empty() {
            aggregate_children_usage(v)
        } else {
            // Unlocked LUKS with no children or locked LUKS - show 0
            0
        }
    } else {
        v.volume.usage.as_ref().map(|u| u.used).unwrap_or(0)
    };

    // Create a single-segment pie for this volume
    let pie_label = v.volume.label.clone();

    let pie_segment = usage_pie::PieSegmentData {
        name: pie_label,
        used,
    };
    let pie_chart = usage_pie::disk_usage_pie(&[pie_segment], v.size, used, false);

    // Name, filesystem type, mount point (center text column)
    let name_text = widget::text(v.label.clone())
        .size(14.0)
        .font(cosmic::iced::font::Font {
            weight: cosmic::iced::font::Weight::Semibold,
            ..Default::default()
        });

    let contents = if v.id_type.is_empty() {
        match v.kind {
            VolumeKind::Filesystem => fl!("filesystem"),
            VolumeKind::LvmLogicalVolume => fl!("lvm-logical-volume"),
            VolumeKind::LvmPhysicalVolume => fl!("lvm-physical-volume"),
            VolumeKind::CryptoContainer => fl!("luks-container"),
            VolumeKind::Partition => fl!("partition-type"),
            VolumeKind::Block => fl!("block-device"),
        }
    } else {
        v.id_type.to_uppercase()
    };

    let type_text = widget::text::caption(format!("{}: {}", fl!("contents"), contents));

    let device_str = match v.device_path.as_ref() {
        Some(s) => s.clone(),
        None => fl!("unresolved"),
    };
    let device_text = widget::text::caption(format!("{}: {}", fl!("device"), device_str));

    // Only show mount info if it's not a LUKS container (containers don't mount, their children do)
    let text_column = if v.kind == VolumeKind::CryptoContainer {
        iced_widget::column![name_text, type_text, device_text]
            .spacing(4)
            .width(Length::Fill)
    } else {
        let mount_text: Element<Message> = if let Some(mount_point) = v.mount_points.first() {
            iced_widget::row![
                widget::text::caption(format!("{}: ", fl!("mounted-at"))),
                cosmic::widget::button::link(mount_point.clone())
                    .padding(0)
                    .on_press(Message::OpenPath(mount_point.clone()))
            ]
            .align_y(Alignment::Center)
            .into()
        } else {
            widget::text::caption(fl!("not-mounted")).into()
        };

        iced_widget::column![name_text, type_text, device_text, mount_text]
            .spacing(4)
            .width(Length::Fill)
    };

    // Action buttons underneath
    let mut action_buttons = Vec::new();

    // Mount/Unmount
    if v.has_filesystem {
        if v.is_mounted() {
            action_buttons.push(
                widget::tooltip(
                    widget::button::icon(icon::from_name("media-playback-stop-symbolic")).on_press(
                        Message::VolumesMessage(VolumesControlMessage::ChildUnmount(
                            v.device_path().unwrap_or_default(),
                        )),
                    ),
                    widget::text(fl!("unmount")),
                    widget::tooltip::Position::Bottom,
                )
                .into(),
            );
        } else {
            action_buttons.push(
                widget::tooltip(
                    widget::button::icon(icon::from_name("media-playback-start-symbolic"))
                        .on_press(Message::VolumesMessage(VolumesControlMessage::ChildMount(
                            v.device_path().unwrap_or_default(),
                        ))),
                    widget::text(fl!("mount")),
                    widget::tooltip::Position::Bottom,
                )
                .into(),
            );
        }
    }

    // Format (for filesystems, not containers)
    if v.kind == VolumeKind::Filesystem && v.has_filesystem {
        action_buttons.push(
            widget::tooltip(
                widget::button::icon(icon::from_name("edit-clear-symbolic")).on_press(
                    Message::VolumesMessage(VolumesControlMessage::OpenFormatPartition),
                ),
                widget::text(fl!("format")),
                widget::tooltip::Position::Bottom,
            )
            .into(),
        );
    }

    // Label (for filesystems with filesystem type)
    if v.kind == VolumeKind::Filesystem && v.has_filesystem {
        action_buttons.push(
            widget::tooltip(
                widget::button::icon(icon::from_name("tag-symbolic")).on_press(
                    Message::VolumesMessage(VolumesControlMessage::OpenEditFilesystemLabel),
                ),
                widget::text(fl!("label")),
                widget::tooltip::Position::Bottom,
            )
            .into(),
        );
    }

    // Check Filesystem (if mounted)
    if v.is_mounted() {
        action_buttons.push(
            widget::tooltip(
                widget::button::icon(icon::from_name("dialog-question-symbolic")).on_press(
                    Message::VolumesMessage(VolumesControlMessage::OpenCheckFilesystem),
                ),
                widget::text(fl!("check-filesystem")),
                widget::tooltip::Position::Bottom,
            )
            .into(),
        );
    }

    // Repair Filesystem (if has filesystem)
    if v.has_filesystem {
        action_buttons.push(
            widget::tooltip(
                widget::button::icon(icon::from_name("emblem-system-symbolic")).on_press(
                    Message::VolumesMessage(VolumesControlMessage::OpenRepairFilesystem),
                ),
                widget::text(fl!("repair")),
                widget::tooltip::Position::Bottom,
            )
            .into(),
        );
    }

    // Take Ownership (if mounted)
    if v.is_mounted() {
        action_buttons.push(
            widget::tooltip(
                widget::button::icon(icon::from_name("system-users-symbolic")).on_press(
                    Message::VolumesMessage(VolumesControlMessage::OpenTakeOwnership),
                ),
                widget::text(fl!("take-ownership")),
                widget::tooltip::Position::Bottom,
            )
            .into(),
        );
    }

    // Edit Mount Options (if has filesystem)
    if v.has_filesystem {
        action_buttons.push(
            widget::tooltip(
                widget::button::icon(icon::from_name("emblem-documents-symbolic")).on_press(
                    Message::VolumesMessage(VolumesControlMessage::OpenEditMountOptions),
                ),
                widget::text(fl!("edit-mount-options")),
                widget::tooltip::Position::Bottom,
            )
            .into(),
        );
    }

    // Create image from partition (backup via image client)
    action_buttons.push(
        widget::tooltip(
            widget::button::icon(icon::from_name("document-save-as-symbolic"))
                .on_press(Message::CreateDiskFromPartition),
            widget::text(fl!("create-image")),
            widget::tooltip::Position::Bottom,
        )
        .into(),
    );

    // Restore image to partition (restore via image client)
    action_buttons.push(
        widget::tooltip(
            widget::button::icon(icon::from_name("document-revert-symbolic"))
                .on_press(Message::RestoreImageToPartition),
            widget::text(fl!("restore-image")),
            widget::tooltip::Position::Bottom,
        )
        .into(),
    );

    let info_and_actions = iced_widget::column![
        text_column,
        widget::Row::from_vec(action_buttons).spacing(4)
    ]
    .spacing(8);

    // Row layout: info_and_actions | pie_chart (aligned right, shrink to fit)
    iced_widget::Row::new()
        .push(info_and_actions)
        .push(
            widget::container(pie_chart)
                .width(Length::Shrink)
                .align_x(cosmic::iced::alignment::Horizontal::Right),
        )
        .spacing(15)
        .align_y(Alignment::Start)
        .width(Length::Fill)
        .into()
}

/// Build info display for a partition - mirrors disk header layout
fn build_partition_info<'a>(
    v: &'a VolumeInfo,
    volume_node: Option<&'a UiVolume>,
    volumes_control: &'a VolumesControl,
    segment: &'a Segment,
) -> Element<'a, Message> {
    use crate::controls::usage_pie;

    // Look up the corresponding PartitionInfo using device_path from segment
    let partition_info = segment.device_path.as_ref().and_then(|device| {
        volumes_control
            .partitions
            .iter()
            .find(|p| &p.device == device)
    });

    let Some(p) = partition_info else {
        return widget::text("No partition information available").into();
    };

    // Pie chart showing usage (right side, matching disk header layout)
    // For LUKS containers, aggregate children's usage
    let used = if let Some(vol) = volume_node
        && vol.kind == VolumeKind::CryptoContainer
        && !vol.children.is_empty()
    {
        aggregate_children_usage(vol)
    } else {
        v.usage.as_ref().map(|u| u.used).unwrap_or(0)
    };

    // Create a single-segment pie for this partition
    let partition_name = if p.name.is_empty() {
        fl!("partition-number", number = p.number)
    } else {
        fl!(
            "partition-number-with-name",
            number = p.number,
            name = p.name.clone()
        )
    };
    let pie_segment = usage_pie::PieSegmentData {
        name: partition_name.clone(),
        used,
    };
    let pie_chart = usage_pie::disk_usage_pie(&[pie_segment], p.size, used, false);

    // Name, type, mount point (center text column)
    let name_text =
        widget::text(partition_name.clone())
            .size(14.0)
            .font(cosmic::iced::font::Font {
                weight: cosmic::iced::font::Weight::Semibold,
                ..Default::default()
            });

    let mut type_str = p.filesystem_type.as_deref().unwrap_or("").to_uppercase();
    type_str = format!("{} - {}", type_str, &p.type_name);
    let type_text = widget::text::caption(format!("{}: {}", fl!("contents"), type_str));

    let device_str = p.device.clone();
    let device_text = widget::text::caption(format!("{}: {}", fl!("device"), device_str));

    let uuid_text = widget::text::caption(format!("{}: {}", fl!("uuid"), &p.uuid));

    // Only show mount info if it's not a LUKS container (containers don't mount, their children do)
    let text_column = if let Some(v) = volume_node
        && v.volume.kind == VolumeKind::CryptoContainer
    {
        iced_widget::column![name_text, type_text, device_text, uuid_text]
            .spacing(4)
            .width(Length::Fill)
    } else {
        let mount_points = volume_node
            .map(|v| v.mount_points.as_slice())
            .unwrap_or(p.mount_points.as_slice());
        let mount_text: Element<Message> = if let Some(mount_point) = mount_points.first() {
            iced_widget::row![
                widget::text::caption(format!("{}: ", fl!("mounted-at"))),
                cosmic::widget::button::link(mount_point.clone())
                    .padding(0)
                    .on_press(Message::OpenPath(mount_point.clone()))
            ]
            .align_y(Alignment::Center)
            .into()
        } else {
            widget::text::caption(fl!("not-mounted")).into()
        };

        iced_widget::column![name_text, type_text, device_text, uuid_text, mount_text]
            .spacing(4)
            .width(Length::Fill)
    };

    // Action buttons underneath
    let mut action_buttons = Vec::new();

    // Lock/Unlock for LUKS containers
    if let Some(v) = volume_node
        && v.volume.kind == VolumeKind::CryptoContainer
    {
        if v.volume.locked {
            action_buttons.push(
                widget::tooltip(
                    widget::button::icon(icon::from_name("changes-allow-symbolic")).on_press(
                        Message::Dialog(Box::new(ShowDialog::UnlockEncrypted(
                            crate::state::dialogs::UnlockEncryptedDialog {
                                partition_path: p.device.to_string(),
                                partition_name: partition_name.clone(),
                                passphrase: String::new(),
                                error: None,
                                running: false,
                            },
                        ))),
                    ),
                    widget::text(fl!("unlock-button")),
                    widget::tooltip::Position::Bottom,
                )
                .into(),
            );
        } else {
            action_buttons.push(
                widget::tooltip(
                    widget::button::icon(icon::from_name("changes-prevent-symbolic")).on_press(
                        Message::VolumesMessage(VolumesControlMessage::LockContainer),
                    ),
                    widget::text(fl!("lock")),
                    widget::tooltip::Position::Bottom,
                )
                .into(),
            );
        }

        // Change Passphrase (only for unlocked containers)
        if !v.volume.locked {
            action_buttons.push(
                widget::tooltip(
                    widget::button::icon(icon::from_name("document-properties-symbolic")).on_press(
                        Message::VolumesMessage(VolumesControlMessage::OpenChangePassphrase),
                    ),
                    widget::text(fl!("change-passphrase")),
                    widget::tooltip::Position::Bottom,
                )
                .into(),
            );
        }

        // Edit Encryption Options
        action_buttons.push(
            widget::tooltip(
                widget::button::icon(icon::from_name("preferences-system-symbolic")).on_press(
                    Message::VolumesMessage(VolumesControlMessage::OpenEditEncryptionOptions),
                ),
                widget::text(fl!("edit-encryption-options")),
                widget::tooltip::Position::Bottom,
            )
            .into(),
        );
    }

    // Mount/Unmount
    if p.has_filesystem {
        if p.is_mounted() {
            action_buttons.push(
                widget::tooltip(
                    widget::button::icon(icon::from_name("media-playback-stop-symbolic"))
                        .on_press(Message::VolumesMessage(VolumesControlMessage::Unmount)),
                    widget::text(fl!("unmount")),
                    widget::tooltip::Position::Bottom,
                )
                .into(),
            );
        } else {
            action_buttons.push(
                widget::tooltip(
                    widget::button::icon(icon::from_name("media-playback-start-symbolic"))
                        .on_press(Message::VolumesMessage(VolumesControlMessage::Mount)),
                    widget::text(fl!("mount")),
                    widget::tooltip::Position::Bottom,
                )
                .into(),
            );
        }
    }

    // Format
    action_buttons.push(
        widget::tooltip(
            widget::button::icon(icon::from_name("edit-clear-all-symbolic")).on_press(
                Message::VolumesMessage(VolumesControlMessage::OpenFormatPartition),
            ),
            widget::text(fl!("format")),
            widget::tooltip::Position::Bottom,
        )
        .into(),
    );

    // Edit and Resize
    action_buttons.push(
        widget::tooltip(
            widget::button::icon(icon::from_name("edit-symbolic")).on_press(
                Message::VolumesMessage(VolumesControlMessage::OpenEditPartition),
            ),
            widget::text(fl!("edit")),
            widget::tooltip::Position::Bottom,
        )
        .into(),
    );

    // Resize (check if there's space)
    let right_free_bytes = volumes_control
        .segments
        .get(volumes_control.selected_segment.saturating_add(1))
        .filter(|s| s.kind == DiskSegmentKind::FreeSpace)
        .map(|s| s.size)
        .unwrap_or(0);
    let max_size = p.size.saturating_add(right_free_bytes);
    let min_size = p.usage.as_ref().map(|u| u.used).unwrap_or(0).min(max_size);
    let resize_enabled = max_size.saturating_sub(min_size) >= 1024;

    if resize_enabled {
        action_buttons.push(
            widget::tooltip(
                widget::button::icon(icon::from_name("view-fullscreen-symbolic")).on_press(
                    Message::VolumesMessage(VolumesControlMessage::OpenResizePartition),
                ),
                widget::text(fl!("resize")),
                widget::tooltip::Position::Bottom,
            )
            .into(),
        );
    }

    // Label
    action_buttons.push(
        widget::tooltip(
            widget::button::icon(icon::from_name("tag-symbolic")).on_press(
                Message::VolumesMessage(VolumesControlMessage::OpenEditFilesystemLabel),
            ),
            widget::text(fl!("label")),
            widget::tooltip::Position::Bottom,
        )
        .into(),
    );

    // Check Filesystem (if mounted)
    if p.can_mount() && p.is_mounted() {
        action_buttons.push(
            widget::tooltip(
                widget::button::icon(icon::from_name("dialog-question-symbolic")).on_press(
                    Message::VolumesMessage(VolumesControlMessage::OpenCheckFilesystem),
                ),
                widget::text(fl!("check-filesystem")),
                widget::tooltip::Position::Bottom,
            )
            .into(),
        );
    }

    // Repair Filesystem (if filesystem type)
    if p.has_filesystem {
        action_buttons.push(
            widget::tooltip(
                widget::button::icon(icon::from_name("emblem-system-symbolic")).on_press(
                    Message::VolumesMessage(VolumesControlMessage::OpenRepairFilesystem),
                ),
                widget::text(fl!("repair")),
                widget::tooltip::Position::Bottom,
            )
            .into(),
        );
    }

    // Take Ownership (if mounted)
    if p.can_mount() && p.is_mounted() {
        action_buttons.push(
            widget::tooltip(
                widget::button::icon(icon::from_name("system-users-symbolic")).on_press(
                    Message::VolumesMessage(VolumesControlMessage::OpenTakeOwnership),
                ),
                widget::text(fl!("take-ownership")),
                widget::tooltip::Position::Bottom,
            )
            .into(),
        );
    }

    // Edit Mount Options (if filesystem)
    if p.has_filesystem {
        action_buttons.push(
            widget::tooltip(
                widget::button::icon(icon::from_name("emblem-documents-symbolic")).on_press(
                    Message::VolumesMessage(VolumesControlMessage::OpenEditMountOptions),
                ),
                widget::text(fl!("edit-mount-options")),
                widget::tooltip::Position::Bottom,
            )
            .into(),
        );
    }

    // Create image from partition (backup via image client)
    action_buttons.push(
        widget::tooltip(
            widget::button::icon(icon::from_name("document-save-as-symbolic"))
                .on_press(Message::CreateDiskFromPartition),
            widget::text(fl!("create-image")),
            widget::tooltip::Position::Bottom,
        )
        .into(),
    );

    // Restore image to partition (restore via image client)
    action_buttons.push(
        widget::tooltip(
            widget::button::icon(icon::from_name("document-revert-symbolic"))
                .on_press(Message::RestoreImageToPartition),
            widget::text(fl!("restore-image")),
            widget::tooltip::Position::Bottom,
        )
        .into(),
    );

    // Delete
    action_buttons.push(
        widget::tooltip(
            widget::button::icon(icon::from_name("edit-delete-symbolic")).on_press(
                Message::Dialog(Box::new(ShowDialog::DeletePartition(
                    DeletePartitionDialog {
                        name: segment.name.clone(),
                        running: false,
                    },
                ))),
            ),
            widget::text(fl!("delete-partition")),
            widget::tooltip::Position::Bottom,
        )
        .into(),
    );

    let info_and_actions = iced_widget::column![
        text_column,
        widget::Row::from_vec(action_buttons).spacing(4)
    ]
    .spacing(8);

    // Row layout: info_and_actions | pie_chart (aligned right, shrink to fit)
    iced_widget::Row::new()
        .push(info_and_actions)
        .push(
            widget::container(pie_chart)
                .width(Length::Shrink)
                .align_x(cosmic::iced::alignment::Horizontal::Right),
        )
        .spacing(15)
        .align_y(Alignment::Start)
        .width(Length::Fill)
        .into()
}

/// Build info display for free space - mirrors disk header layout
fn build_free_space_info<'a>(
    segment: &'a Segment,
    filesystem_tools: &'a [storage_types::FilesystemToolInfo],
) -> Element<'a, Message> {
    use crate::controls::usage_pie;

    // Empty pie chart for free space (0% used)
    let pie_segment = usage_pie::PieSegmentData {
        name: fl!("free-space-segment"),
        used: 0,
    };
    let pie_chart = usage_pie::disk_usage_pie(&[pie_segment], segment.size, 0, false);

    // Name and size (left text column)
    let name_text =
        widget::text(fl!("free-space-segment"))
            .size(14.0)
            .font(cosmic::iced::font::Font {
                weight: cosmic::iced::font::Weight::Semibold,
                ..Default::default()
            });

    let size_text = widget::text::caption(format!(
        "{}: {}",
        fl!("size"),
        bytes_to_pretty(&segment.size, true)
    ));
    let offset_text = widget::text::caption(format!(
        "{}: {}",
        fl!("offset"),
        bytes_to_pretty(&segment.offset, false)
    ));

    let available_text = widget::text::caption(fl!("can-create-partition"));

    let text_column = iced_widget::column![name_text, size_text, offset_text, available_text]
        .spacing(4)
        .width(Length::Fill);

    // Action button for creating a partition in free space
    let filesystem_tools_clone = filesystem_tools.to_vec();
    let add_partition_button = widget::tooltip(
        widget::button::icon(icon::from_name("list-add-symbolic")).on_press(Message::Dialog(
            Box::new(ShowDialog::AddPartition(
                crate::state::dialogs::CreatePartitionDialog {
                    info: segment.get_create_info(),
                    step: crate::state::dialogs::CreatePartitionStep::Basics,
                    running: false,
                    error: None,
                    filesystem_tools: filesystem_tools_clone,
                },
            )),
        )),
        widget::text(fl!("create-partition")),
        widget::tooltip::Position::Bottom,
    );

    let info_and_actions = iced_widget::column![
        text_column,
        widget::Row::with_children(vec![add_partition_button.into()]).spacing(4)
    ]
    .spacing(8);

    // Row layout: info_and_actions | pie_chart (aligned right, shrink to fit)
    iced_widget::Row::new()
        .push(info_and_actions)
        .push(
            widget::container(pie_chart)
                .width(Length::Shrink)
                .align_x(cosmic::iced::alignment::Horizontal::Right),
        )
        .spacing(15)
        .align_y(Alignment::Start)
        .width(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic::iced::keyboard::Modifiers;

    #[test]
    fn usage_row_selection_message_defaults_to_single_click_selection() {
        let message = usage_row_selection_message("/tmp/a".to_string(), 3, Modifiers::empty());

        match message {
            Message::UsageSelectionSingle { path, index } => {
                assert_eq!(path, "/tmp/a");
                assert_eq!(index, 3);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn usage_row_selection_message_uses_ctrl_for_toggle() {
        let message = usage_row_selection_message("/tmp/b".to_string(), 5, Modifiers::CTRL);

        match message {
            Message::UsageSelectionCtrl { path, index } => {
                assert_eq!(path, "/tmp/b");
                assert_eq!(index, 5);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn usage_row_selection_message_uses_shift_for_range_selection() {
        let message = usage_row_selection_message("/tmp/c".to_string(), 7, Modifiers::SHIFT);

        match message {
            Message::UsageSelectionShift { index } => {
                assert_eq!(index, 7);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }
}

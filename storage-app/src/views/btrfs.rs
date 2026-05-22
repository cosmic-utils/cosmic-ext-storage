use crate::app::Message;
use crate::controls::usage_pie;
use crate::fl;
use crate::message::volumes::VolumesControlMessage;
use crate::state::btrfs::BtrfsState;
use cosmic::Element;
use cosmic::iced::Length;
use cosmic::iced::widget as iced_widget;
use cosmic::widget;
use std::collections::HashMap;
use storage_types::{BtrfsSubvolume, VolumeInfo};

/// Helper to get expander icon name
fn expander_icon(expanded: bool) -> &'static str {
    if expanded {
        "go-down-symbolic"
    } else {
        "go-next-symbolic"
    }
}

/// Builds the BTRFS management section for a BTRFS volume
pub fn btrfs_management_section<'a>(
    volume: &'a VolumeInfo,
    state: &'a BtrfsState,
) -> Element<'a, Message> {
    let header = widget::text(fl!("btrfs-management"))
        .size(14.0)
        .font(cosmic::iced::font::Font {
            weight: cosmic::iced::font::Weight::Semibold,
            ..Default::default()
        });

    let mut content_items: Vec<Element<'a, Message>> = vec![header.into()];

    // === Usage Breakdown Section ===
    // Try to get mount point from state first, then from volume
    let mount_point = state
        .mount_point
        .as_ref()
        .or_else(|| volume.mount_points.first());

    tracing::debug!(
        "btrfs_management_section: state.mount_point={:?}, volume.mount_points={:?}, effective_mount_point={:?}",
        state.mount_point,
        volume.mount_points,
        mount_point
    );

    if let Some(_mount_point) = mount_point {
        // Check if we need to load usage
        if state.used_space.is_none() && !state.loading_usage {
            // Trigger load on next render cycle (will be caught by update handler)
            // Just note that we need to load - the actual message is sent elsewhere
        }

        if state.loading_usage {
            content_items.push(widget::text::caption(fl!("btrfs-loading-usage")).into());
        } else if let Some(used_space_result) = &state.used_space {
            match used_space_result {
                Ok(used_bytes) => {
                    // Create pie chart showing usage
                    let pie_segment = usage_pie::PieSegmentData {
                        name: "BTRFS".to_string(),
                        used: *used_bytes,
                    };
                    let pie_chart = usage_pie::disk_usage_pie(
                        &[pie_segment],
                        volume.size,
                        *used_bytes,
                        false, // no legend
                    );

                    // Display pie chart right-aligned (matching Volume Info layout)
                    content_items.push(
                        widget::container(pie_chart)
                            .width(Length::Fill)
                            .align_x(cosmic::iced::alignment::Horizontal::Right)
                            .into(),
                    );
                }
                Err(error) => {
                    content_items.push(
                        widget::text::caption(fl!("btrfs-usage-error", error = error)).into(),
                    );
                }
            }
        }

        // Spacing after usage section
        content_items.push(widget::space().height(8).into());
    }

    // === Subvolumes Section ===
    // Check if we need to load subvolumes
    if state.subvolumes.is_none() && !state.loading {
        // Try to get mount point from state first, then from volume
        let mount_point = state
            .mount_point
            .as_ref()
            .or_else(|| volume.mount_points.first());

        if let Some(_mp) = mount_point {
            // Note: This will trigger on every render until loaded
            // A better approach would be to trigger once, but this works for now
            content_items.push(widget::text::caption("Loading subvolumes...").into());

            // Send message to load (will be handled in update)
            // For now, just show loading state
        } else {
            // Neither state nor volume has a mount point
            if volume.has_filesystem {
                // Filesystem exists but not detected as mounted
                content_items.push(widget::text::caption(fl!("btrfs-not-mounted-refresh")).into());
            } else {
                content_items.push(widget::text::caption(fl!("btrfs-not-mounted")).into());
            }
        }
    } else if state.loading {
        content_items.push(widget::text::caption(fl!("btrfs-loading-subvolumes")).into());
    } else if let Some(result) = &state.subvolumes {
        match result {
            Ok(subvolumes) => {
                // Add Create buttons row with icon button styling
                let button_row = iced_widget::row![
                    widget::tooltip(
                        widget::button::icon(widget::icon::from_name("list-add-symbolic"))
                            .on_press(Message::VolumesMessage(
                                VolumesControlMessage::OpenBtrfsCreateSubvolume
                            )),
                        widget::text(fl!("btrfs-create-subvolume")),
                        widget::tooltip::Position::Bottom,
                    ),
                    widget::tooltip(
                        widget::button::icon(widget::icon::from_name("camera-photo-symbolic"))
                            .on_press(Message::VolumesMessage(
                                VolumesControlMessage::OpenBtrfsCreateSnapshot
                            )),
                        widget::text(fl!("btrfs-create-snapshot")),
                        widget::tooltip::Position::Bottom,
                    ),
                ]
                .spacing(8);

                content_items.push(button_row.into());

                if subvolumes.is_empty() {
                    content_items.push(widget::text::caption(fl!("btrfs-no-subvolumes")).into());
                    content_items.push(
                        widget::text::caption(fl!("btrfs-no-subvolumes-desc"))
                            .size(11)
                            .into(),
                    );
                } else {
                    // Build hierarchical view
                    let subvol_list = build_subvolume_hierarchy(subvolumes, state);
                    content_items.push(subvol_list);
                }
            }
            Err(error) => {
                content_items.push(widget::text::caption(format!("Error: {}", error)).into());
            }
        }
    }

    // Spacing at end
    content_items.push(widget::space().height(8).into());

    iced_widget::column(content_items).spacing(8).into()
}

/// Build hierarchical subvolume list with snapshots nested under parents
fn build_subvolume_hierarchy<'a>(
    subvolumes: &'a [BtrfsSubvolume],
    state: &'a BtrfsState,
) -> Element<'a, Message> {
    // Group snapshots by their source subvolume UUID
    // snapshots_map: source UUID -> list of snapshots
    let mut snapshots_map: HashMap<String, Vec<&BtrfsSubvolume>> = HashMap::new();

    for subvol in subvolumes {
        // If this subvolume has a parent_uuid, it's a snapshot
        if let Some(parent_uuid) = &subvol.parent_uuid {
            snapshots_map
                .entry(parent_uuid.clone())
                .or_default()
                .push(subvol);
        }
    }

    let mut list = iced_widget::column![].spacing(4);

    // Display all subvolumes that are NOT snapshots (don't have parent_uuid)
    // These are the original subvolumes
    for subvol in subvolumes {
        if subvol.parent_uuid.is_none() {
            // This is an original subvolume (not a snapshot)
            list = list.push(render_subvolume_row(subvol, &snapshots_map, state, 0));
        }
    }

    list.into()
}

/// Render a single subvolume row with optional child snapshots
fn render_subvolume_row<'a>(
    subvol: &'a BtrfsSubvolume,
    snapshots_map: &HashMap<String, Vec<&'a BtrfsSubvolume>>,
    state: &'a BtrfsState,
    indent_level: u16,
) -> Element<'a, Message> {
    let mount_point = state.mount_point.as_ref();
    let snapshots = snapshots_map.get(&subvol.uuid);
    let has_snapshots = snapshots.is_some_and(|s| !s.is_empty());
    let is_expanded = state
        .expanded_subvolumes
        .get(&subvol.id)
        .copied()
        .unwrap_or(false);

    let mut row_items: Vec<Element<'a, Message>> = Vec::new();

    // Indentation
    if indent_level > 0 {
        row_items.push(
            widget::Space::new()
                .width((indent_level * 20) as f32)
                .into(),
        );
    }

    // Expander (if has snapshots)
    if has_snapshots {
        let expander_btn = if let Some(mp) = mount_point {
            widget::button::icon(widget::icon::from_name(expander_icon(is_expanded)).size(16))
                .on_press(Message::BtrfsToggleSubvolumeExpanded {
                    mount_point: mp.clone(),
                    subvolume_id: subvol.id,
                })
                .padding(2)
        } else {
            widget::button::icon(widget::icon::from_name(expander_icon(is_expanded)).size(16))
                .padding(2)
        };
        row_items.push(expander_btn.into());
    } else {
        // Spacer where expander would be
        row_items.push(widget::space().width(20.0).into());
    }

    // Path (normal text size, fills space)
    row_items.push(
        widget::text(&subvol.path)
            .size(13.0)
            .width(cosmic::iced::Length::Fill)
            .into(),
    );

    // ID (caption size, fixed width)
    row_items.push(
        widget::text::caption(format!("{}", subvol.id))
            .width(80)
            .into(),
    );

    // Delete button
    let delete_button = if let (Some(bp), Some(mp)) = (&state.block_path, mount_point) {
        widget::button::icon(widget::icon::from_name("edit-delete-symbolic"))
            .on_press(Message::BtrfsDeleteSubvolume {
                block_path: bp.clone(),
                mount_point: mp.clone(),
                path: subvol.path.clone(),
            })
            .padding(4)
    } else {
        widget::button::icon(widget::icon::from_name("edit-delete-symbolic")).padding(4)
    };
    row_items.push(delete_button.into());

    let row = iced_widget::row(row_items)
        .spacing(8)
        .align_y(cosmic::iced::Alignment::Center);

    let mut col = iced_widget::column![row].spacing(2);

    // If expanded and has snapshots, render them indented
    if is_expanded
        && has_snapshots
        && let Some(snapshot_list) = snapshots
    {
        for snapshot in snapshot_list.iter() {
            col = col.push(render_subvolume_row(
                snapshot,
                snapshots_map,
                state,
                indent_level + 1,
            ));
        }
    }

    col.into()
}

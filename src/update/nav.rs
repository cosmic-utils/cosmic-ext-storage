use crate::message::app::Message;
use crate::models::UiDrive;
use crate::state::app::AppModel;
use crate::state::btrfs::BtrfsState;
use crate::state::dialogs::ShowDialog;
use crate::state::volumes::VolumesControl;

use cosmic::app::Task;
use cosmic::widget::icon;
use std::collections::HashMap;

pub(super) fn update_nav(
    app: &mut AppModel,
    drive_models: Vec<UiDrive>,
    selected: Option<String>,
) -> Task<Message> {
    // Cache drive models for the custom sidebar tree.
    app.sidebar.set_drives(drive_models.clone());

    // Some actions (unlock/format/create/delete) trigger a refresh; close the dialog if
    // it is in a running state so it doesn't linger after success.
    let should_close = match app.dialog.as_ref() {
        Some(ShowDialog::UnlockEncrypted(s)) => s.running,
        Some(ShowDialog::FormatDisk(s)) => s.running,
        Some(ShowDialog::AddPartition(s)) => s.running,
        Some(ShowDialog::FormatPartition(s)) => s.running,
        Some(ShowDialog::EditPartition(s)) => s.running,
        Some(ShowDialog::ResizePartition(s)) => s.running,
        Some(ShowDialog::EditFilesystemLabel(s)) => s.running,
        Some(ShowDialog::EditMountOptions(s)) => s.running,
        Some(ShowDialog::ConfirmAction(s)) => s.running,
        Some(ShowDialog::TakeOwnership(s)) => s.running,
        Some(ShowDialog::ChangePassphrase(s)) => s.running,
        Some(ShowDialog::EditEncryptionOptions(s)) => s.running,
        Some(ShowDialog::DeletePartition(s)) => s.running,
        Some(ShowDialog::BtrfsCreateSubvolume(s)) => s.running,
        Some(ShowDialog::BtrfsCreateSnapshot(s)) => s.running,
        _ => false,
    };

    if should_close {
        app.dialog = None;
    }

    let selected = selected.or_else(|| {
        app.nav
            .active_data::<UiDrive>()
            .map(|d| d.device().to_string())
    });

    // Volumes-level preference; keep it stable across nav rebuilds.
    let show_reserved = app
        .nav
        .active_data::<VolumesControl>()
        .map(|v| v.show_reserved)
        .unwrap_or(app.config.show_reserved);

    app.nav.clear();

    let mut drive_entities: HashMap<String, cosmic::widget::nav_bar::Id> = HashMap::new();

    let selected = selected.or_else(|| drive_models.first().map(|d| d.device().to_string()));

    for drive in &drive_models {
        let icon_name = if drive.disk.removable {
            "drive-removable-media-symbolic"
        } else {
            "disks-symbolic"
        };

        let should_activate = selected.as_ref().is_some_and(|s| drive.device() == s);

        let mut volumes_control =
            VolumesControl::new(drive, show_reserved, app.filesystem_tools.clone());

        // Initialize BTRFS state for the selected segment if it contains BTRFS
        // (checks all segments and looks through LUKS containers)
        let selected_idx = volumes_control.selected_segment;
        if let Some(segment) = volumes_control.segments.get(selected_idx)
            && let Some(volume) = &segment.volume
        {
            let btrfs_info = crate::state::btrfs::detect_btrfs_for_volume(&drive.volumes, volume);
            tracing::info!(
                "update_nav: drive={}, segment={}, btrfs_detected={}, has_filesystem={}, mount_points={:?}",
                drive.name(),
                selected_idx,
                btrfs_info.is_some(),
                volume.has_filesystem,
                volume.mount_points
            );

            if let Some((mount_point, block_path)) = btrfs_info {
                tracing::info!(
                    "update_nav: Initializing BTRFS state with mount_point={:?}, block_path={}",
                    mount_point,
                    block_path
                );
                volumes_control.btrfs_state = Some(BtrfsState::new(mount_point, Some(block_path)));
            }
        }

        let mut nav_item = app
            .nav
            .insert()
            .text(drive.name().clone())
            .data::<VolumesControl>(volumes_control)
            .data::<UiDrive>(drive.clone())
            .icon(icon::from_name(icon_name));

        if should_activate {
            nav_item = nav_item.activate();
        }

        let id = nav_item.id();
        drive_entities.insert(drive.device().to_string(), id);
    }

    app.sidebar.set_drive_entities(drive_entities);

    //  Trigger BTRFS data loading for activated drive
    if let Some(volumes_control) = app.nav.active_data::<VolumesControl>()
        && let Some(btrfs_state) = &volumes_control.btrfs_state
        && let Some(mount_point) = &btrfs_state.mount_point
        && let Some(block_path) = &btrfs_state.block_path
    {
        let mut tasks = Vec::new();

        // Load subvolumes if not already loaded/loading
        if btrfs_state.subvolumes.is_none() && !btrfs_state.loading {
            tasks.push(Task::done(
                Message::BtrfsLoadSubvolumes {
                    block_path: block_path.clone(),
                    mount_point: mount_point.clone(),
                }
                .into(),
            ));
        }

        // Load usage info if not already loaded/loading
        if btrfs_state.used_space.is_none() && !btrfs_state.loading_usage {
            tasks.push(Task::done(
                Message::BtrfsLoadUsage {
                    block_path: block_path.clone(),
                    mount_point: mount_point.clone(),
                }
                .into(),
            ));
        }

        if !tasks.is_empty() {
            return Task::batch(tasks);
        }
    }

    Task::none()
}

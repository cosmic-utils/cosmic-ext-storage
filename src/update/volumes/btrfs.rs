use crate::models::load_all_drives;
use crate::operations::BtrfsClient;
use cosmic::Task;

use crate::app::Message;
use crate::errors::ui::{UiErrorContext, log_error_and_show_dialog};
use crate::fl;
use crate::message::dialogs::{BtrfsCreateSnapshotMessage, BtrfsCreateSubvolumeMessage};
use crate::state::dialogs::{BtrfsCreateSnapshotDialog, BtrfsCreateSubvolumeDialog, ShowDialog};
use crate::state::volumes::VolumesControl;

pub(super) fn open_create_subvolume(
    control: &mut VolumesControl,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    if dialog.is_some() {
        return Task::none();
    }

    // Get mount point and block path from BTRFS state
    let Some(btrfs_state) = &control.btrfs_state else {
        return Task::none();
    };

    let Some(mount_point) = &btrfs_state.mount_point else {
        return Task::none();
    };

    let Some(block_path) = &btrfs_state.block_path else {
        return Task::none();
    };

    *dialog = Some(ShowDialog::BtrfsCreateSubvolume(
        BtrfsCreateSubvolumeDialog {
            mount_point: mount_point.clone(),
            block_path: block_path.clone(),
            name: String::new(),
            running: false,
            error: None,
        },
    ));

    Task::none()
}

pub(super) fn btrfs_create_subvolume_message(
    _control: &mut VolumesControl,
    msg: BtrfsCreateSubvolumeMessage,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    let Some(ShowDialog::BtrfsCreateSubvolume(state)) = dialog.as_mut() else {
        return Task::none();
    };

    match msg {
        BtrfsCreateSubvolumeMessage::NameUpdate(name) => {
            state.name = name;
            state.error = None;
        }
        BtrfsCreateSubvolumeMessage::Cancel => {
            return Task::done(Message::CloseDialog.into());
        }
        BtrfsCreateSubvolumeMessage::Create => {
            if state.running {
                return Task::none();
            }

            // Validate name
            let name = state.name.trim();
            if name.is_empty() {
                state.error = Some(fl!("btrfs-subvolume-name-required"));
                return Task::none();
            }

            if name.contains('/') {
                state.error = Some(fl!("btrfs-subvolume-invalid-chars"));
                return Task::none();
            }

            if name.len() > 255 {
                state.error = Some("Subvolume name too long".to_string());
                return Task::none();
            }

            state.running = true;
            state.error = None;

            let mount_point = state.mount_point.clone();
            let block_path = state.block_path.clone();
            let name = name.to_string();

            return Task::perform(
                async move {
                    let btrfs_client = BtrfsClient::new().await?;
                    btrfs_client.create_subvolume(&mount_point, &name).await?;
                    load_all_drives().await
                },
                move |result| match result {
                    Ok(drives) => {
                        // Close dialog and refresh, preserving BTRFS volume selection
                        Message::UpdateNavWithChildSelection(drives, Some(block_path.clone()))
                            .into()
                    }
                    Err(e) => {
                        let ctx = UiErrorContext::new("create_subvolume");
                        log_error_and_show_dialog(
                            fl!("btrfs-create-subvolume-failed"),
                            e.into(),
                            ctx,
                        )
                        .into()
                    }
                },
            );
        }
    }

    Task::none()
}

pub(super) fn open_create_snapshot(
    control: &mut VolumesControl,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    if dialog.is_some() {
        return Task::none();
    }

    // Get mount point and block path from BTRFS state
    let Some(btrfs_state) = &control.btrfs_state else {
        return Task::none();
    };

    let Some(mount_point) = &btrfs_state.mount_point else {
        return Task::none();
    };

    let Some(block_path) = &btrfs_state.block_path else {
        return Task::none();
    };

    // Get subvolumes list
    let subvolumes: Vec<storage_types::BtrfsSubvolume> = match &btrfs_state.subvolumes {
        Some(Ok(subvols)) if !subvols.is_empty() => subvols.clone(),
        _ => {
            // No subvolumes available
            return Task::none();
        }
    };

    *dialog = Some(ShowDialog::BtrfsCreateSnapshot(BtrfsCreateSnapshotDialog {
        mount_point: mount_point.clone(),
        block_path: block_path.clone(),
        subvolumes,
        selected_source_index: 0,
        snapshot_name: String::new(),
        read_only: true,
        running: false,
        error: None,
    }));

    Task::none()
}

pub(super) fn btrfs_create_snapshot_message(
    _control: &mut VolumesControl,
    msg: BtrfsCreateSnapshotMessage,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    let Some(ShowDialog::BtrfsCreateSnapshot(state)) = dialog.as_mut() else {
        return Task::none();
    };

    match msg {
        BtrfsCreateSnapshotMessage::SourceIndexUpdate(index) => {
            if index < state.subvolumes.len() {
                state.selected_source_index = index;
            }
            state.error = None;
        }
        BtrfsCreateSnapshotMessage::NameUpdate(name) => {
            state.snapshot_name = name;
            state.error = None;
        }
        BtrfsCreateSnapshotMessage::ReadOnlyUpdate(read_only) => {
            state.read_only = read_only;
        }
        BtrfsCreateSnapshotMessage::Cancel => {
            return Task::done(Message::CloseDialog.into());
        }
        BtrfsCreateSnapshotMessage::Create => {
            if state.running {
                return Task::none();
            }

            // Validate snapshot name
            let name = state.snapshot_name.trim();
            if name.is_empty() {
                state.error = Some(fl!("btrfs-subvolume-name-required"));
                return Task::none();
            }

            if name.contains('/') {
                state.error = Some(fl!("btrfs-subvolume-invalid-chars"));
                return Task::none();
            }

            if name.len() > 255 {
                state.error = Some("Snapshot name too long".to_string());
                return Task::none();
            }

            state.running = true;
            state.error = None;

            // Get source subvolume path
            let source_subvol = &state.subvolumes[state.selected_source_index];
            let source = source_subvol.path.clone();
            let dest = name.to_string(); // dest is a string, not std::path::PathBuf
            let read_only = state.read_only;
            let mount_point = state.mount_point.clone();
            let block_path = state.block_path.clone();

            return Task::perform(
                async move {
                    let btrfs_client = BtrfsClient::new().await?;
                    btrfs_client
                        .create_snapshot(&mount_point, &source, &dest, read_only)
                        .await?;
                    load_all_drives().await
                },
                move |result| match result {
                    Ok(drives) => {
                        // Close dialog and refresh, preserving BTRFS volume selection
                        Message::UpdateNavWithChildSelection(drives, Some(block_path.clone()))
                            .into()
                    }
                    Err(e) => {
                        let ctx = UiErrorContext::new("create_snapshot");
                        let msg = log_error_and_show_dialog(
                            fl!("btrfs-create-snapshot-failed"),
                            e.into(),
                            ctx,
                        );
                        msg.into()
                    }
                },
            );
        }
    }

    Task::none()
}

use cosmic::Task;

use crate::app::Message;
use crate::message::volumes::VolumesControlMessage;
use crate::state::dialogs::ShowDialog;
use crate::state::volumes::{DetailTab, VolumesControl};

mod btrfs;
mod create;
mod encryption;
mod filesystem;
pub(crate) mod helpers;
mod mount;
mod mount_options;
mod partition;
mod selection;

impl VolumesControl {
    pub fn update(
        &mut self,
        message: VolumesControlMessage,
        dialog: &mut Option<ShowDialog>,
    ) -> Task<cosmic::Action<Message>> {
        match message {
            VolumesControlMessage::SegmentSelected(index) => {
                selection::segment_selected(self, index, dialog)
            }
            VolumesControlMessage::SelectDetailTab(tab) => {
                self.detail_tab = tab;
                if tab == DetailTab::Usage
                    && self.usage_state.result.is_none()
                    && !self.usage_state.loading
                {
                    return Task::done(cosmic::Action::App(Message::UsageConfigureRequested));
                }

                // If switching to BTRFS tab, ensure data is loaded
                if tab == DetailTab::BtrfsManagement
                    && let Some(btrfs_state) = &self.btrfs_state
                    && let Some(mp) = &btrfs_state.mount_point
                    && let Some(bp) = &btrfs_state.block_path
                {
                    let mut tasks = Vec::new();
                    if btrfs_state.subvolumes.is_none() && !btrfs_state.loading {
                        tasks.push(Task::done(cosmic::Action::App(
                            Message::BtrfsLoadSubvolumes {
                                block_path: bp.clone(),
                                mount_point: mp.clone(),
                            },
                        )));
                    }
                    if btrfs_state.used_space.is_none() && !btrfs_state.loading_usage {
                        tasks.push(Task::done(cosmic::Action::App(Message::BtrfsLoadUsage {
                            block_path: bp.clone(),
                            mount_point: mp.clone(),
                        })));
                    }
                    if !tasks.is_empty() {
                        return Task::batch(tasks);
                    }
                }
                Task::none()
            }
            VolumesControlMessage::SelectVolume {
                segment_index,
                device_path,
            } => selection::select_volume(self, segment_index, device_path, dialog),
            VolumesControlMessage::Mount => mount::mount(self),
            VolumesControlMessage::Unmount => mount::unmount(self),
            VolumesControlMessage::ChildMount(device_path) => mount::child_mount(self, device_path),
            VolumesControlMessage::ChildUnmount(device_path) => {
                mount::child_unmount(self, device_path)
            }

            VolumesControlMessage::LockContainer => encryption::lock_container(self),
            VolumesControlMessage::Delete => partition::delete(self, dialog),
            VolumesControlMessage::OpenFormatPartition => {
                partition::open_format_partition(self, dialog)
            }
            VolumesControlMessage::OpenEditPartition => {
                partition::open_edit_partition(self, dialog)
            }
            VolumesControlMessage::OpenResizePartition => {
                partition::open_resize_partition(self, dialog)
            }
            VolumesControlMessage::OpenEditFilesystemLabel => {
                filesystem::open_edit_filesystem_label(self, dialog)
            }
            VolumesControlMessage::OpenEditMountOptions => {
                mount_options::open_edit_mount_options(self, dialog)
            }
            VolumesControlMessage::OpenCheckFilesystem => {
                filesystem::open_check_filesystem(self, dialog)
            }
            VolumesControlMessage::CheckFilesystemConfirm => {
                filesystem::check_filesystem_confirm(self, dialog)
            }
            VolumesControlMessage::OpenRepairFilesystem => {
                filesystem::open_repair_filesystem(self, dialog)
            }
            VolumesControlMessage::RepairFilesystemConfirm => {
                filesystem::repair_filesystem_confirm(self, dialog)
            }
            VolumesControlMessage::OpenTakeOwnership => {
                encryption::open_take_ownership(self, dialog)
            }
            VolumesControlMessage::OpenChangePassphrase => {
                encryption::open_change_passphrase(self, dialog)
            }
            VolumesControlMessage::OpenEditEncryptionOptions => {
                encryption::open_edit_encryption_options(self, dialog)
            }
            VolumesControlMessage::OpenBtrfsCreateSubvolume => {
                btrfs::open_create_subvolume(self, dialog)
            }
            VolumesControlMessage::OpenBtrfsCreateSnapshot => {
                btrfs::open_create_snapshot(self, dialog)
            }

            VolumesControlMessage::CreateMessage(msg) => create::create_message(self, msg, dialog),
            VolumesControlMessage::UnlockMessage(unlock_message) => {
                encryption::unlock_message(self, unlock_message, dialog)
            }
            VolumesControlMessage::EditPartitionMessage(msg) => {
                partition::edit_partition_message(self, msg, dialog)
            }
            VolumesControlMessage::ResizePartitionMessage(msg) => {
                partition::resize_partition_message(self, msg, dialog)
            }
            VolumesControlMessage::EditFilesystemLabelMessage(msg) => {
                filesystem::edit_filesystem_label_message(self, msg, dialog)
            }
            VolumesControlMessage::EditMountOptionsMessage(msg) => {
                mount_options::edit_mount_options_message(self, msg, dialog)
            }
            VolumesControlMessage::TakeOwnershipMessage(msg) => {
                encryption::take_ownership_message(self, msg, dialog)
            }
            VolumesControlMessage::ChangePassphraseMessage(msg) => {
                encryption::change_passphrase_message(self, msg, dialog)
            }
            VolumesControlMessage::EditEncryptionOptionsMessage(msg) => {
                encryption::edit_encryption_options_message(self, msg, dialog)
            }
            VolumesControlMessage::BtrfsCreateSubvolumeMessage(msg) => {
                btrfs::btrfs_create_subvolume_message(self, msg, dialog)
            }
            VolumesControlMessage::BtrfsCreateSnapshotMessage(msg) => {
                btrfs::btrfs_create_snapshot_message(self, msg, dialog)
            }
        }
    }
}

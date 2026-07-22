use crate::models::{UiDrive, load_all_drives};
use cosmic::Task;

use crate::app::Message;
use crate::errors::ui::{UiErrorContext, log_error_and_show_dialog};
use crate::fl;
use crate::message::dialogs::CreateMessage;
use crate::operations::{FilesystemsClient, PartitionsClient};
use crate::state::dialogs::{CreatePartitionStep, FormatPartitionStep, ShowDialog};
use storage_types::FormatOptions;

use storage_types::CreatePartitionInfo;

use crate::state::volumes::VolumesControl;

fn create_partition_step_can_advance(state: &crate::state::dialogs::CreatePartitionDialog) -> bool {
    match state.step {
        CreatePartitionStep::Basics => {
            let filesystem_type = crate::utils::partition_types::common_partition_filesystem_type(
                &state.info.table_type,
                state.info.selected_partition_type_index,
            );

            filesystem_type.is_some_and(|fs_type| {
                state
                    .filesystem_tools
                    .iter()
                    .any(|tool| tool.fs_type == fs_type && tool.available)
            })
        }
        CreatePartitionStep::Sizing => {
            state.info.size > 0 && state.info.size <= state.info.max_size
        }
        CreatePartitionStep::Options => true,
    }
}

pub(super) fn create_message(
    control: &mut VolumesControl,
    create_message: CreateMessage,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    let d = match dialog.as_mut() {
        Some(d) => d,
        None => {
            tracing::warn!("create message received with no active dialog; ignoring");
            return Task::none();
        }
    };

    match d {
        ShowDialog::DeletePartition(_) => {}

        ShowDialog::EditMountOptions(_) | ShowDialog::EditEncryptionOptions(_) => {}

        ShowDialog::AddPartition(state) => match create_message {
            CreateMessage::PrevStep => {
                if state.running {
                    return Task::none();
                }
                state.step = match state.step {
                    CreatePartitionStep::Basics => CreatePartitionStep::Basics,
                    CreatePartitionStep::Sizing => CreatePartitionStep::Basics,
                    CreatePartitionStep::Options => CreatePartitionStep::Sizing,
                };
            }
            CreateMessage::NextStep => {
                if state.running {
                    return Task::none();
                }
                if !create_partition_step_can_advance(state) {
                    return Task::none();
                }
                state.step = match state.step {
                    CreatePartitionStep::Basics => CreatePartitionStep::Sizing,
                    CreatePartitionStep::Sizing => CreatePartitionStep::Options,
                    CreatePartitionStep::Options => CreatePartitionStep::Options,
                };
            }
            CreateMessage::SetStep(step) => {
                if state.running {
                    return Task::none();
                }

                if step.number() <= state.step.number() {
                    state.step = step;
                    state.error = None;
                }
            }
            CreateMessage::SetFormatStep(_) => {}
            CreateMessage::SizeUpdate(size) => {
                state.info.size = size;
                state.error = None;
            }
            CreateMessage::SizeUnitUpdate(unit_index) => {
                state.info.size_unit_index = unit_index;
                state.error = None;
            }
            CreateMessage::NameUpdate(name) => {
                state.info.name = name;
                state.error = None;
            }
            CreateMessage::PasswordUpdate(password) => {
                state.info.password = password;
                state.error = None;
            }
            CreateMessage::ConfirmedPasswordUpdate(confirmed_password) => {
                state.info.confirmed_password = confirmed_password;
                state.error = None;
            }
            CreateMessage::PasswordProtectedUpdate(protect) => {
                state.info.password_protected = protect;
                state.error = None;
            }
            CreateMessage::EraseUpdate(erase) => {
                state.info.erase = erase;
                state.error = None;
            }
            CreateMessage::PartitionTypeUpdate(p_type) => {
                state.info.selected_partition_type_index = p_type;
                state.error = None;
            }
            CreateMessage::Cancel => return Task::done(Message::CloseDialog.into()),
            CreateMessage::Partition => {
                if state.running {
                    return Task::none();
                }

                // UI-side validation for encrypted partition creation.
                if state.info.password_protected {
                    if state.info.password.is_empty() {
                        tracing::warn!(operation = "create_partition", "password required");
                        state.error = Some(fl!("password-required").to_string());
                        return Task::none();
                    }
                    if state.info.password != state.info.confirmed_password {
                        tracing::warn!(operation = "create_partition", "password mismatch");
                        state.error = Some(fl!("password-mismatch").to_string());
                        return Task::none();
                    }
                }

                state.running = true;
                state.error = None;

                let mut create_partition_info: CreatePartitionInfo = state.info.clone();
                if create_partition_info.name.is_empty() {
                    create_partition_info.name = fl!("untitled").to_string();
                }

                // Populate filesystem_type from selected partition type index
                if create_partition_info.filesystem_type.is_empty() {
                    create_partition_info.filesystem_type =
                        crate::utils::partition_types::common_partition_filesystem_type(
                            &create_partition_info.table_type,
                            create_partition_info.selected_partition_type_index,
                        )
                        .unwrap_or_default();
                }

                let device = control.device.clone();
                return Task::perform(
                    async move {
                        let partitions_client = PartitionsClient::new().await.map_err(|e| {
                            anyhow::anyhow!("Failed to create partitions client: {}", e)
                        })?;
                        partitions_client
                            .create_partition_with_filesystem(&device, &create_partition_info)
                            .await
                            .map_err(|e| anyhow::anyhow!("Failed to create partition: {}", e))?;
                        load_all_drives().await.map_err(|e| e.into())
                    },
                    |result: Result<Vec<UiDrive>, anyhow::Error>| match result {
                        Ok(drives) => Message::UpdateNav(drives, None).into(),
                        Err(e) => {
                            let ctx = UiErrorContext::new("create_partition");
                            log_error_and_show_dialog(fl!("create-partition-failed"), e, ctx).into()
                        }
                    },
                );
            }
        },

        ShowDialog::FormatPartition(state) => match create_message {
            CreateMessage::PrevStep => {
                if state.running {
                    return Task::none();
                }

                state.step = match state.step {
                    FormatPartitionStep::Basics => FormatPartitionStep::Basics,
                    FormatPartitionStep::Options => FormatPartitionStep::Basics,
                };
            }
            CreateMessage::NextStep => {
                if state.running {
                    return Task::none();
                }

                state.step = match state.step {
                    FormatPartitionStep::Basics => FormatPartitionStep::Options,
                    FormatPartitionStep::Options => FormatPartitionStep::Options,
                };
            }
            CreateMessage::SetFormatStep(step) => {
                if state.running {
                    return Task::none();
                }

                if step.number() <= state.step.number() {
                    state.step = step;
                }
            }
            CreateMessage::NameUpdate(name) => {
                state.info.name = name;
            }
            CreateMessage::EraseUpdate(erase) => state.info.erase = erase,
            CreateMessage::PartitionTypeUpdate(p_type) => {
                state.info.selected_partition_type_index = p_type
            }
            CreateMessage::Cancel => return Task::done(Message::CloseDialog.into()),
            CreateMessage::Partition => {
                if state.running {
                    return Task::none();
                }
                state.running = true;

                let volume = state.volume.clone();
                let info = state.info.clone();
                return Task::perform(
                    async move {
                        let fs_type =
                            crate::utils::partition_types::common_partition_filesystem_type(
                                info.table_type.as_str(),
                                info.selected_partition_type_index,
                            )
                            .ok_or_else(|| anyhow::anyhow!("Invalid filesystem selection"))?;

                        let filesystems_client = FilesystemsClient::new().await.map_err(|e| {
                            anyhow::anyhow!("Failed to create filesystems client: {}", e)
                        })?;
                        let options = FormatOptions {
                            erase: info.erase,
                            ..FormatOptions::default()
                        };
                        let device = volume
                            .device_path
                            .as_ref()
                            .ok_or_else(|| anyhow::anyhow!("Volume has no device path"))?;
                        filesystems_client
                            .format(device, &fs_type, &info.name, options)
                            .await
                            .map_err(|e| anyhow::anyhow!("Failed to format: {}", e))?;
                        load_all_drives().await.map_err(|e| e.into())
                    },
                    |result: Result<Vec<UiDrive>, anyhow::Error>| match result {
                        Ok(drives) => Message::UpdateNav(drives, None).into(),
                        Err(e) => {
                            let ctx = UiErrorContext::new("format_partition");
                            log_error_and_show_dialog(fl!("format-partition").to_string(), e, ctx)
                                .into()
                        }
                    },
                );
            }
            _ => {}
        },

        ShowDialog::UnlockEncrypted(_) => {
            tracing::warn!("create message received while an unlock dialog is open; ignoring");
        }

        ShowDialog::FormatDisk(_) => {
            tracing::warn!("create message received while a format disk dialog is open; ignoring");
        }

        ShowDialog::SmartData(_) => {
            tracing::warn!("create message received while a SMART dialog is open; ignoring");
        }

        ShowDialog::NewDiskImage(_)
        | ShowDialog::AttachDiskImage(_)
        | ShowDialog::ImageOperation(_) => {
            tracing::warn!("create message received while an image dialog is open; ignoring");
        }

        ShowDialog::EditPartition(_)
        | ShowDialog::ResizePartition(_)
        | ShowDialog::EditFilesystemLabel(_)
        | ShowDialog::ConfirmAction(_)
        | ShowDialog::TakeOwnership(_)
        | ShowDialog::ChangePassphrase(_)
        | ShowDialog::UnmountBusy(_)
        | ShowDialog::BtrfsCreateSubvolume(_)
        | ShowDialog::BtrfsCreateSnapshot(_) => {
            tracing::warn!("create message received while a different dialog is open; ignoring");
        }

        ShowDialog::Info { .. } => {
            tracing::warn!("create message received while an info dialog is open; ignoring");
        }

        ShowDialog::ConfirmDeleteRemote { .. } => {
            tracing::warn!(
                "create message received while a delete confirmation dialog is open; ignoring"
            );
        }
    }

    // Preserve behavior: no fallthrough action here.
    Task::none()
}

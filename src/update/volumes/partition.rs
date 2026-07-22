use crate::models::UiDrive;
use crate::models::load_all_drives;
use cosmic::Task;

use crate::app::Message;
use crate::errors::ui::{UiErrorContext, log_error_and_show_dialog};
use crate::fl;
use crate::message::dialogs::{EditPartitionMessage, ResizePartitionMessage};
use crate::operations::{FilesystemsClient, LuksClient, PartitionsClient};
use crate::state::dialogs::{
    EditPartitionDialog, EditPartitionStep, FormatPartitionDialog, FormatPartitionStep,
    ResizePartitionDialog, ResizePartitionStep, ShowDialog,
};
use crate::utils::DiskSegmentKind;

use storage_types::{CreatePartitionInfo, VolumeKind};

use crate::state::volumes::VolumesControl;

pub(super) fn delete(
    control: &mut VolumesControl,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    let d = match dialog.as_mut() {
        Some(d) => d,
        None => {
            tracing::warn!("delete received with no active dialog; ignoring");
            return Task::none();
        }
    };

    let ShowDialog::DeletePartition(delete_state) = d else {
        tracing::warn!("delete received while a different dialog is open; ignoring");
        return Task::none();
    };

    if delete_state.running {
        return Task::none();
    }

    delete_state.running = true;

    let Some(segment) = control.segments.get(control.selected_segment).cloned() else {
        return Task::none();
    };

    let Some(p) = segment.volume else {
        return Task::none();
    };

    let volume_node =
        crate::state::volumes::find_volume_for_partition(&control.volumes, &p).cloned();
    let is_unlocked_crypto = matches!(
        volume_node.as_ref(),
        Some(v) if v.volume.kind == VolumeKind::CryptoContainer && !v.volume.locked
    );
    let mounted_children: Vec<String> = if is_unlocked_crypto {
        volume_node
            .as_ref()
            .map(crate::update::volumes::helpers::collect_mounted_descendants_leaf_first)
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    Task::perform(
        async move {
            if is_unlocked_crypto {
                let fs_client = FilesystemsClient::new()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to create filesystems client: {}", e))?;
                let luks_client = LuksClient::new()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to create LUKS client: {}", e))?;

                for v in mounted_children {
                    let device = &v;
                    fs_client
                        .unmount(device, false, false)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to unmount {}: {}", device, e))?;
                }

                let cleartext_device = p
                    .device_path
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Partition has no device path"))?;
                luks_client
                    .lock(cleartext_device)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to lock LUKS device: {}", e))?;
            }

            let partitions_client = PartitionsClient::new()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create partitions client: {}", e))?;
            let device = p
                .device_path
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Partition has no device path"))?;
            partitions_client
                .delete_partition(device)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to delete partition: {}", e))?;

            load_all_drives().await.map_err(|e| e.into())
        },
        |result: Result<Vec<UiDrive>, anyhow::Error>| match result {
            Ok(drives) => Message::UpdateNav(drives, None).into(),
            Err(e) => {
                tracing::error!(?e, "delete failed");
                Message::Dialog(Box::new(ShowDialog::Info {
                    title: fl!("delete-failed"),
                    body: format!("{e:#}"),
                }))
                .into()
            }
        },
    )
}

pub(super) fn open_format_partition(
    control: &mut VolumesControl,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    if dialog.is_some() {
        return Task::none();
    }

    let Some(segment) = control.segments.get(control.selected_segment) else {
        return Task::none();
    };
    let Some(volume) = segment.volume.clone() else {
        return Task::none();
    };

    let table_type = if segment.table_type.trim().is_empty() {
        "gpt".to_string()
    } else {
        segment.table_type.clone()
    };

    let selected_partition_type_index =
        crate::utils::partition_types::common_partition_type_index_for(
            &table_type,
            if volume.id_type.trim().is_empty() {
                None
            } else {
                Some(volume.id_type.as_str())
            },
        );

    let info = CreatePartitionInfo {
        name: volume.label.clone(),
        size: segment.size,
        max_size: segment.size,
        offset: segment.offset,
        erase: false,
        selected_partition_type_index,
        table_type,
        ..Default::default()
    };

    *dialog = Some(ShowDialog::FormatPartition(FormatPartitionDialog {
        volume,
        info,
        step: FormatPartitionStep::Basics,
        running: false,
        filesystem_tools: control.filesystem_tools.clone(),
    }));

    Task::none()
}

pub(super) fn open_edit_partition(
    control: &mut VolumesControl,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    if dialog.is_some() {
        return Task::none();
    }

    let Some(segment) = control.segments.get(control.selected_segment) else {
        return Task::none();
    };
    let Some(volume) = segment.volume.clone() else {
        return Task::none();
    };

    if volume.kind != storage_types::VolumeKind::Partition {
        return Task::none();
    }

    let partition_types = storage_types::get_all_partition_type_infos(segment.table_type.as_str());
    if partition_types.is_empty() {
        return Task::done(
            Message::Dialog(Box::new(ShowDialog::Info {
                title: fl!("app-title"),
                body: fl!("edit-partition-no-types"),
            }))
            .into(),
        );
    }

    // Find corresponding PartitionInfo to get partition-specific data
    let partition_info = control
        .partitions
        .iter()
        .find(|p| Some(&p.device) == volume.device_path.as_ref());

    let selected_type_index = partition_info
        .and_then(|p| partition_types.iter().position(|t| t.ty == p.type_id))
        .unwrap_or(0);

    let legacy_bios_bootable = partition_info
        .map(|p| p.is_legacy_bios_bootable())
        .unwrap_or(false);
    let system_partition = partition_info
        .map(|p| p.is_system_partition())
        .unwrap_or(false);
    let hidden = partition_info.map(|p| p.is_hidden()).unwrap_or(false);
    let name = volume.label.clone();

    *dialog = Some(ShowDialog::EditPartition(EditPartitionDialog {
        volume,
        step: EditPartitionStep::Basics,
        partition_types,
        selected_type_index,
        name,
        legacy_bios_bootable,
        system_partition,
        hidden,
        running: false,
    }));

    Task::none()
}

pub(super) fn open_resize_partition(
    control: &mut VolumesControl,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    if dialog.is_some() {
        return Task::none();
    }

    let Some(segment) = control.segments.get(control.selected_segment) else {
        return Task::none();
    };
    let Some(volume) = segment.volume.clone() else {
        return Task::none();
    };

    if volume.kind != storage_types::VolumeKind::Partition {
        return Task::none();
    }

    let right_free_bytes = control
        .segments
        .get(control.selected_segment.saturating_add(1))
        .filter(|s| s.kind == DiskSegmentKind::FreeSpace)
        .map(|s| s.size)
        .unwrap_or(0);

    let max_size_bytes = volume.size.saturating_add(right_free_bytes);
    let min_size_bytes = volume
        .usage
        .as_ref()
        .map(|u| u.used)
        .unwrap_or(0)
        .min(max_size_bytes);

    if max_size_bytes.saturating_sub(min_size_bytes) < 1024 {
        return Task::none();
    }

    let new_size_bytes = volume.size.clamp(min_size_bytes, max_size_bytes);

    *dialog = Some(ShowDialog::ResizePartition(ResizePartitionDialog {
        volume,
        step: ResizePartitionStep::Sizing,
        min_size_bytes,
        max_size_bytes,
        new_size_bytes,
        running: false,
    }));

    Task::none()
}

pub(super) fn edit_partition_message(
    _control: &mut VolumesControl,
    msg: EditPartitionMessage,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    let Some(ShowDialog::EditPartition(state)) = dialog.as_mut() else {
        return Task::none();
    };

    match msg {
        EditPartitionMessage::PrevStep => {
            if state.running {
                return Task::none();
            }
            state.step = match state.step {
                EditPartitionStep::Basics => EditPartitionStep::Basics,
                EditPartitionStep::Flags => EditPartitionStep::Basics,
                EditPartitionStep::Review => EditPartitionStep::Flags,
            };
        }
        EditPartitionMessage::NextStep => {
            if state.running {
                return Task::none();
            }
            state.step = match state.step {
                EditPartitionStep::Basics => EditPartitionStep::Flags,
                EditPartitionStep::Flags => EditPartitionStep::Review,
                EditPartitionStep::Review => EditPartitionStep::Review,
            };
        }
        EditPartitionMessage::SetStep(step) => {
            if state.running {
                return Task::none();
            }
            if step.number() <= state.step.number() {
                state.step = step;
            }
        }
        EditPartitionMessage::TypeUpdate(idx) => state.selected_type_index = idx,
        EditPartitionMessage::NameUpdate(name) => state.name = name,
        EditPartitionMessage::LegacyBiosBootableUpdate(v) => state.legacy_bios_bootable = v,
        EditPartitionMessage::SystemPartitionUpdate(v) => state.system_partition = v,
        EditPartitionMessage::HiddenUpdate(v) => state.hidden = v,
        EditPartitionMessage::Cancel => return Task::done(Message::CloseDialog.into()),
        EditPartitionMessage::Confirm => {
            if state.running {
                return Task::none();
            }

            let partition_type = state
                .partition_types
                .get(state.selected_type_index)
                .map(|t| t.ty.to_string());

            let Some(partition_type) = partition_type else {
                return Task::none();
            };

            state.running = true;

            let volume = state.volume.clone();
            let name = state.name.clone();
            let legacy = state.legacy_bios_bootable;
            let system = state.system_partition;
            let hidden = state.hidden;

            return Task::perform(
                async move {
                    let flags = storage_types::make_partition_flags_bits(legacy, system, hidden);

                    let partitions_client = PartitionsClient::new().await.map_err(|e| {
                        anyhow::anyhow!("Failed to create partitions client: {}", e)
                    })?;
                    let device = volume
                        .device_path
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("Partition has no device path"))?;

                    partitions_client
                        .set_partition_type(device, &partition_type)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to set partition type: {}", e))?;
                    partitions_client
                        .set_partition_name(device, &name)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to set partition name: {}", e))?;
                    partitions_client
                        .set_partition_flags(device, flags)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to set partition flags: {}", e))?;

                    load_all_drives().await.map_err(|e| e.into())
                },
                |result: Result<Vec<UiDrive>, anyhow::Error>| match result {
                    Ok(drives) => Message::UpdateNav(drives, None).into(),
                    Err(e) => {
                        let ctx = UiErrorContext::new("edit_partition");
                        log_error_and_show_dialog(fl!("edit-partition").to_string(), e, ctx).into()
                    }
                },
            );
        }
    }

    Task::none()
}

pub(super) fn resize_partition_message(
    _control: &mut VolumesControl,
    msg: ResizePartitionMessage,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    let Some(ShowDialog::ResizePartition(state)) = dialog.as_mut() else {
        return Task::none();
    };

    match msg {
        ResizePartitionMessage::PrevStep => {
            if state.running {
                return Task::none();
            }
            state.step = match state.step {
                ResizePartitionStep::Sizing => ResizePartitionStep::Sizing,
                ResizePartitionStep::Review => ResizePartitionStep::Sizing,
            };
        }
        ResizePartitionMessage::NextStep => {
            if state.running {
                return Task::none();
            }
            state.step = match state.step {
                ResizePartitionStep::Sizing => ResizePartitionStep::Review,
                ResizePartitionStep::Review => ResizePartitionStep::Review,
            };
        }
        ResizePartitionMessage::SetStep(step) => {
            if state.running {
                return Task::none();
            }
            if step.number() <= state.step.number() {
                state.step = step;
            }
        }
        ResizePartitionMessage::SizeUpdate(size) => {
            state.new_size_bytes = size.clamp(state.min_size_bytes, state.max_size_bytes)
        }
        ResizePartitionMessage::Cancel => {
            return Task::done(Message::CloseDialog.into());
        }
        ResizePartitionMessage::Confirm => {
            if state.running {
                return Task::none();
            }

            // Disable when range is too small.
            if state.max_size_bytes.saturating_sub(state.min_size_bytes) < 1024 {
                return Task::none();
            }

            state.running = true;
            let volume = state.volume.clone();
            let new_size = state.new_size_bytes;

            return Task::perform(
                async move {
                    let partitions_client = PartitionsClient::new().await.map_err(|e| {
                        anyhow::anyhow!("Failed to create partitions client: {}", e)
                    })?;
                    let device = volume
                        .device_path
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("Volume has no device path"))?;
                    partitions_client
                        .resize_partition(device, new_size)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to resize partition: {}", e))?;
                    load_all_drives().await.map_err(|e| e.into())
                },
                |result: Result<Vec<UiDrive>, anyhow::Error>| match result {
                    Ok(drives) => Message::UpdateNav(drives, None).into(),
                    Err(e) => {
                        let ctx = UiErrorContext::new("resize_partition");
                        log_error_and_show_dialog(fl!("resize-partition").to_string(), e, ctx)
                            .into()
                    }
                },
            );
        }
    }

    Task::none()
}

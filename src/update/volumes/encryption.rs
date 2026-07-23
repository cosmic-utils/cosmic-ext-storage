use crate::models::UiDrive;
use crate::models::load_all_drives;
use cosmic::Task;

use crate::app::Message;
use crate::errors::ui::{UiErrorContext, log_error_and_show_dialog};
use crate::fl;
use crate::message::dialogs::{
    ChangePassphraseMessage, EditEncryptionOptionsMessage, TakeOwnershipMessage, UnlockMessage,
};
use crate::operations::{FilesystemsClient, LuksClient};
use crate::state::dialogs::{
    ChangePassphraseDialog, EditEncryptionOptionsDialog, EditEncryptionOptionsStep,
    FilesystemTarget, ShowDialog, TakeOwnershipDialog, UnlockEncryptedDialog,
};

use storage_types::VolumeKind;

use crate::state::volumes::VolumesControl;

pub(super) fn open_take_ownership(
    control: &mut VolumesControl,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    if dialog.is_some() {
        return Task::none();
    }

    let target = if let Some(node) = control.selected_volume_node() {
        if !node.volume.has_filesystem {
            return Task::none();
        }
        FilesystemTarget::Node(node.clone())
    } else {
        let Some(segment) = control.segments.get(control.selected_segment) else {
            return Task::none();
        };
        let Some(volume) = segment.volume.clone() else {
            return Task::none();
        };

        if !volume.has_filesystem {
            return Task::none();
        }

        FilesystemTarget::Volume(volume)
    };

    *dialog = Some(ShowDialog::TakeOwnership(TakeOwnershipDialog {
        target,
        recursive: true,
        running: false,
    }));

    Task::none()
}

pub(super) fn open_change_passphrase(
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

    let is_crypto_container =
        crate::state::volumes::find_volume_for_partition(&control.volumes, &volume)
            .is_some_and(|n| n.volume.kind == VolumeKind::CryptoContainer);
    if !is_crypto_container {
        return Task::none();
    }

    *dialog = Some(ShowDialog::ChangePassphrase(ChangePassphraseDialog {
        volume,
        current_passphrase: String::new(),
        new_passphrase: String::new(),
        confirm_passphrase: String::new(),
        error: None,
        running: false,
    }));

    Task::none()
}

pub(super) fn open_edit_encryption_options(
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

    let is_crypto_container =
        crate::state::volumes::find_volume_for_partition(&control.volumes, &volume)
            .is_some_and(|n| n.volume.kind == VolumeKind::CryptoContainer);
    if !is_crypto_container {
        return Task::none();
    }

    let suggested_name = if volume.label.trim().is_empty() {
        volume
            .device_path
            .as_deref()
            .and_then(|p| p.split('/').next_back())
            .unwrap_or("luks")
            .to_string()
    } else {
        volume.label.clone()
    };

    let device_path = volume.device_path.clone();
    Task::perform(
        async move {
            let settings: Option<storage_types::EncryptionOptionsSettings> =
                if let Some(ref device) = device_path {
                    match LuksClient::new().await {
                        Ok(client) => client.get_encryption_options(device).await.ok().flatten(),
                        Err(_) => None,
                    }
                } else {
                    None
                };
            let error: Option<String> = None;

            let (use_defaults, unlock_at_startup, require_auth, other_options, name) =
                if let Some(s) = settings {
                    (
                        false,
                        s.unlock_at_startup,
                        s.require_auth,
                        s.other_options,
                        if s.name.is_empty() {
                            suggested_name.clone()
                        } else {
                            s.name
                        },
                    )
                } else {
                    (
                        true,
                        true,
                        false,
                        "nofail".to_string(),
                        suggested_name.clone(),
                    )
                };

            ShowDialog::EditEncryptionOptions(EditEncryptionOptionsDialog {
                volume,
                step: EditEncryptionOptionsStep::Behavior,
                use_defaults,
                unlock_at_startup,
                require_auth,
                other_options,
                name,
                // Never prefill passphrase.
                passphrase: String::new(),
                show_passphrase: false,
                error,
                running: false,
            })
        },
        |dialog_state| Message::Dialog(Box::new(dialog_state)).into(),
    )
}

pub(super) fn unlock_message(
    control: &mut VolumesControl,
    unlock_message: UnlockMessage,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    let d = match dialog.as_mut() {
        Some(d) => d,
        None => {
            tracing::warn!("unlock message received with no active dialog; ignoring");
            return Task::none();
        }
    };

    let ShowDialog::UnlockEncrypted(state) = d else {
        tracing::warn!("unlock message received while a different dialog is open; ignoring");
        return Task::none();
    };

    match unlock_message {
        UnlockMessage::PassphraseUpdate(p) => {
            state.passphrase = p;
            state.error = None;
            Task::none()
        }
        UnlockMessage::Cancel => Task::done(Message::CloseDialog.into()),
        UnlockMessage::Confirm => {
            if state.running {
                return Task::none();
            }

            state.running = true;

            let partition_path = state.partition_path.clone();
            let partition_name = state.partition_name.clone();
            let passphrase = state.passphrase.clone();
            let passphrase_for_task = passphrase.clone();

            // Look up the partition by device path
            let part = control
                .partitions
                .iter()
                .find(|p| p.device == partition_path)
                .cloned();

            let Some(p) = part else {
                tracing::error!(
                    operation = "unlock_encrypted",
                    device = %partition_path,
                    partition_name = %partition_name,
                    "unlock missing partition in model"
                );
                return Task::done(
                    Message::Dialog(Box::new(ShowDialog::Info {
                        title: fl!("unlock-failed"),
                        body: fl!("unlock-missing-partition", name = partition_name),
                    }))
                    .into(),
                );
            };

            let device_path_for_selection = partition_path.clone();
            Task::perform(
                async move {
                    let luks_client = LuksClient::new()
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to create LUKS client: {}", e))?;
                    let device = &p.device;
                    luks_client
                        .unlock(device, &passphrase_for_task)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to unlock: {}", e))?;
                    load_all_drives().await.map_err(|e| e.into())
                },
                move |result: Result<Vec<UiDrive>, anyhow::Error>| match result {
                    Ok(drives) => {
                        // After unlock, select the unlocked volume (which may have new child nodes)
                        Message::UpdateNavWithChildSelection(
                            drives,
                            Some(device_path_for_selection.clone()),
                        )
                        .into()
                    }
                    Err(e) => {
                        tracing::error!(
                            ?e,
                            operation = "unlock_encrypted",
                            device_path = %partition_path,
                            "unlock encrypted dialog error"
                        );
                        Message::Dialog(Box::new(ShowDialog::UnlockEncrypted(
                            UnlockEncryptedDialog {
                                partition_path: partition_path.clone(),
                                partition_name: partition_name.clone(),
                                passphrase: passphrase.clone(),
                                error: Some(e.to_string()),
                                running: false,
                            },
                        )))
                        .into()
                    }
                },
            )
        }
    }
}

pub(super) fn take_ownership_message(
    _control: &mut VolumesControl,
    msg: TakeOwnershipMessage,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    let Some(ShowDialog::TakeOwnership(state)) = dialog.as_mut() else {
        return Task::none();
    };

    match msg {
        TakeOwnershipMessage::RecursiveUpdate(v) => {
            state.recursive = v;
            Task::none()
        }
        TakeOwnershipMessage::Cancel => Task::done(Message::CloseDialog.into()),
        TakeOwnershipMessage::Confirm => {
            if state.running {
                return Task::none();
            }

            state.running = true;
            let device_path = match &state.target {
                FilesystemTarget::Volume(v) => v
                    .device_path
                    .clone()
                    .unwrap_or_else(|| "/dev/unknown".to_string()),
                FilesystemTarget::Node(n) => n
                    .volume
                    .device_path
                    .clone()
                    .unwrap_or_else(|| "/dev/unknown".to_string()),
            };
            let recursive = state.recursive;

            Task::perform(
                async move {
                    let client = FilesystemsClient::new().await.map_err(|e| {
                        anyhow::anyhow!("Failed to create filesystems client: {}", e)
                    })?;
                    client
                        .take_ownership(&device_path, recursive)
                        .await
                        .map_err(|e| anyhow::anyhow!("Take ownership failed: {}", e))?;
                    load_all_drives().await.map_err(|e| e.into())
                },
                |result: Result<Vec<UiDrive>, anyhow::Error>| match result {
                    Ok(drives) => Message::UpdateNav(drives, None).into(),
                    Err(e) => {
                        let ctx = UiErrorContext::new("take_ownership");
                        log_error_and_show_dialog(fl!("take-ownership").to_string(), e, ctx).into()
                    }
                },
            )
        }
    }
}

pub(super) fn change_passphrase_message(
    _control: &mut VolumesControl,
    msg: ChangePassphraseMessage,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    let Some(ShowDialog::ChangePassphrase(state)) = dialog.as_mut() else {
        return Task::none();
    };

    match msg {
        ChangePassphraseMessage::CurrentUpdate(v) => {
            state.current_passphrase = v;
            state.error = None;
            Task::none()
        }
        ChangePassphraseMessage::NewUpdate(v) => {
            state.new_passphrase = v;
            state.error = None;
            Task::none()
        }
        ChangePassphraseMessage::ConfirmUpdate(v) => {
            state.confirm_passphrase = v;
            state.error = None;
            Task::none()
        }
        ChangePassphraseMessage::Cancel => Task::done(Message::CloseDialog.into()),
        ChangePassphraseMessage::Confirm => {
            if state.running {
                return Task::none();
            }

            if state.new_passphrase.is_empty() || state.new_passphrase != state.confirm_passphrase {
                tracing::warn!(operation = "change_passphrase", "passphrase mismatch");
                state.error = Some(fl!("passphrase-mismatch").to_string());
                return Task::none();
            }

            state.running = true;
            let volume = state.volume.clone();
            let current = state.current_passphrase.clone();
            let new = state.new_passphrase.clone();

            Task::perform(
                async move {
                    let luks_client = LuksClient::new()
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to create LUKS client: {}", e))?;
                    let device = volume
                        .device_path
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("Volume has no device path"))?;
                    luks_client
                        .change_passphrase(device, &current, &new)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to change passphrase: {}", e))?;
                    load_all_drives().await.map_err(|e| e.into())
                },
                |result: Result<Vec<UiDrive>, anyhow::Error>| match result {
                    Ok(drives) => Message::UpdateNav(drives, None).into(),
                    Err(e) => {
                        let ctx = UiErrorContext::new("change_passphrase");
                        log_error_and_show_dialog(fl!("change-passphrase").to_string(), e, ctx)
                            .into()
                    }
                },
            )
        }
    }
}

pub(super) fn edit_encryption_options_message(
    _control: &mut VolumesControl,
    msg: EditEncryptionOptionsMessage,
    dialog: &mut Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    let Some(ShowDialog::EditEncryptionOptions(state)) = dialog.as_mut() else {
        return Task::none();
    };

    match msg {
        EditEncryptionOptionsMessage::PrevStep => {
            if state.running {
                return Task::none();
            }
            state.step = match state.step {
                EditEncryptionOptionsStep::Behavior => EditEncryptionOptionsStep::Behavior,
                EditEncryptionOptionsStep::Credentials => EditEncryptionOptionsStep::Behavior,
                EditEncryptionOptionsStep::Review => EditEncryptionOptionsStep::Credentials,
            };
            state.error = None;
            Task::none()
        }
        EditEncryptionOptionsMessage::NextStep => {
            if state.running {
                return Task::none();
            }
            state.step = match state.step {
                EditEncryptionOptionsStep::Behavior => EditEncryptionOptionsStep::Credentials,
                EditEncryptionOptionsStep::Credentials => EditEncryptionOptionsStep::Review,
                EditEncryptionOptionsStep::Review => EditEncryptionOptionsStep::Review,
            };
            state.error = None;
            Task::none()
        }
        EditEncryptionOptionsMessage::SetStep(step) => {
            if state.running {
                return Task::none();
            }
            if step.number() <= state.step.number() {
                state.step = step;
                state.error = None;
            }
            Task::none()
        }
        EditEncryptionOptionsMessage::UseDefaultsUpdate(v) => {
            state.use_defaults = v;
            state.error = None;
            Task::none()
        }
        EditEncryptionOptionsMessage::UnlockAtStartupUpdate(v) => {
            state.unlock_at_startup = v;
            state.error = None;
            Task::none()
        }
        EditEncryptionOptionsMessage::RequireAuthUpdate(v) => {
            state.require_auth = v;
            state.error = None;
            Task::none()
        }
        EditEncryptionOptionsMessage::OtherOptionsUpdate(v) => {
            state.other_options = v;
            state.error = None;
            Task::none()
        }
        EditEncryptionOptionsMessage::NameUpdate(v) => {
            state.name = v;
            state.error = None;
            Task::none()
        }
        EditEncryptionOptionsMessage::PassphraseUpdate(v) => {
            state.passphrase = v;
            state.error = None;
            Task::none()
        }
        EditEncryptionOptionsMessage::ShowPassphraseUpdate(v) => {
            state.show_passphrase = v;
            Task::none()
        }
        EditEncryptionOptionsMessage::Cancel => Task::done(Message::CloseDialog.into()),
        EditEncryptionOptionsMessage::Confirm => {
            if state.running {
                return Task::none();
            }
            state.running = true;
            let device_path = state
                .volume
                .device_path
                .clone()
                .unwrap_or_else(|| "/dev/unknown".to_string());
            let use_defaults = state.use_defaults;
            let unlock_at_startup = state.unlock_at_startup;
            let require_auth = state.require_auth;
            let other_options = state.other_options.clone();
            let name = state.name.clone();
            let passphrase = state.passphrase.clone();

            Task::perform(
                async move {
                    let luks_client = LuksClient::new()
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to create LUKS client: {}", e))?;
                    if use_defaults {
                        luks_client
                            .default_encryption_options(&device_path)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to clear encryption options: {}", e)
                            })?;
                    } else {
                        let settings = storage_types::EncryptionOptionsSettings {
                            name,
                            unlock_at_startup,
                            require_auth,
                            other_options,
                            passphrase: if passphrase.is_empty() {
                                None
                            } else {
                                Some(passphrase)
                            },
                        };
                        luks_client
                            .set_encryption_options(&device_path, &settings)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to set encryption options: {}", e)
                            })?;
                    }
                    load_all_drives().await.map_err(|e| e.into())
                },
                move |result: Result<Vec<UiDrive>, anyhow::Error>| match result {
                    Ok(drives) => Message::UpdateNav(drives, None).into(),
                    Err(e) => {
                        let ctx = UiErrorContext::new("edit_encryption_options");
                        log_error_and_show_dialog(fl!("edit-encryption-options"), e, ctx).into()
                    }
                },
            )
        }
    }
}

pub(super) fn lock_container(control: &mut VolumesControl) -> Task<cosmic::Action<Message>> {
    let segment = control.segments.get(control.selected_segment).cloned();
    if let Some(s) = segment
        && let Some(p) = s.volume
    {
        let mounted_children: Vec<String> =
            crate::state::volumes::find_volume_for_partition(&control.volumes, &p)
                .map(crate::update::volumes::helpers::collect_mounted_descendants_leaf_first)
                .unwrap_or_default();

        let device_path_for_selection = p.device_path.clone().unwrap_or_else(|| p.label.clone());
        return Task::perform(
            async move {
                let fs_client = FilesystemsClient::new()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to create filesystems client: {}", e))?;
                let luks_client = LuksClient::new()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to create LUKS client: {}", e))?;

                for device in &mounted_children {
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
                    .map_err(|e| anyhow::anyhow!("Failed to lock: {}", e))?;

                load_all_drives().await.map_err(|e| e.into())
            },
            move |result: Result<Vec<UiDrive>, anyhow::Error>| match result {
                Ok(drives) => {
                    // After lock, stay on the locked container
                    Message::UpdateNavWithChildSelection(
                        drives,
                        Some(device_path_for_selection.clone()),
                    )
                    .into()
                }
                Err(e) => {
                    let ctx = UiErrorContext::new("lock_container");
                    log_error_and_show_dialog(fl!("lock-failed"), e, ctx).into()
                }
            },
        );
    }

    Task::none()
}

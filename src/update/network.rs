// SPDX-License-Identifier: GPL-3.0-only

//! Network mount message handling

use crate::errors::ui::{UiErrorContext, log_error_and_show_dialog};
use crate::message::app::Message;
use crate::message::network::NetworkMessage;
use crate::operations::RcloneClient;
use crate::state::app::AppModel;
use crate::state::dialogs::ShowDialog;
use cosmic::app::Task;
use storage_types::rclone::{
    ConfigScope, MountStatus, RemoteConfig, rclone_provider, supported_remote_types,
};

/// Handle network-related messages
pub(crate) fn handle_network_message(app: &mut AppModel, message: NetworkMessage) -> Task<Message> {
    let client = RcloneClient::with_operations(app.runtime.operations());
    match message {
        NetworkMessage::LoadRemotes => {
            app.network.loading = true;
            return Task::perform(
                async move {
                    match client.list_remotes().await {
                        Ok(list) => Ok(list.remotes),
                        Err(e) => Err(format!("Failed to list remotes: {}", e)),
                    }
                },
                |result| Message::NetworkRemotesLoaded(result).into(),
            );
        }

        NetworkMessage::RemotesLoaded(result) => {
            app.network.loading = false;
            match result {
                Ok(remotes) => {
                    app.network.rclone_available = true;
                    let refresh_tasks: Vec<Task<Message>> = remotes
                        .iter()
                        .map(|remote| {
                            Task::done(
                                Message::Network(NetworkMessage::RefreshStatus {
                                    name: remote.name.clone(),
                                    scope: remote.scope,
                                })
                                .into(),
                            )
                        })
                        .collect();
                    app.network.set_remotes(remotes);
                    return Task::batch(refresh_tasks);
                }
                Err(e) => {
                    app.network.rclone_available = false;
                    app.network.error = Some(e);
                }
            }
        }

        NetworkMessage::SelectRemote { name, scope } => {
            app.network.select(Some(name.clone()), Some(scope));
            if let Some(config) = app
                .network
                .get_mount(&name, scope)
                .map(|m| m.config.clone())
            {
                app.network.start_edit(config);
                if let Some(editor) = app.network.editor.as_mut() {
                    editor.mount_on_boot = None;
                }
                let name_for_task = name.clone();
                return Task::perform(
                    async move {
                        client
                            .get_mount_on_boot(&name_for_task, &scope.to_string())
                            .await
                            .map_err(|e| e.to_string())
                    },
                    move |result| {
                        Message::Network(NetworkMessage::MountOnBootLoaded {
                            name: name.clone(),
                            scope,
                            result,
                        })
                        .into()
                    },
                );
            } else {
                app.network.clear_editor();
            }
        }

        NetworkMessage::BeginCreateRemote => {
            app.network.select(None, None);
            app.network.clear_editor();
            app.network.start_wizard();
        }

        NetworkMessage::EditorNameChanged(name) => {
            if let Some(editor) = app.network.editor.as_mut() {
                editor.name = name;
                editor.error = None;
            }
        }

        NetworkMessage::EditorTypeIndexChanged(index) => {
            if let Some(editor) = app.network.editor.as_mut()
                && let Some(remote_type) = supported_remote_types().get(index)
            {
                editor.remote_type = remote_type.clone();
                editor.error = None;
            }
        }

        NetworkMessage::EditorFieldChanged { key, value } => {
            if let Some(editor) = app.network.editor.as_mut() {
                editor.options.insert(key, value);
                editor.error = None;
            }
        }

        NetworkMessage::EditorNewOptionKeyChanged(value) => {
            if let Some(editor) = app.network.editor.as_mut() {
                editor.new_option_key = value;
                editor.error = None;
            }
        }

        NetworkMessage::EditorNewOptionValueChanged(value) => {
            if let Some(editor) = app.network.editor.as_mut() {
                editor.new_option_value = value;
                editor.error = None;
            }
        }

        NetworkMessage::EditorAddCustomOption => {
            if let Some(editor) = app.network.editor.as_mut() {
                let key = editor.new_option_key.trim().to_string();
                if key.is_empty() {
                    editor.error = Some("Option key cannot be empty".to_string());
                    return Task::none();
                }
                if key.eq_ignore_ascii_case("type") {
                    editor.error = Some("Option key cannot be 'type'".to_string());
                    return Task::none();
                }
                if editor.options.keys().any(|k| k.eq_ignore_ascii_case(&key)) {
                    editor.error = Some("Option already exists".to_string());
                    return Task::none();
                }
                editor.options.insert(key, editor.new_option_value.clone());
                editor.new_option_key.clear();
                editor.new_option_value.clear();
                editor.error = None;
            }
        }

        NetworkMessage::EditorRemoveCustomOption { key } => {
            if let Some(editor) = app.network.editor.as_mut() {
                editor.options.remove(&key);
            }
        }

        NetworkMessage::EditorShowAdvanced(show) => {
            if let Some(editor) = app.network.editor.as_mut() {
                editor.show_advanced = show;
            }
        }

        NetworkMessage::EditorShowHidden(show) => {
            if let Some(editor) = app.network.editor.as_mut() {
                editor.show_hidden = show;
            }
        }

        NetworkMessage::EditorToggleSection(section) => {
            if let Some(editor) = app.network.editor.as_mut() {
                if editor.expanded_sections.contains(&section) {
                    editor.expanded_sections.remove(&section);
                } else {
                    editor.expanded_sections.insert(section);
                }
            }
        }

        // ── Wizard messages ─────────────────────────────────────────
        NetworkMessage::WizardSelectType(type_name) => {
            if let Some(wizard) = app.network.wizard.as_mut() {
                wizard.remote_type = type_name;
                wizard.error = None;
            }
        }

        NetworkMessage::WizardAdvanced => {
            // Switch from wizard to full editor mode
            app.network.wizard_to_editor();
        }

        NetworkMessage::WizardSetName(name) => {
            if let Some(wizard) = app.network.wizard.as_mut() {
                wizard.name = name;
                wizard.error = None;
            }
        }

        NetworkMessage::WizardFieldChanged { key, value } => {
            if let Some(wizard) = app.network.wizard.as_mut() {
                wizard.options.insert(key, value);
                wizard.error = None;
            }
        }

        NetworkMessage::WizardNext => {
            if let Some(wizard) = app.network.wizard.as_mut()
                && wizard.can_advance()
            {
                wizard.next_step();
            }
        }

        NetworkMessage::WizardBack => {
            if let Some(wizard) = app.network.wizard.as_mut() {
                wizard.prev_step();
            }
        }

        NetworkMessage::WizardCreate => {
            // Extract wizard data and create the remote
            let wizard_data = {
                let Some(wizard) = app.network.wizard.as_mut() else {
                    return Task::none();
                };
                if wizard.running {
                    return Task::none();
                }

                if wizard.name.trim().is_empty() {
                    wizard.error = Some("Remote name cannot be empty".to_string());
                    return Task::none();
                }

                let provider = rclone_provider(&wizard.remote_type);
                if provider.is_none() {
                    wizard.error = Some("Unsupported remote type".to_string());
                    return Task::none();
                }

                // Validate required fields
                if let Some(provider) = provider {
                    for option in &provider.options {
                        if !option.required || option.is_hidden() {
                            continue;
                        }
                        let value = wizard.options.get(&option.name).map(|v| v.trim());
                        if value.is_none() || value == Some("") {
                            wizard.error =
                                Some(format!("Missing required field '{}'", option.name));
                            return Task::none();
                        }
                    }
                }

                let name = wizard.name.clone();
                let remote_type = wizard.remote_type.clone();
                let scope = wizard.scope;
                let options: std::collections::HashMap<String, String> = wizard
                    .options
                    .iter()
                    .filter(|(_, v)| !v.trim().is_empty())
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                let has_secrets = provider.is_some_and(|provider| {
                    provider
                        .options
                        .iter()
                        .filter(|option| option.is_secure())
                        .any(|option| {
                            options
                                .get(&option.name)
                                .is_some_and(|value| !value.trim().is_empty())
                        })
                });

                wizard.running = true;
                wizard.error = None;

                (name, remote_type, scope, options, has_secrets)
            };

            let (name, remote_type, scope, options, has_secrets) = wizard_data;
            let operation_id = uuid::Uuid::new_v4();
            app.network.wizard.as_mut().unwrap().operation_id = Some(operation_id);

            return Task::perform(
                async move {
                    let config = RemoteConfig {
                        name: name.clone(),
                        remote_type,
                        scope,
                        options,
                        has_secrets,
                    };
                    client
                        .create_remote(&config)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(config)
                },
                move |result| {
                    Message::Network(NetworkMessage::WizardCreateCompleted {
                        operation_id,
                        result,
                    })
                    .into()
                },
            );
        }

        NetworkMessage::WizardCancel => {
            app.network.clear_wizard();
        }

        NetworkMessage::WizardCreateCompleted {
            operation_id,
            result,
        } => {
            if !app
                .network
                .wizard
                .as_ref()
                .is_some_and(|wizard| wizard.running && wizard.operation_id == Some(operation_id))
            {
                return Task::none();
            }
            match result {
                Ok(config) => {
                    let name = config.name.clone();
                    let scope = config.scope;
                    app.network.clear_wizard();
                    // Publish the confirmed configuration before selection. Chaining
                    // LoadRemotes -> SelectRemote does not await the nested load task.
                    app.network.mounts.insert(
                        (name.clone(), scope),
                        crate::state::network::NetworkMountState::new(config),
                    );
                    let selection =
                        handle_network_message(app, NetworkMessage::SelectRemote { name, scope });
                    return Task::batch([
                        selection,
                        Task::done(Message::Network(NetworkMessage::LoadRemotes).into()),
                    ]);
                }
                Err(e) => {
                    if let Some(wizard) = app.network.wizard.as_mut() {
                        wizard.running = false;
                        wizard.operation_id = None;
                        wizard.error = Some(e);
                    }
                }
            }
        }

        NetworkMessage::SaveRemote => {
            let (
                name,
                remote_type,
                scope,
                options,
                has_secrets,
                is_edit,
                original_name,
                original_scope,
            ) = {
                let Some(editor) = app.network.editor.as_mut() else {
                    return Task::none();
                };
                if editor.running {
                    return Task::none();
                }

                if editor.name.trim().is_empty() {
                    editor.error = Some("Remote name cannot be empty".to_string());
                    return Task::none();
                }

                let provider = rclone_provider(&editor.remote_type);
                if provider.is_none() {
                    editor.error = Some("Unsupported remote type".to_string());
                    return Task::none();
                }

                if let Some(provider) = provider {
                    for option in &provider.options {
                        if !option.required || option.is_hidden() {
                            continue;
                        }
                        let value = editor.options.get(&option.name).map(|v| v.trim());
                        if value.is_none() || value == Some("") {
                            editor.error =
                                Some(format!("Missing required field '{}'", option.name));
                            return Task::none();
                        }
                    }
                }

                let name = editor.name.clone();
                let remote_type = editor.remote_type.clone();
                let scope = editor.scope;
                let options: std::collections::HashMap<String, String> = editor
                    .options
                    .iter()
                    .filter(|(_, v)| !v.trim().is_empty())
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                let has_secrets = provider.is_some_and(|provider| {
                    provider
                        .options
                        .iter()
                        .filter(|option| option.is_secure())
                        .any(|option| {
                            options
                                .get(&option.name)
                                .is_some_and(|value| !value.trim().is_empty())
                        })
                });

                let is_edit = !editor.is_new;
                let original_name = editor.original_name.clone();
                let original_scope = editor.original_scope;

                editor.running = true;
                editor.error = None;

                (
                    name,
                    remote_type,
                    scope,
                    options,
                    has_secrets,
                    is_edit,
                    original_name,
                    original_scope,
                )
            };

            let operation_id = uuid::Uuid::new_v4();
            app.network.editor.as_mut().unwrap().operation_id = Some(operation_id);

            return Task::perform(
                async move {
                    let config = RemoteConfig {
                        name: name.clone(),
                        remote_type,
                        scope,
                        options,
                        has_secrets,
                    };

                    if is_edit {
                        if let Some(original) = &original_name {
                            let scope_changed = original_scope.is_some_and(|s| s != scope);
                            if original != &name || scope_changed {
                                client
                                    .delete_remote(
                                        original,
                                        &original_scope.unwrap_or(scope).to_string(),
                                    )
                                    .await
                                    .map_err(|e| e.to_string())?;
                                client
                                    .create_remote(&config)
                                    .await
                                    .map_err(|e| e.to_string())?;
                            } else {
                                client
                                    .update_remote(&name, &config)
                                    .await
                                    .map_err(|e| e.to_string())?;
                            }
                        } else {
                            client
                                .update_remote(&name, &config)
                                .await
                                .map_err(|e| e.to_string())?;
                        }
                    } else {
                        client
                            .create_remote(&config)
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                    Ok(())
                },
                move |result| {
                    Message::Network(NetworkMessage::SaveCompleted {
                        operation_id,
                        result,
                    })
                    .into()
                },
            );
        }

        NetworkMessage::SaveCompleted {
            operation_id,
            result,
        } => {
            if let Some(editor) = app.network.editor.as_mut()
                && editor.running
                && editor.operation_id == Some(operation_id)
            {
                editor.running = false;
                editor.operation_id = None;
                match result {
                    Ok(()) => {
                        editor.error = None;
                        editor.is_new = false;
                        editor.original_name = Some(editor.name.clone());
                        editor.original_scope = Some(editor.scope);
                        editor.mount_on_boot = Some(false);
                        let name = editor.name.clone();
                        let scope = editor.scope;
                        return Task::batch(vec![
                            Task::done(Message::Network(NetworkMessage::LoadRemotes).into()),
                            Task::done(
                                Message::Network(NetworkMessage::LoadMountOnBoot { name, scope })
                                    .into(),
                            ),
                        ]);
                    }
                    Err(e) => {
                        editor.error = Some(e);
                    }
                }
            }
        }

        NetworkMessage::LoadMountOnBoot { name, scope } => {
            let name_for_task = name.clone();
            return Task::perform(
                async move {
                    client
                        .get_mount_on_boot(&name_for_task, &scope.to_string())
                        .await
                        .map_err(|e| e.to_string())
                },
                move |result| {
                    Message::Network(NetworkMessage::MountOnBootLoaded {
                        name: name.clone(),
                        scope,
                        result,
                    })
                    .into()
                },
            );
        }

        NetworkMessage::MountOnBootLoaded {
            name,
            scope,
            result,
        } => {
            if app
                .network
                .selected
                .as_ref()
                .is_some_and(|(n, s)| n == &name && *s == scope)
                && let Some(editor) = app.network.editor.as_mut()
            {
                match result {
                    Ok(enabled) => {
                        editor.mount_on_boot = Some(enabled);
                    }
                    Err(e) => {
                        editor.mount_on_boot = Some(false);
                        let ctx = UiErrorContext::new("rclone_mount_on_boot_status");
                        return Task::done(
                            log_error_and_show_dialog(
                                "Failed to read mount on boot status",
                                anyhow::anyhow!(e),
                                ctx,
                            )
                            .into(),
                        );
                    }
                }
            }
        }

        NetworkMessage::ToggleMountOnBoot(enabled) => {
            let Some(editor) = app.network.editor.as_mut() else {
                return Task::none();
            };
            if editor.is_new {
                return Task::none();
            }
            let Some((name, scope)) = app.network.selected.clone() else {
                return Task::none();
            };
            let previous = editor.mount_on_boot.unwrap_or(false);
            editor.running = true;
            editor.error = None;
            let name_for_task = name.clone();
            return Task::perform(
                async move {
                    client
                        .set_mount_on_boot(&name_for_task, &scope.to_string(), enabled)
                        .await
                        .map_err(|e| e.to_string())
                },
                move |result| {
                    Message::Network(NetworkMessage::MountOnBootUpdated {
                        name: name.clone(),
                        scope,
                        enabled,
                        previous,
                        result,
                    })
                    .into()
                },
            );
        }

        NetworkMessage::MountOnBootUpdated {
            name,
            scope,
            enabled,
            previous,
            result,
        } => {
            if app
                .network
                .selected
                .as_ref()
                .is_some_and(|(n, s)| n == &name && *s == scope)
                && let Some(editor) = app.network.editor.as_mut()
            {
                editor.running = false;
                match result {
                    Ok(()) => {
                        editor.mount_on_boot = Some(enabled);
                    }
                    Err(e) => {
                        editor.mount_on_boot = Some(previous);
                        let ctx = UiErrorContext::new("rclone_mount_on_boot_update");
                        return Task::done(
                            log_error_and_show_dialog(
                                "Failed to update mount on boot",
                                anyhow::anyhow!(e),
                                ctx,
                            )
                            .into(),
                        );
                    }
                }
            }
        }

        NetworkMessage::OpenMountPath(path) => {
            return Task::done(Message::OpenPath(path).into());
        }

        NetworkMessage::MountRemote { name, scope } => {
            let Some(request_id) = app.network.begin_mount_request(&name, scope, true) else {
                return Task::none();
            };
            app.network.set_loading(&name, scope, true);
            let name_for_task = name.clone();
            return Task::perform(
                async move {
                    {
                        match client.mount(&name_for_task, &scope.to_string()).await {
                            Ok(()) => Ok(()),
                            Err(e) => Err(e.to_string()),
                        }
                    }
                },
                move |result| {
                    Message::Network(NetworkMessage::MountResult {
                        name: name.clone(),
                        scope,
                        request_id,
                        result: result.map(|()| MountStatus::Mounted),
                    })
                    .into()
                },
            );
        }

        NetworkMessage::UnmountRemote { name, scope } => {
            let Some(request_id) = app.network.begin_mount_request(&name, scope, true) else {
                return Task::none();
            };
            app.network.set_loading(&name, scope, true);
            let name_for_task = name.clone();
            return Task::perform(
                async move {
                    {
                        match client.unmount(&name_for_task, &scope.to_string()).await {
                            Ok(()) => Ok(()),
                            Err(e) => Err(e.to_string()),
                        }
                    }
                },
                move |result| {
                    Message::Network(NetworkMessage::MountResult {
                        name: name.clone(),
                        scope,
                        request_id,
                        result: result.map(|()| MountStatus::Unmounted),
                    })
                    .into()
                },
            );
        }

        NetworkMessage::RestartRemote { name, scope } => {
            let Some(request_id) = app.network.begin_mount_request(&name, scope, true) else {
                return Task::none();
            };
            // Restart is implemented as unmount followed by mount
            // Set loading state and start with unmount
            app.network.set_loading(&name, scope, true);
            let name_for_task = name.clone();
            return Task::perform(
                async move {
                    {
                        // First unmount
                        if let Err(e) = client.unmount(&name_for_task, &scope.to_string()).await {
                            // If unmount fails, try to mount anyway (might not have been mounted)
                            tracing::warn!(
                                "Unmount during restart failed: {}, attempting mount anyway",
                                e
                            );
                        }
                        // Then mount
                        match client.mount(&name_for_task, &scope.to_string()).await {
                            Ok(()) => Ok(()),
                            Err(e) => Err(e.to_string()),
                        }
                    }
                },
                move |result| {
                    Message::Network(NetworkMessage::MountResult {
                        name: name.clone(),
                        scope,
                        request_id,
                        result: result.map(|()| MountStatus::Mounted),
                    })
                    .into()
                },
            );
        }

        NetworkMessage::MountResult {
            name,
            scope,
            request_id,
            result,
        } => {
            let Some(mount) = app.network.get_mount_mut(&name, scope) else {
                return Task::none();
            };
            if mount.request_id != Some(request_id) {
                return Task::none();
            }
            mount.request_id = None;
            let mutating = mount.loading;
            mount.loading = false;
            match result {
                Ok(status) => {
                    mount.status = status;
                    mount.error = None;
                }
                Err(error) => {
                    mount.status = MountStatus::Error(error.clone());
                    mount.error = Some(error.clone());
                    if mutating {
                        app.dialog = Some(ShowDialog::Info {
                            title: "Network operation failed".into(),
                            body: format!("Remote '{name}': {error}"),
                        });
                    }
                }
            }
        }

        NetworkMessage::TestRemote { name, scope } => {
            let name_for_task = name.clone();
            return Task::perform(
                async move {
                    {
                        match client.test_remote(&name_for_task, &scope.to_string()).await {
                            Ok(result) => {
                                if result.success {
                                    Ok(format!(
                                        "Connection successful{}",
                                        result
                                            .latency_ms
                                            .map(|l| format!(" ({}ms)", l))
                                            .unwrap_or_default()
                                    ))
                                } else {
                                    Err(result.message)
                                }
                            }
                            Err(e) => Err(e.to_string()),
                        }
                    }
                },
                move |result| Message::Network(NetworkMessage::TestCompleted { result }).into(),
            );
        }

        NetworkMessage::TestCompleted { result } => {
            // Show test result in a dialog
            let (title, body) = match result {
                Ok(msg) => ("Connection Test".to_string(), msg),
                Err(e) => ("Connection Test Failed".to_string(), e),
            };
            app.dialog = Some(ShowDialog::Info { title, body });
        }

        NetworkMessage::RefreshStatus { name, scope } => {
            let Some(request_id) = app.network.begin_mount_request(&name, scope, false) else {
                return Task::none();
            };
            let name_for_task = name.clone();
            return Task::perform(
                async move {
                    {
                        match client
                            .get_mount_status(&name_for_task, &scope.to_string())
                            .await
                        {
                            Ok(status) => Ok(status.status),
                            Err(e) => Err(e.to_string()),
                        }
                    }
                },
                move |result| {
                    Message::Network(NetworkMessage::MountResult {
                        name: name.clone(),
                        scope,
                        request_id,
                        result,
                    })
                    .into()
                },
            );
        }

        NetworkMessage::DeleteRemote { name, scope } => {
            // Show confirmation dialog before deleting
            app.dialog = Some(ShowDialog::ConfirmDeleteRemote { name, scope });
        }

        NetworkMessage::ConfirmDeleteRemote { name, scope } => {
            // Close the dialog first
            app.dialog = None;
            // Then proceed with the actual delete
            let name_for_task = name.clone();
            return Task::perform(
                async move {
                    {
                        match client
                            .delete_remote(&name_for_task, &scope.to_string())
                            .await
                        {
                            Ok(()) => Ok(()),
                            Err(e) => Err(e.to_string()),
                        }
                    }
                },
                move |result| {
                    Message::Network(NetworkMessage::DeleteCompleted {
                        name: name.clone(),
                        result,
                    })
                    .into()
                },
            );
        }

        NetworkMessage::DeleteCompleted { name, result } => {
            match result {
                Ok(()) => {
                    // Remove from state
                    app.network
                        .mounts
                        .remove(&(name.clone(), ConfigScope::User));
                    app.network
                        .mounts
                        .remove(&(name.clone(), ConfigScope::System));
                    tracing::info!("Deleted remote: {}", name);
                    if app
                        .network
                        .selected
                        .as_ref()
                        .is_some_and(|(n, _)| n == &name)
                    {
                        app.network.select(None, None);
                        app.network.clear_editor();
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to delete remote {}: {}", name, e);
                    // Show error dialog
                    app.dialog = Some(ShowDialog::Info {
                        title: "Delete Failed".to_string(),
                        body: format!("Failed to delete remote '{}': {}", name, e),
                    });
                }
            }
        }
    }

    Task::none()
}

// Remote config dialog handling removed in favor of main view editor

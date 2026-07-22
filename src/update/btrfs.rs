use crate::errors::ui::{UiErrorContext, log_error_and_show_dialog};
use crate::fl;
use crate::message::app::Message;
use crate::models::load_all_drives;
use crate::operations::BtrfsClient;
use crate::state::app::AppModel;
use crate::state::dialogs::{ConfirmActionDialog, FilesystemTarget, ShowDialog};
use crate::state::volumes::VolumesControl;
use cosmic::app::Task;

/// Handle BTRFS management messages
pub(super) fn handle_btrfs_message(app: &mut AppModel, message: Message) -> Task<Message> {
    match message {
        Message::BtrfsLoadSubvolumes {
            block_path: _,
            mount_point,
        } => {
            // Set loading state
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>()
                && let Some(btrfs_state) = &mut volumes_control.btrfs_state
            {
                btrfs_state.loading = true;
            }

            // Spawn async task to load subvolumes
            let mount_point_for_callback = mount_point.clone();
            Task::perform(
                async move {
                    let btrfs_client = BtrfsClient::new().await?;
                    let subvol_list = btrfs_client.list_subvolumes(&mount_point).await?;
                    Ok(subvol_list.subvolumes)
                },
                move |result: anyhow::Result<Vec<storage_types::BtrfsSubvolume>>| {
                    if let Err(ref e) = result {
                        tracing::error!("Failed to load BTRFS subvolumes: {:#}", e);
                    }
                    let result = result.map_err(|e| format!("{:#}", e));
                    Message::BtrfsSubvolumesLoaded {
                        mount_point: mount_point_for_callback.clone(),
                        result,
                    }
                    .into()
                },
            )
        }

        Message::BtrfsSubvolumesLoaded {
            mount_point,
            result,
        } => {
            // Update state with loaded subvolumes
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>()
                && let Some(btrfs_state) = &mut volumes_control.btrfs_state
                && btrfs_state.mount_point.as_deref() == Some(&mount_point)
            {
                btrfs_state.loading = false;
                btrfs_state.subvolumes = Some(result);
            }
            Task::none()
        }

        Message::BtrfsDeleteSubvolume {
            block_path,
            mount_point,
            path,
        } => {
            // Show confirmation dialog
            let subvol_name = path.rsplit('/').next().unwrap_or(&path).to_string();

            // Get a dummy FilesystemTarget (required by ConfirmActionDialog but not used for BTRFS)
            let target = if let Some(volumes_control) = app.nav.active_data::<VolumesControl>()
                && let Some(segment) = volumes_control
                    .segments
                    .get(volumes_control.selected_segment)
                && let Some(volume) = &segment.volume
            {
                FilesystemTarget::Volume(volume.clone())
            } else {
                return Task::none();
            };

            app.dialog = Some(ShowDialog::ConfirmAction(ConfirmActionDialog {
                title: fl!("btrfs-delete-subvolume"),
                body: fl!("btrfs-delete-confirm", name = subvol_name.as_str()),
                target,
                ok_message: Message::BtrfsDeleteSubvolumeConfirm {
                    block_path,
                    mount_point,
                    path,
                },
                running: false,
            }));

            Task::none()
        }

        Message::BtrfsDeleteSubvolumeConfirm {
            block_path: _,
            mount_point,
            path,
        } => {
            // Set dialog to running state
            if let Some(ShowDialog::ConfirmAction(state)) = &mut app.dialog {
                state.running = true;
            }

            // Perform the actual delete
            Task::perform(
                async move {
                    let btrfs_client = BtrfsClient::new().await?;
                    btrfs_client
                        .delete_subvolume(&mount_point, &path, false)
                        .await?;
                    load_all_drives().await
                },
                |result| match result {
                    Ok(drives) => {
                        // Close dialog and refresh drives (subvolume list will reload)
                        Message::UpdateNav(drives, None).into()
                    }
                    Err(e) => {
                        let ctx = UiErrorContext::new("delete_subvolume");
                        log_error_and_show_dialog(
                            fl!("btrfs-delete-subvolume-failed"),
                            e.into(),
                            ctx,
                        )
                        .into()
                    }
                },
            )
        }

        Message::BtrfsLoadUsage {
            block_path: _,
            mount_point,
        } => {
            // Mark as loading in state
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>()
                && let Some(btrfs_state) = &mut volumes_control.btrfs_state
            {
                btrfs_state.loading_usage = true;
            }

            // Load usage in background task
            Task::perform(
                async move {
                    let btrfs_client = match BtrfsClient::new().await {
                        Ok(client) => client,
                        Err(e) => {
                            return Message::BtrfsUsageLoaded {
                                mount_point,
                                used_space: Err(format!(
                                    "Failed to initialize BTRFS client: {}",
                                    e
                                )),
                            };
                        }
                    };
                    let result = btrfs_client
                        .get_usage(&mount_point)
                        .await
                        .map(|usage| usage.used_bytes)
                        .map_err(|e| format!("Failed to get usage: {}", e));

                    Message::BtrfsUsageLoaded {
                        mount_point,
                        used_space: result,
                    }
                },
                |msg| msg.into(),
            )
        }

        Message::BtrfsUsageLoaded {
            mount_point: _,
            used_space,
        } => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>()
                && let Some(btrfs_state) = &mut volumes_control.btrfs_state
            {
                btrfs_state.loading_usage = false;
                btrfs_state.used_space = Some(used_space);
            }
            Task::none()
        }

        Message::BtrfsToggleSubvolumeExpanded {
            mount_point,
            subvolume_id,
        } => {
            // Toggle the expanded state for a subvolume's snapshots
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>()
                && let Some(btrfs_state) = &mut volumes_control.btrfs_state
                && btrfs_state.mount_point.as_deref() == Some(&mount_point)
            {
                let expanded = btrfs_state
                    .expanded_subvolumes
                    .entry(subvolume_id)
                    .or_insert(false);
                *expanded = !*expanded;
            }
            Task::none()
        }

        Message::BtrfsLoadDefaultSubvolume { mount_point } => {
            let mount_point_for_async = mount_point.clone();
            Task::perform(
                async move {
                    let btrfs_client = BtrfsClient::new().await?;
                    let default_id = btrfs_client.get_default(&mount_point).await?;
                    // Need to fetch default subvolume info from the list
                    let subvol_list = btrfs_client.list_subvolumes(&mount_point).await?;
                    let default_subvol = subvol_list
                        .subvolumes
                        .into_iter()
                        .find(|s| s.id == default_id)
                        .ok_or_else(|| anyhow::anyhow!("Default subvolume not found"))?;
                    Ok(default_subvol)
                },
                move |result: anyhow::Result<storage_types::BtrfsSubvolume>| {
                    let result = result.map_err(|e| format!("{:#}", e));
                    Message::BtrfsDefaultSubvolumeLoaded {
                        mount_point: mount_point_for_async.clone(),
                        result,
                    }
                    .into()
                },
            )
        }

        Message::BtrfsDefaultSubvolumeLoaded {
            mount_point,
            result,
        } => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>()
                && let Some(btrfs_state) = &mut volumes_control.btrfs_state
                && btrfs_state.mount_point.as_deref() == Some(&mount_point)
            {
                match result {
                    Ok(subvol) => {
                        btrfs_state.default_subvolume_id = Some(subvol.id);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load default subvolume: {}", e);
                    }
                }
            }
            Task::none()
        }

        Message::BtrfsSetDefaultSubvolume {
            mount_point,
            subvolume_id,
        } => {
            // Find the subvolume path by ID
            let subvol_path = if let Some(volumes_control) = app.nav.active_data::<VolumesControl>()
                && let Some(btrfs_state) = &volumes_control.btrfs_state
                && let Some(Ok(subvolumes)) = &btrfs_state.subvolumes
            {
                subvolumes
                    .iter()
                    .find(|s| s.id == subvolume_id)
                    .map(|s| s.path.clone())
            } else {
                None
            };

            // Get the btrfs client from app state
            if let Some(path) = subvol_path {
                let mount_point_clone = mount_point.clone();
                let mount_point_for_closure = mount_point.clone();
                Task::perform(
                    async move {
                        let btrfs_client = BtrfsClient::new().await?;
                        btrfs_client.set_default(&mount_point_clone, &path).await?;
                        Ok(())
                    },
                    move |result: anyhow::Result<()>| {
                        match result {
                            Ok(()) => {
                                // Reload subvolumes to update default flag
                                Message::BtrfsLoadSubvolumes {
                                    block_path: String::new(),
                                    mount_point: mount_point_for_closure.clone(),
                                }
                            }
                            Err(e) => {
                                let ctx = UiErrorContext::new("set_default_subvolume");
                                log_error_and_show_dialog(fl!("btrfs-set-default-failed"), e, ctx)
                            }
                        }
                        .into()
                    },
                )
            } else {
                Task::none()
            }
        }

        Message::BtrfsToggleReadonly {
            mount_point,
            subvolume_id,
        } => {
            // Find the subvolume by ID
            let subvol_info = if let Some(volumes_control) = app.nav.active_data::<VolumesControl>()
                && let Some(btrfs_state) = &volumes_control.btrfs_state
                && let Some(Ok(subvolumes)) = &btrfs_state.subvolumes
            {
                const BTRFS_SUBVOL_RDONLY: u64 = 1 << 1;
                subvolumes
                    .iter()
                    .find(|s| s.id == subvolume_id)
                    .map(|s| (s.path.clone(), (s.flags & BTRFS_SUBVOL_RDONLY) != 0))
            } else {
                None
            };

            if let Some((path, current_readonly)) = subvol_info {
                let new_readonly = !current_readonly;
                let mount_point_for_closure = mount_point.clone();
                Task::perform(
                    async move {
                        let btrfs_client = BtrfsClient::new().await?;
                        btrfs_client
                            .set_readonly(&mount_point, &path, new_readonly)
                            .await?;
                        Ok(())
                    },
                    move |result: anyhow::Result<()>| {
                        let result = result.map_err(|e| format!("{:#}", e));
                        Message::BtrfsReadonlyToggled {
                            mount_point: mount_point_for_closure.clone(),
                            result,
                        }
                        .into()
                    },
                )
            } else {
                Task::none()
            }
        }

        Message::BtrfsReadonlyToggled {
            mount_point,
            result,
        } => {
            match result {
                Ok(()) => {
                    // Reload subvolumes to update readonly flag
                    handle_btrfs_message(
                        app,
                        Message::BtrfsLoadSubvolumes {
                            block_path: String::new(),
                            mount_point,
                        },
                    )
                }
                Err(e) => {
                    let ctx = UiErrorContext::new("toggle_readonly");
                    Task::done(
                        log_error_and_show_dialog(
                            fl!("btrfs-readonly-failed"),
                            anyhow::anyhow!(e),
                            ctx,
                        )
                        .into(),
                    )
                }
            }
        }

        Message::BtrfsShowProperties {
            mount_point: _,
            subvolume_id,
        } => {
            // Find and store the selected subvolume
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>()
                && let Some(btrfs_state) = &mut volumes_control.btrfs_state
                && let Some(Ok(subvolumes)) = &btrfs_state.subvolumes
            {
                let subvol = subvolumes.iter().find(|s| s.id == subvolume_id).cloned();
                if let Some(subvol) = subvol {
                    btrfs_state.selected_subvolume = Some(subvol);
                    btrfs_state.show_properties_dialog = true;
                }
            }
            Task::none()
        }

        Message::BtrfsCloseProperties { mount_point } => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>()
                && let Some(btrfs_state) = &mut volumes_control.btrfs_state
                && btrfs_state.mount_point.as_deref() == Some(&mount_point)
            {
                btrfs_state.show_properties_dialog = false;
                btrfs_state.selected_subvolume = None;
            }
            Task::none()
        }

        Message::BtrfsLoadDeletedSubvolumes { mount_point } => {
            let mount_point_for_async = mount_point.clone();
            Task::perform(
                async move {
                    let btrfs_client = BtrfsClient::new().await?;
                    let deleted_list = btrfs_client.list_deleted(&mount_point).await?;
                    Ok(deleted_list)
                },
                move |result: anyhow::Result<Vec<storage_types::DeletedSubvolume>>| {
                    let result = result.map_err(|e| format!("{:#}", e));
                    Message::BtrfsDeletedSubvolumesLoaded {
                        mount_point: mount_point_for_async.clone(),
                        result,
                    }
                    .into()
                },
            )
        }

        Message::BtrfsDeletedSubvolumesLoaded {
            mount_point,
            result,
        } => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>()
                && let Some(btrfs_state) = &mut volumes_control.btrfs_state
                && btrfs_state.mount_point.as_deref() == Some(&mount_point)
            {
                match result {
                    Ok(deleted) => {
                        btrfs_state.deleted_subvolumes = Some(deleted);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load deleted subvolumes: {}", e);
                    }
                }
            }
            Task::none()
        }

        Message::BtrfsToggleShowDeleted { mount_point } => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>()
                && let Some(btrfs_state) = &mut volumes_control.btrfs_state
                && btrfs_state.mount_point.as_deref() == Some(&mount_point)
            {
                btrfs_state.show_deleted = !btrfs_state.show_deleted;

                // Load deleted subvolumes if we're showing them and haven't loaded yet
                if btrfs_state.show_deleted && btrfs_state.deleted_subvolumes.is_none() {
                    return handle_btrfs_message(
                        app,
                        Message::BtrfsLoadDeletedSubvolumes {
                            mount_point: mount_point.clone(),
                        },
                    );
                }
            }
            Task::none()
        }

        Message::BtrfsRefreshAll { mount_point } => {
            // Reload all BTRFS data
            Task::batch(vec![
                handle_btrfs_message(
                    app,
                    Message::BtrfsLoadSubvolumes {
                        block_path: String::new(),
                        mount_point: mount_point.clone(),
                    },
                ),
                handle_btrfs_message(
                    app,
                    Message::BtrfsLoadDefaultSubvolume {
                        mount_point: mount_point.clone(),
                    },
                ),
            ])
        }

        _ => Task::none(),
    }
}

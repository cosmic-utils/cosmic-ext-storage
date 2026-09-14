use crate::errors::ui::{UiErrorContext, log_error_and_show_dialog};
use crate::fl;
use crate::message::app::Message;
use crate::operations::BtrfsClient;
use crate::state::app::AppModel;
use crate::state::volumes::VolumesControl;
use cosmic::app::Task;

/// Handle BTRFS management messages
pub(super) fn handle_btrfs_message(app: &mut AppModel, message: Message) -> Task<Message> {
    match message {
        Message::BtrfsLoadSubvolumes {
            block_path,
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
                    let subvol_list = btrfs_client.list_native_subvolumes(&block_path).await?;
                    Ok(subvol_list.subvolumes)
                },
                move |result: anyhow::Result<Vec<storage_types::BtrfsSubvolume>>| {
                    if let Err(ref e) = result {
                        tracing::error!("Failed to load BTRFS subvolumes: {:#}", e);
                    }
                    let result = result.map_err(|e| e.to_string());
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
            mount_point: _,
            path: _,
        }
        | Message::BtrfsDeleteSubvolumeConfirm {
            block_path,
            mount_point: _,
            path: _,
        } => Task::done(cosmic::Action::App(Message::LogicalViewRequested {
            device_path: Some(block_path),
        })),

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
                                used_space: Err(e.to_string()),
                            };
                        }
                    };
                    let result = btrfs_client
                        .get_usage(&mount_point)
                        .await
                        .map(|usage| usage.used_bytes)
                        .map_err(|e| e.to_string());

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
            let block_path = app
                .nav
                .active_data::<VolumesControl>()
                .and_then(|control| control.btrfs_state.as_ref())
                .and_then(|state| state.block_path.clone());
            let mount_point_for_async = mount_point.clone();
            Task::perform(
                async move {
                    let block_path = block_path.ok_or_else(|| {
                        anyhow::anyhow!(
                            "UDisks Btrfs support is unavailable for this filesystem. Install or enable the udisks2 Btrfs module."
                        )
                    })?;
                    let btrfs_client = BtrfsClient::new().await?;
                    let subvol_list = btrfs_client.list_native_subvolumes(&block_path).await?;
                    let default_id = subvol_list.default_id;
                    let default_subvol = subvol_list
                        .subvolumes
                        .into_iter()
                        .find(|s| s.id == default_id)
                        .ok_or_else(|| anyhow::anyhow!("Default subvolume not found"))?;
                    Ok(default_subvol)
                },
                move |result: anyhow::Result<storage_types::BtrfsSubvolume>| {
                    let result = result.map_err(|e| e.to_string());
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
            mount_point: _,
            subvolume_id: _,
        } => {
            let block_path = app
                .nav
                .active_data::<VolumesControl>()
                .and_then(|control| control.btrfs_state.as_ref())
                .and_then(|state| state.block_path.clone());

            block_path.map_or_else(Task::none, |device_path| {
                Task::done(cosmic::Action::App(Message::LogicalViewRequested {
                    device_path: Some(device_path),
                }))
            })
        }

        Message::BtrfsToggleReadonly {
            mount_point: _,
            subvolume_id: _,
        } => {
            let ctx = UiErrorContext::new("toggle_readonly");
            Task::done(
                log_error_and_show_dialog(
                    fl!("btrfs-readonly-failed"),
                    anyhow::anyhow!(
                        "UDisks does not expose a Polkit-authorized Btrfs readonly-subvolume operation."
                    ),
                    ctx,
                )
                .into(),
            )
        }

        Message::BtrfsReadonlyToggled {
            mount_point,
            result,
        } => match result {
            Ok(()) => {
                let block_path = app
                    .nav
                    .active_data::<VolumesControl>()
                    .and_then(|control| control.btrfs_state.as_ref())
                    .and_then(|state| state.block_path.clone());
                block_path.map_or_else(Task::none, |block_path| {
                    handle_btrfs_message(
                        app,
                        Message::BtrfsLoadSubvolumes {
                            block_path,
                            mount_point,
                        },
                    )
                })
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
        },

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
                    Err(anyhow::anyhow!(
                        "UDisks does not expose a Polkit-authorized Btrfs deleted-subvolume listing."
                    ))
                },
                move |result: anyhow::Result<Vec<storage_types::DeletedSubvolume>>| {
                    let result = result.map_err(|e| e.to_string());
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
            let block_path = app
                .nav
                .active_data::<VolumesControl>()
                .and_then(|control| control.btrfs_state.as_ref())
                .and_then(|state| state.block_path.clone());
            let Some(block_path) = block_path else {
                return Task::none();
            };
            // Reload all BTRFS data
            Task::batch(vec![
                handle_btrfs_message(
                    app,
                    Message::BtrfsLoadSubvolumes {
                        block_path,
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

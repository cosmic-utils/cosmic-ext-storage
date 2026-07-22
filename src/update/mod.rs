mod btrfs;
mod drive;
mod image;
mod nav;
mod network;
mod smart;
pub(crate) mod volumes;

use std::collections::HashSet;

use crate::app::APP_ID;
use crate::app::REPOSITORY;
use crate::config::{Config, LoggingLevel};
use crate::errors::ui::{UiErrorContext, log_error_and_show_dialog};
use crate::fl;
use crate::logging;
use crate::message::app::{ImagePathPickerKind, Message};
use crate::message::network::NetworkMessage;
use crate::models::load_all_drives;
use crate::operations::FilesystemsClient;
use crate::state::app::AppModel;
use crate::state::dialogs::ShowDialog;
use crate::state::sidebar::SidebarNodeKey;
use crate::state::volumes::{DetailTab, UsageTabState, VolumesControl};
use cosmic::app::Task;
use cosmic::cosmic_config::CosmicConfigEntry;
use cosmic::dialog::file_chooser;
use cosmic::widget::nav_bar;
use storage_types::{UsageCategory, UsageScanParallelismPreset};

const USAGE_TOP_FILES_MIN: u32 = 1;
const USAGE_TOP_FILES_MAX: u32 = 1000;

fn visible_usage_categories(result: &storage_types::UsageScanResult) -> Vec<UsageCategory> {
    result
        .categories
        .iter()
        .filter(|entry| entry.bytes > 0)
        .map(|entry| entry.category)
        .collect()
}

fn usage_filtered_file_paths(state: &UsageTabState) -> Vec<String> {
    let Some(result) = &state.result else {
        return Vec::new();
    };

    let selected: HashSet<UsageCategory> = state.selected_categories.iter().copied().collect();

    let mut files: Vec<(u64, String)> = result
        .categories
        .iter()
        .filter(|entry| entry.bytes > 0 && selected.contains(&entry.category))
        .flat_map(|entry| {
            result
                .top_files_by_category
                .iter()
                .find(|top| top.category == entry.category)
                .into_iter()
                .flat_map(|top| top.files.iter())
        })
        .map(|file| (file.bytes, file.path.to_string_lossy().to_string()))
        .collect();

    files.sort_by(|(left_bytes, left_path), (right_bytes, right_path)| {
        right_bytes
            .cmp(left_bytes)
            .then_with(|| left_path.cmp(right_path))
    });

    files.into_iter().map(|(_, path)| path).collect()
}

/// Find the segment index and whether the volume is a child for a given device path
/// Handles messages emitted by the application and its widgets.
pub(crate) fn update(app: &mut AppModel, message: Message) -> Task<Message> {
    match message {
        Message::OpenRepositoryUrl => {
            _ = open::that_detached(REPOSITORY);
        }
        Message::OpenPath(path) => {
            _ = open::that_detached(path);
        }
        Message::ToggleContextPage(context_page) => {
            if app.context_page == context_page {
                // Close the context drawer if the toggled context page is the same.
                app.core.window.show_context = !app.core.window.show_context;
            } else {
                // Open the context drawer to display the requested context page.
                app.context_page = context_page;
                app.core.window.show_context = true;
            }
        }
        Message::UpdateConfig(config) => {
            app.config = config;
        }
        Message::FilesystemToolsLoaded(tools) => {
            app.filesystem_tools = tools;
        }
        Message::UsageScanLoad {
            scan_id,
            top_files_per_category,
            mount_points,
            show_all_files,
            parallelism_preset,
        } => {
            return Task::perform(
                async move {
                    let result = match FilesystemsClient::new().await {
                        Ok(client) => client
                            .get_usage_scan(
                                &scan_id,
                                top_files_per_category,
                                &mount_points,
                                show_all_files,
                                parallelism_preset,
                            )
                            .await
                            .map_err(|e| e.to_string()),
                        Err(e) => Err(e.to_string()),
                    };

                    Message::UsageScanLoaded { scan_id, result }
                },
                |msg| msg.into(),
            );
        }
        Message::UsageScanLoaded { scan_id, result } => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>()
                && volumes_control
                    .usage_state
                    .active_scan_id
                    .as_ref()
                    .is_some_and(|active| active == &scan_id)
            {
                volumes_control.usage_state.loading = false;
                volumes_control.usage_state.active_scan_id = None;
                volumes_control.usage_state.operation_status = None;
                match result {
                    Ok(scan_result) => {
                        let visible_categories = visible_usage_categories(&scan_result);
                        volumes_control.usage_state.result = Some(scan_result);
                        volumes_control.usage_state.selected_categories = visible_categories;
                        volumes_control.usage_state.error = None;
                    }
                    Err(error) => {
                        volumes_control.usage_state.error = Some(error);
                        volumes_control.usage_state.result = None;
                    }
                }
            }
        }
        Message::UsageScanProgress {
            scan_id,
            processed_bytes,
            estimated_total_bytes,
        } => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>()
                && volumes_control
                    .usage_state
                    .active_scan_id
                    .as_ref()
                    .is_some_and(|active| active == &scan_id)
            {
                volumes_control.usage_state.progress_processed_bytes = processed_bytes;
                volumes_control.usage_state.progress_estimated_total_bytes = estimated_total_bytes;
            }
        }
        Message::UsageCategoryFilterToggled(category) => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                let selected_categories = &mut volumes_control.usage_state.selected_categories;
                if let Some(index) = selected_categories
                    .iter()
                    .position(|selected| selected == &category)
                {
                    if selected_categories.len() > 1 {
                        selected_categories.remove(index);
                    }
                } else {
                    selected_categories.push(category);
                }
                volumes_control.usage_state.selected_paths.clear();
                volumes_control.usage_state.selection_anchor_index = None;
            }
        }
        Message::UsageShowAllFilesToggled(show_all_files) => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                if !show_all_files {
                    volumes_control.usage_state.show_all_files = false;
                    return Task::done(cosmic::Action::App(Message::UsageRefreshRequested));
                }

                if volumes_control
                    .usage_state
                    .show_all_files_authorized_for_session
                {
                    volumes_control.usage_state.show_all_files = true;
                    return Task::done(cosmic::Action::App(Message::UsageRefreshRequested));
                }

                volumes_control.usage_state.show_all_files = false;

                return Task::perform(
                    async move {
                        let result = match FilesystemsClient::new().await {
                            Ok(client) => client
                                .authorize_usage_show_all_files()
                                .await
                                .map(|_| ())
                                .map_err(|e| e.to_string()),
                            Err(e) => Err(e.to_string()),
                        };

                        Message::UsageShowAllFilesAuthCompleted { result }
                    },
                    |msg| msg.into(),
                );
            }
        }
        Message::UsageShowAllFilesAuthCompleted { result } => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                match result {
                    Ok(()) => {
                        volumes_control
                            .usage_state
                            .show_all_files_authorized_for_session = true;
                        volumes_control.usage_state.show_all_files = true;
                        volumes_control.usage_state.error = None;
                        return Task::done(cosmic::Action::App(Message::UsageRefreshRequested));
                    }
                    Err(error) => {
                        volumes_control.usage_state.show_all_files = false;
                        volumes_control.usage_state.error = Some(error);
                    }
                }
            }
        }
        Message::UsageTopFilesPerCategoryChanged(top_files_per_category) => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                volumes_control.usage_state.top_files_per_category =
                    top_files_per_category.clamp(USAGE_TOP_FILES_MIN, USAGE_TOP_FILES_MAX);
            }
        }
        Message::UsageRefreshRequested => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                if volumes_control.usage_state.loading {
                    return Task::none();
                }

                if volumes_control.usage_state.scan_mount_points.is_empty() {
                    return Task::done(cosmic::Action::App(Message::UsageConfigureRequested));
                }

                let scan_id = format!(
                    "usage-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|duration| duration.as_millis())
                        .unwrap_or(0)
                );

                let mount_points = volumes_control.usage_state.scan_mount_points.clone();
                let show_all_files = volumes_control.usage_state.show_all_files;
                let parallelism_preset = volumes_control.usage_state.scan_parallelism_preset;

                volumes_control.usage_state.loading = true;
                volumes_control.usage_state.progress_processed_bytes = 0;
                volumes_control.usage_state.progress_estimated_total_bytes = 0;
                volumes_control.usage_state.active_scan_id = Some(scan_id.clone());
                volumes_control.usage_state.error = None;
                volumes_control.usage_state.operation_status = None;
                volumes_control.usage_state.result = None;
                volumes_control.usage_state.selected_paths.clear();
                volumes_control.usage_state.selection_anchor_index = None;

                return Task::done(cosmic::Action::App(Message::UsageScanLoad {
                    scan_id,
                    top_files_per_category: volumes_control.usage_state.top_files_per_category,
                    mount_points,
                    show_all_files,
                    parallelism_preset,
                }));
            }
        }
        Message::UsageConfigureRequested => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                if volumes_control.usage_state.loading {
                    return Task::none();
                }

                volumes_control.usage_state.wizard_open = true;
                volumes_control.usage_state.wizard_loading_mounts = true;
                volumes_control.usage_state.wizard_error = None;
                volumes_control.usage_state.wizard_show_all_files =
                    volumes_control.usage_state.show_all_files;
                volumes_control.usage_state.wizard_parallelism_preset =
                    if volumes_control.usage_state.scan_mount_points.is_empty() {
                        app.config.usage_scan_parallelism
                    } else {
                        volumes_control.usage_state.scan_parallelism_preset
                    };

                return Task::perform(
                    async move {
                        let result = match FilesystemsClient::new().await {
                            Ok(client) => client
                                .list_usage_mount_points()
                                .await
                                .map_err(|e| e.to_string()),
                            Err(e) => Err(e.to_string()),
                        };

                        Message::UsageWizardMountPointsLoaded { result }
                    },
                    |msg| msg.into(),
                );
            }
        }
        Message::UsageWizardMountPointsLoaded { result } => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                volumes_control.usage_state.wizard_loading_mounts = false;
                match result {
                    Ok(mut mount_points) => {
                        mount_points.sort_unstable();
                        mount_points.dedup();
                        volumes_control.usage_state.wizard_mount_points = mount_points.clone();
                        volumes_control.usage_state.wizard_selected_mount_points =
                            if volumes_control.usage_state.scan_mount_points.is_empty() {
                                mount_points
                            } else {
                                volumes_control
                                    .usage_state
                                    .scan_mount_points
                                    .iter()
                                    .filter(|mount| mount_points.contains(mount))
                                    .cloned()
                                    .collect()
                            };
                        volumes_control.usage_state.wizard_error = None;
                    }
                    Err(error) => {
                        volumes_control.usage_state.wizard_mount_points.clear();
                        volumes_control
                            .usage_state
                            .wizard_selected_mount_points
                            .clear();
                        volumes_control.usage_state.wizard_error = Some(error);
                    }
                }
            }
        }
        Message::UsageWizardMountToggled {
            mount_point,
            selected,
        } => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                if selected {
                    if !volumes_control
                        .usage_state
                        .wizard_selected_mount_points
                        .iter()
                        .any(|mount| mount == &mount_point)
                    {
                        volumes_control
                            .usage_state
                            .wizard_selected_mount_points
                            .push(mount_point);
                    }
                } else {
                    volumes_control
                        .usage_state
                        .wizard_selected_mount_points
                        .retain(|mount| mount != &mount_point);
                }
            }
        }
        Message::UsageWizardShowAllFilesToggled(show_all_files) => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                volumes_control.usage_state.wizard_show_all_files = show_all_files;
            }
        }
        Message::UsageWizardParallelismChanged(index) => {
            let preset = UsageScanParallelismPreset::from_index(index);

            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                volumes_control.usage_state.wizard_parallelism_preset = preset;
                volumes_control.usage_state.scan_parallelism_preset = preset;
            }

            app.config.usage_scan_parallelism = preset;
            if let Ok(helper) = cosmic::cosmic_config::Config::new(APP_ID, Config::VERSION) {
                let _ = app.config.write_entry(&helper);
            }
        }
        Message::UsageWizardCancel => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                volumes_control.usage_state.wizard_open = false;
                volumes_control.usage_state.wizard_loading_mounts = false;
                volumes_control.usage_state.wizard_error = None;
            }
        }
        Message::UsageWizardStartScan => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                if volumes_control.usage_state.wizard_loading_mounts {
                    return Task::none();
                }

                if volumes_control
                    .usage_state
                    .wizard_selected_mount_points
                    .is_empty()
                {
                    volumes_control.usage_state.wizard_error =
                        Some(fl!("usage-select-at-least-one-mount-point"));
                    return Task::none();
                }

                if volumes_control.usage_state.wizard_show_all_files
                    && !volumes_control
                        .usage_state
                        .show_all_files_authorized_for_session
                {
                    return Task::perform(
                        async move {
                            let result = match FilesystemsClient::new().await {
                                Ok(client) => client
                                    .authorize_usage_show_all_files()
                                    .await
                                    .map(|_| ())
                                    .map_err(|e| e.to_string()),
                                Err(e) => Err(e.to_string()),
                            };

                            Message::UsageWizardShowAllFilesAuthCompleted { result }
                        },
                        |msg| msg.into(),
                    );
                }

                let scan_id = format!(
                    "usage-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|duration| duration.as_millis())
                        .unwrap_or(0)
                );

                let mount_points = volumes_control
                    .usage_state
                    .wizard_selected_mount_points
                    .clone();
                let show_all_files = volumes_control.usage_state.wizard_show_all_files;
                let parallelism_preset = volumes_control.usage_state.wizard_parallelism_preset;

                volumes_control.usage_state.wizard_open = false;
                volumes_control.usage_state.wizard_loading_mounts = false;
                volumes_control.usage_state.wizard_error = None;
                volumes_control.usage_state.show_all_files = show_all_files;
                volumes_control.usage_state.scan_mount_points = mount_points.clone();
                volumes_control.usage_state.scan_parallelism_preset = parallelism_preset;

                volumes_control.usage_state.loading = true;
                volumes_control.usage_state.progress_processed_bytes = 0;
                volumes_control.usage_state.progress_estimated_total_bytes = 0;
                volumes_control.usage_state.active_scan_id = Some(scan_id.clone());
                volumes_control.usage_state.error = None;
                volumes_control.usage_state.operation_status = None;
                volumes_control.usage_state.result = None;
                volumes_control.usage_state.selected_paths.clear();
                volumes_control.usage_state.selection_anchor_index = None;

                return Task::done(cosmic::Action::App(Message::UsageScanLoad {
                    scan_id,
                    top_files_per_category: volumes_control.usage_state.top_files_per_category,
                    mount_points,
                    show_all_files,
                    parallelism_preset,
                }));
            }
        }
        Message::UsageWizardShowAllFilesAuthCompleted { result } => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                match result {
                    Ok(()) => {
                        volumes_control
                            .usage_state
                            .show_all_files_authorized_for_session = true;
                        volumes_control.usage_state.wizard_error = None;
                        return Task::done(cosmic::Action::App(Message::UsageWizardStartScan));
                    }
                    Err(error) => {
                        volumes_control.usage_state.wizard_error = Some(error);
                    }
                }
            }
        }
        Message::UsageSelectionSingle { path, index } => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                volumes_control.usage_state.selected_paths = vec![path];
                volumes_control.usage_state.selection_anchor_index = Some(index);
            }
        }
        Message::UsageSelectionCtrl { path, index } => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                if let Some(existing_index) = volumes_control
                    .usage_state
                    .selected_paths
                    .iter()
                    .position(|selected_path| selected_path == &path)
                {
                    volumes_control
                        .usage_state
                        .selected_paths
                        .remove(existing_index);
                } else {
                    volumes_control.usage_state.selected_paths.push(path);
                }
                volumes_control.usage_state.selection_anchor_index = Some(index);
            }
        }
        Message::UsageSelectionShift { index } => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                let paths = usage_filtered_file_paths(&volumes_control.usage_state);

                if paths.is_empty() {
                    return Task::none();
                }

                let clamped_index = index.min(paths.len().saturating_sub(1));
                let anchor_index = volumes_control
                    .usage_state
                    .selection_anchor_index
                    .unwrap_or(clamped_index)
                    .min(paths.len().saturating_sub(1));
                let start = anchor_index.min(clamped_index);
                let end = anchor_index.max(clamped_index);

                volumes_control.usage_state.selected_paths = paths[start..=end].to_vec();
                volumes_control.usage_state.selection_anchor_index = Some(anchor_index);
            }
        }
        Message::UsageSelectionModifiersChanged(modifiers) => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                volumes_control.usage_state.selection_modifiers = modifiers;
            }
        }
        Message::UsageSelectionClear => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                volumes_control.usage_state.selected_paths.clear();
                volumes_control.usage_state.selection_anchor_index = None;
            }
        }
        Message::UsageDeleteStart => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                if volumes_control.usage_state.deleting
                    || volumes_control.usage_state.selected_paths.is_empty()
                {
                    return Task::none();
                }

                volumes_control.usage_state.deleting = true;
                volumes_control.usage_state.operation_status = None;
                let selected_paths = volumes_control.usage_state.selected_paths.clone();

                return Task::perform(
                    async move {
                        let result = match FilesystemsClient::new().await {
                            Ok(client) => client
                                .delete_usage_files(&selected_paths)
                                .await
                                .map_err(|e| e.to_string()),
                            Err(e) => Err(e.to_string()),
                        };

                        Message::UsageDeleteCompleted { result }
                    },
                    |msg| msg.into(),
                );
            }
        }
        Message::UsageDeleteCompleted { result } => {
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                volumes_control.usage_state.deleting = false;
                match result {
                    Ok(delete_result) => {
                        let deleted_paths: HashSet<String> =
                            delete_result.deleted.iter().cloned().collect();

                        if !deleted_paths.is_empty()
                            && let Some(scan_result) = volumes_control.usage_state.result.as_mut()
                        {
                            let mut removed_by_category: std::collections::BTreeMap<
                                storage_types::UsageCategory,
                                u64,
                            > = std::collections::BTreeMap::new();

                            for category_entry in &mut scan_result.top_files_by_category {
                                let mut removed_bytes = 0_u64;
                                category_entry.files.retain(|file| {
                                    let path = file.path.to_string_lossy();
                                    if deleted_paths.contains(path.as_ref()) {
                                        removed_bytes = removed_bytes.saturating_add(file.bytes);
                                        false
                                    } else {
                                        true
                                    }
                                });

                                if removed_bytes > 0 {
                                    removed_by_category
                                        .entry(category_entry.category)
                                        .and_modify(|bytes| {
                                            *bytes = bytes.saturating_add(removed_bytes)
                                        })
                                        .or_insert(removed_bytes);
                                }
                            }

                            for category_total in &mut scan_result.categories {
                                if let Some(removed_bytes) =
                                    removed_by_category.get(&category_total.category)
                                {
                                    category_total.bytes =
                                        category_total.bytes.saturating_sub(*removed_bytes);
                                }
                            }

                            let removed_total: u64 = removed_by_category.values().copied().sum();
                            if removed_total > 0 {
                                scan_result.total_bytes =
                                    scan_result.total_bytes.saturating_sub(removed_total);
                            }

                            let failed_paths: HashSet<String> = delete_result
                                .failed
                                .iter()
                                .map(|failure| failure.path.clone())
                                .collect();

                            volumes_control
                                .usage_state
                                .selected_paths
                                .retain(|path| failed_paths.contains(path));

                            let current_category_paths =
                                usage_filtered_file_paths(&volumes_control.usage_state);

                            volumes_control.usage_state.selection_anchor_index =
                                volumes_control.usage_state.selected_paths.first().and_then(
                                    |first_selected| {
                                        current_category_paths
                                            .iter()
                                            .position(|path| path == first_selected)
                                    },
                                );
                        }

                        volumes_control.usage_state.error = None;
                        volumes_control.usage_state.operation_status = Some(fl!(
                            "usage-delete-summary",
                            deleted = delete_result.deleted.len(),
                            failed = delete_result.failed.len(),
                        ));
                    }
                    Err(error) => {
                        volumes_control.usage_state.error = Some(error);
                        volumes_control.usage_state.operation_status = None;
                    }
                }
            }
        }
        Message::ToggleShowReserved(show_reserved) => {
            app.config.show_reserved = show_reserved;

            // Persist config change
            if let Ok(helper) = cosmic::cosmic_config::Config::new(APP_ID, Config::VERSION) {
                let _ = app.config.write_entry(&helper);
            }

            // Update the active volumes control if one is selected
            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                volumes_control.set_show_reserved(show_reserved);
            }
        }
        Message::UsageScanParallelismChanged(index) => {
            let preset = UsageScanParallelismPreset::from_index(index);
            app.config.usage_scan_parallelism = preset;

            if let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() {
                volumes_control.usage_state.wizard_parallelism_preset = preset;
                volumes_control.usage_state.scan_parallelism_preset = preset;
            }

            if let Ok(helper) = cosmic::cosmic_config::Config::new(APP_ID, Config::VERSION) {
                let _ = app.config.write_entry(&helper);
            }
        }
        Message::ToggleLogToDisk(log_to_disk) => {
            app.config.log_to_disk = log_to_disk;

            if let Ok(helper) = cosmic::cosmic_config::Config::new(APP_ID, Config::VERSION) {
                let _ = app.config.write_entry(&helper);
            }

            logging::set_log_to_disk(log_to_disk);
        }
        Message::LogLevelChanged(index) => {
            let level = LoggingLevel::from_index(index);
            app.config.log_level = level;

            if let Ok(helper) = cosmic::cosmic_config::Config::new(APP_ID, Config::VERSION) {
                let _ = app.config.write_entry(&helper);
            }

            logging::set_log_level(level);
        }
        Message::OpenImagePathPicker(kind) => {
            let title = match kind {
                ImagePathPickerKind::NewDiskImage | ImagePathPickerKind::ImageOperationCreate => {
                    fl!("image-destination-path")
                }
                ImagePathPickerKind::AttachDiskImage
                | ImagePathPickerKind::ImageOperationRestore => fl!("image-file-path"),
            };

            return Task::perform(
                async move {
                    let result = match kind {
                        ImagePathPickerKind::NewDiskImage
                        | ImagePathPickerKind::ImageOperationCreate => {
                            let dialog = file_chooser::save::Dialog::new().title(title);
                            match dialog.save_file().await {
                                Ok(response) => response
                                    .url()
                                    .and_then(|url| url.to_file_path().ok())
                                    .map(|path| path.to_string_lossy().to_string()),
                                Err(file_chooser::Error::Cancelled) => None,
                                Err(err) => {
                                    tracing::warn!(?err, "save file dialog failed");
                                    None
                                }
                            }
                        }
                        ImagePathPickerKind::AttachDiskImage
                        | ImagePathPickerKind::ImageOperationRestore => {
                            let dialog = file_chooser::open::Dialog::new().title(title);
                            match dialog.open_file().await {
                                Ok(response) => response
                                    .url()
                                    .to_file_path()
                                    .ok()
                                    .map(|path| path.to_string_lossy().to_string()),
                                Err(file_chooser::Error::Cancelled) => None,
                                Err(err) => {
                                    tracing::warn!(?err, "open file dialog failed");
                                    None
                                }
                            }
                        }
                    };

                    Message::ImagePathPicked(kind, result)
                },
                |msg| msg.into(),
            );
        }
        Message::ImagePathPicked(kind, path) => match kind {
            ImagePathPickerKind::NewDiskImage => {
                if let Some(ShowDialog::NewDiskImage(state)) = app.dialog.as_mut()
                    && let Some(path) = path
                {
                    state.path = path;
                }
            }
            ImagePathPickerKind::AttachDiskImage => {
                if let Some(ShowDialog::AttachDiskImage(state)) = app.dialog.as_mut()
                    && let Some(path) = path
                {
                    state.path = path;
                }
            }
            ImagePathPickerKind::ImageOperationCreate
            | ImagePathPickerKind::ImageOperationRestore => {
                if let Some(ShowDialog::ImageOperation(state)) = app.dialog.as_mut()
                    && let Some(path) = path
                {
                    state.image_path = path;
                }
            }
        },
        Message::LaunchUrl(url) => match open::that_detached(&url) {
            Ok(()) => {}
            Err(err) => {
                tracing::warn!(?url, %err, "failed to open url");
            }
        },
        Message::VolumesMessage(message) => {
            let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() else {
                tracing::warn!("received volumes message with no active VolumesControl");
                return Task::none();
            };

            return volumes_control.update(message, &mut app.dialog);
        }

        Message::FormatDisk(msg) => {
            return drive::format_disk(app, msg);
        }
        Message::DriveRemoved(_drive_model) => {
            return Task::perform(
                async {
                    match load_all_drives().await {
                        Ok(drives) => Some(drives),
                        Err(e) => {
                            tracing::error!(%e, "failed to refresh drives after drive removal");
                            None
                        }
                    }
                },
                move |drives| match drives {
                    None => Message::None.into(),
                    Some(drives) => Message::UpdateNav(drives, None).into(),
                },
            );
        }
        Message::DriveAdded(_drive_model) => {
            return Task::perform(
                async {
                    match load_all_drives().await {
                        Ok(drives) => Some(drives),
                        Err(e) => {
                            tracing::error!(%e, "failed to refresh drives after drive add");
                            None
                        }
                    }
                },
                move |drives| match drives {
                    None => Message::None.into(),
                    Some(drives) => Message::UpdateNav(drives, None).into(),
                },
            );
        }
        Message::None => {}
        Message::UpdateNav(drive_models, selected) => {
            return nav::update_nav(app, drive_models, selected);
        }

        // BTRFS management
        Message::BtrfsLoadSubvolumes { .. }
        | Message::BtrfsSubvolumesLoaded { .. }
        | Message::BtrfsDeleteSubvolume { .. }
        | Message::BtrfsDeleteSubvolumeConfirm { .. }
        | Message::BtrfsLoadUsage { .. }
        | Message::BtrfsUsageLoaded { .. }
        | Message::BtrfsToggleSubvolumeExpanded { .. }
        | Message::BtrfsLoadDefaultSubvolume { .. }
        | Message::BtrfsDefaultSubvolumeLoaded { .. }
        | Message::BtrfsSetDefaultSubvolume { .. }
        | Message::BtrfsToggleReadonly { .. }
        | Message::BtrfsReadonlyToggled { .. }
        | Message::BtrfsShowProperties { .. }
        | Message::BtrfsCloseProperties { .. }
        | Message::BtrfsLoadDeletedSubvolumes { .. }
        | Message::BtrfsDeletedSubvolumesLoaded { .. }
        | Message::BtrfsToggleShowDeleted { .. }
        | Message::BtrfsRefreshAll { .. } => {
            return btrfs::handle_btrfs_message(app, message);
        }

        Message::UpdateNavWithChildSelection(drive_models, child_device_path) => {
            // Preserve tab selection and BTRFS state before nav rebuild
            let saved_state = app
                .nav
                .active_data::<VolumesControl>()
                .map(|v| (v.detail_tab, v.btrfs_state.clone(), v.usage_state.clone()));

            // Update drives while preserving child volume selection
            let task = nav::update_nav(app, drive_models, None);

            // Restore child selection if provided
            if let Some(device_path) = child_device_path {
                app.sidebar.selected_child = Some(crate::state::sidebar::SidebarNodeKey::Volume(
                    device_path.clone(),
                ));

                if let Some(control) = app.nav.active_data_mut::<VolumesControl>()
                    && let Some((segment_idx, is_child)) =
                        crate::state::volumes::find_segment_for_volume(control, &device_path)
                {
                    control.selected_volume = if is_child {
                        Some(device_path.clone())
                    } else {
                        None
                    };

                    control.segments.iter_mut().for_each(|s| s.state = false);
                    control.selected_segment = segment_idx;
                    if let Some(segment) = control.segments.get_mut(segment_idx) {
                        segment.state = true;
                    }

                    // Restore preserved tab selection and BTRFS state
                    if let Some((saved_tab, saved_btrfs, saved_usage)) = saved_state {
                        control.detail_tab = saved_tab;
                        control.btrfs_state = saved_btrfs;
                        control.usage_state = saved_usage;

                        // Refresh BTRFS data if on BTRFS tab
                        if saved_tab == DetailTab::BtrfsManagement
                            && let Some(btrfs_state) = &control.btrfs_state
                            && let (Some(mount_point), Some(block_path)) =
                                (&btrfs_state.mount_point, &btrfs_state.block_path)
                        {
                            let refresh_task = Task::batch(vec![
                                Task::done(
                                    Message::BtrfsLoadSubvolumes {
                                        block_path: block_path.clone(),
                                        mount_point: mount_point.clone(),
                                    }
                                    .into(),
                                ),
                                Task::done(
                                    Message::BtrfsLoadUsage {
                                        block_path: block_path.clone(),
                                        mount_point: mount_point.clone(),
                                    }
                                    .into(),
                                ),
                            ]);
                            return task.chain(refresh_task);
                        }
                    }
                }
            }

            return task;
        }
        Message::Dialog(show_dialog) => app.dialog = Some(*show_dialog),
        Message::CloseDialog => {
            app.dialog = None;
        }
        Message::Eject => {
            return drive::eject(app);
        }
        Message::PowerOff => {
            return drive::power_off(app);
        }
        Message::Format => {
            drive::format(app);
        }
        Message::SmartData => {
            return drive::smart_data(app);
        }
        Message::StandbyNow => {
            return drive::standby_now(app);
        }
        Message::Wakeup => {
            return drive::wakeup(app);
        }

        // Sidebar (custom treeview)
        Message::SidebarSelectDrive { device_path } => {
            app.network.select(None, None);
            app.network.clear_editor();
            app.sidebar.selected_child = None;
            if let Some(id) = app.sidebar.drive_entities.get(&device_path).copied() {
                return on_nav_select(app, id);
            }
        }
        Message::SidebarClearChildSelection => {
            app.sidebar.selected_child = None;
        }
        Message::SidebarSelectChild { device_path } => {
            app.network.select(None, None);
            app.network.clear_editor();
            app.sidebar.selected_child = Some(SidebarNodeKey::Volume(device_path.clone()));

            // Find which drive contains this volume node
            let drive_for_volume = app
                .sidebar
                .drives
                .iter()
                .find(|d| {
                    crate::state::volumes::find_volume_in_ui_tree(&d.volumes, &device_path)
                        .is_some()
                })
                .cloned();

            // If the volume belongs to a different drive, switch to that drive first
            if let Some(drive) = drive_for_volume {
                let current_drive_device_path = app.sidebar.active_drive_block_path(&app.nav);
                if current_drive_device_path.as_deref() != Some(drive.device()) {
                    // Switch to the correct drive
                    if let Some(id) = app.sidebar.drive_entities.get(drive.device()).copied() {
                        let switch_task = on_nav_select(app, id);
                        // After switching, we need to select the volume again
                        return switch_task.chain(Task::done(cosmic::Action::App(
                            Message::SidebarSelectChild { device_path },
                        )));
                    }
                }
            }

            // Sync with volumes control: select the corresponding volume
            let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>() else {
                return Task::none();
            };

            let Some(vol_node) = crate::state::volumes::find_volume_in_ui_tree(
                &volumes_control.volumes,
                &device_path,
            ) else {
                return Task::none();
            };

            let Some((segment_idx, is_child)) =
                crate::state::volumes::find_segment_for_volume(volumes_control, &device_path)
            else {
                return Task::none();
            };

            // Apply the selection change
            volumes_control.selected_segment = segment_idx;
            volumes_control.selected_volume = if is_child {
                vol_node.device_path()
            } else {
                None
            };

            // Update segment state
            volumes_control
                .segments
                .iter_mut()
                .for_each(|s| s.state = false);
            if let Some(segment) = volumes_control.segments.get_mut(segment_idx) {
                segment.state = true;
            }
        }
        Message::SidebarToggleExpanded(key) => {
            app.sidebar.toggle_expanded(key);
        }
        Message::SidebarDriveEject { device_path } => {
            if let Some(drive) = app.sidebar.find_drive(&device_path) {
                return drive::eject_drive(drive.clone());
            }
        }
        Message::SidebarVolumeUnmount { drive, device_path } => {
            let Some(drive_model) = app.sidebar.find_drive(&drive) else {
                return Task::none();
            };

            let Some(node) =
                crate::state::volumes::find_volume_in_ui_tree(&drive_model.volumes, &device_path)
            else {
                return Task::none();
            };

            let Some(device_to_unmount) = node.device().map(|s| s.to_string()) else {
                return Task::none();
            };
            let device_path_for_closure = device_path.clone();
            let device = drive_model.device().to_string();

            return Task::perform(
                async move {
                    let fs_client = FilesystemsClient::new().await.map_err(|e| {
                        anyhow::anyhow!("Failed to create filesystems client: {}", e)
                    })?;
                    fs_client
                        .unmount(&device_to_unmount, false, false)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to unmount: {}", e))?;
                    load_all_drives()
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to reload drives: {}", e))
                },
                move |res| match res {
                    Ok(drives) => Message::UpdateNav(drives, None).into(),
                    Err(e) => {
                        let ctx = UiErrorContext {
                            operation: "sidebar_volume_unmount",
                            device_path: Some(device_path_for_closure.as_str()),
                            device: Some(device.as_str()),
                            drive_path: None,
                        };
                        log_error_and_show_dialog(fl!("unmount-failed"), e, ctx).into()
                    }
                },
            );
        }
        Message::SmartDialog(msg) => {
            return smart::smart_dialog(app, msg);
        }
        Message::NewDiskImage => {
            image::new_disk_image(app);
        }
        Message::AttachDisk => {
            image::attach_disk(app);
        }
        Message::CreateDiskFrom => {
            return image::create_disk_from(app);
        }
        Message::RestoreImageTo => {
            return image::restore_image_to(app);
        }
        Message::CreateDiskFromPartition => {
            return image::create_disk_from_partition(app);
        }
        Message::RestoreImageToPartition => {
            return image::restore_image_to_partition(app);
        }
        Message::NewDiskImageDialog(msg) => {
            return image::new_disk_image_dialog(app, msg);
        }
        Message::AttachDiskImageDialog(msg) => {
            return image::attach_disk_image_dialog(app, msg);
        }
        Message::ImageOperationDialog(msg) => {
            return image::image_operation_dialog(app, msg);
        }
        Message::ImageOperationStarted(operation_id) => {
            app.image_op_operation_id = Some(operation_id.clone());
            if let Some(ShowDialog::ImageOperation(state)) = app.dialog.as_mut() {
                state.operation_id = Some(operation_id);
            }
        }
        Message::UnmountBusy(msg) => {
            use crate::message::dialogs::UnmountBusyMessage;

            // Extract dialog data before consuming it
            let dialog_data = if let Some(ShowDialog::UnmountBusy(ref dialog)) = app.dialog {
                Some((
                    dialog.device_path.clone(),
                    dialog.mount_point.clone(),
                    dialog.processes.iter().map(|p| p.pid).collect::<Vec<_>>(),
                ))
            } else {
                None
            };

            match msg {
                UnmountBusyMessage::Cancel => {
                    tracing::debug!("User cancelled unmount busy dialog");
                    app.dialog = None;
                }
                UnmountBusyMessage::Retry => {
                    tracing::info!(
                        device_path = dialog_data
                            .as_ref()
                            .map(|(dp, _, _)| dp.as_str())
                            .unwrap_or("unknown"),
                        "User requested unmount retry"
                    );
                    app.dialog = None;

                    if let Some((device_path, _, _)) = dialog_data {
                        // Retry the unmount operation
                        if let Some(volumes) = app.nav.active_data::<VolumesControl>() {
                            return retry_unmount(volumes, device_path);
                        }
                    }
                }
                UnmountBusyMessage::KillAndRetry => {
                    if let Some((device, mount_point, _pids)) = dialog_data {
                        app.dialog = None;
                        tracing::info!(
                            device = %device,
                            "User requested kill processes and unmount"
                        );

                        let device_path_for_selection = device.clone();
                        return Task::perform(
                            async move {
                                let fs_client = match FilesystemsClient::new().await {
                                    Ok(c) => c,
                                    Err(e) => {
                                        tracing::error!(?e, "Failed to create filesystems client");
                                        return Err(None);
                                    }
                                };
                                // Unmount with kill_processes=true so the service kills blocking processes
                                let unmount_result =
                                    match fs_client.unmount(&device, false, true).await {
                                        Ok(r) => r,
                                        Err(e) => {
                                            tracing::error!(?e, "Failed to unmount with kill");
                                            return Err(None);
                                        }
                                    };
                                if unmount_result.success {
                                    match load_all_drives().await {
                                        Ok(drives) => Ok(drives),
                                        Err(e) => {
                                            tracing::error!(
                                                ?e,
                                                "Failed to reload drives after unmount"
                                            );
                                            Err(None)
                                        }
                                    }
                                } else if !unmount_result.blocking_processes.is_empty() {
                                    let device_for_tuple = device.clone();
                                    Err(Some((
                                        device_for_tuple,
                                        mount_point,
                                        unmount_result.blocking_processes,
                                        device,
                                    )))
                                } else {
                                    if let Some(err) = unmount_result.error {
                                        tracing::error!("unmount with kill failed: {}", err);
                                    }
                                    Err(None)
                                }
                            },
                            move |result| match result {
                                Ok(drives) => Message::UpdateNavWithChildSelection(
                                    drives,
                                    Some(device_path_for_selection.clone()),
                                )
                                .into(),
                                Err(Some((device, mount_point, processes, device_path))) => {
                                    Message::Dialog(Box::new(ShowDialog::UnmountBusy(
                                        crate::state::dialogs::UnmountBusyDialog {
                                            device,
                                            mount_point,
                                            processes,
                                            device_path,
                                        },
                                    )))
                                    .into()
                                }
                                Err(None) => Message::None.into(),
                            },
                        );
                    } else {
                        app.dialog = None;
                    }
                }
            }
        }
        Message::RetryUnmountAfterKill(device_path) => {
            tracing::debug!("Retrying unmount after killing processes");
            if let Some(volumes) = app.nav.active_data::<VolumesControl>() {
                return retry_unmount(volumes, device_path);
            }
        }

        // Network mounts (RClone, Samba, FTP)
        Message::Network(msg) => {
            return network::handle_network_message(app, msg);
        }
        Message::LoadNetworkRemotes => {
            return network::handle_network_message(app, NetworkMessage::LoadRemotes);
        }
        Message::NetworkRemotesLoaded(result) => {
            return network::handle_network_message(app, NetworkMessage::RemotesLoaded(result));
        }
    }
    Task::none()
}

/// Called when a nav item is selected.
pub(crate) fn on_nav_select(app: &mut AppModel, id: nav_bar::Id) -> Task<Message> {
    // Activate the page in the model.
    if app.dialog.is_none() {
        let previous_show_reserved = app
            .nav
            .active_data::<VolumesControl>()
            .map(|v| v.show_reserved);

        app.nav.activate(id);

        if let Some(show_reserved) = previous_show_reserved
            && let Some(volumes_control) = app.nav.active_data_mut::<VolumesControl>()
        {
            volumes_control.set_show_reserved(show_reserved);
        }

        app.update_title()
    } else {
        Task::none()
    }
}

/// Helper function to retry unmount operation on a volume by device path
fn retry_unmount(volumes: &VolumesControl, device_path: String) -> Task<Message> {
    // Find the volume node
    let node =
        crate::state::volumes::find_volume_in_ui_tree(&volumes.volumes, &device_path).cloned();

    if let Some(node) = node {
        let device = node
            .volume
            .device_path
            .clone()
            .unwrap_or_else(|| device_path.clone());
        let mount_point = node.volume.mount_points.first().cloned();
        let device_path_for_retry = device_path.clone();
        let device_path_for_selection = device_path.clone();

        Task::perform(
            async move {
                let fs_client = match FilesystemsClient::new().await {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!(?e, "Failed to create filesystems client");
                        return Err(None);
                    }
                };

                let unmount_result = match fs_client.unmount(&device, false, false).await {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::error!(?e, "Failed to unmount");
                        return Err(None);
                    }
                };

                if unmount_result.success {
                    // Success - reload drives
                    match load_all_drives().await {
                        Ok(drives) => Ok(drives),
                        Err(e) => {
                            tracing::error!(?e, "Failed to reload drives after unmount");
                            Err(None)
                        }
                    }
                } else if !unmount_result.blocking_processes.is_empty() {
                    // Device is busy with processes
                    let mp = mount_point.unwrap_or_default();
                    tracing::warn!(
                        mount_point = %mp,
                        process_count = unmount_result.blocking_processes.len(),
                        "Unmount still busy after retry"
                    );
                    Err(Some((
                        device,
                        mp,
                        unmount_result.blocking_processes,
                        device_path_for_retry,
                    )))
                } else {
                    // Generic error
                    if let Some(err) = unmount_result.error {
                        tracing::error!("unmount retry failed: {}", err);
                    } else {
                        tracing::error!("unmount retry failed with unknown error");
                    }
                    Err(None)
                }
            },
            move |result| match result {
                Ok(drives) => Message::UpdateNavWithChildSelection(
                    drives,
                    Some(device_path_for_selection.clone()),
                )
                .into(),
                Err(Some((device, mount_point, processes, device_path))) => {
                    // Still busy - show dialog again
                    Message::Dialog(Box::new(ShowDialog::UnmountBusy(
                        crate::state::dialogs::UnmountBusyDialog {
                            device,
                            mount_point,
                            processes,
                            device_path,
                        },
                    )))
                    .into()
                }
                Err(None) => {
                    // Generic error already logged
                    Message::None.into()
                }
            },
        )
    } else {
        tracing::warn!("Volume not found for retry: {}", device_path);
        Task::none()
    }
}

// SPDX-License-Identifier: GPL-3.0-only
#![allow(clippy::too_many_arguments)]

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use storage_types::{
    FilesystemToolInfo, FormatOptions, MountOptions, MountOptionsSettings, UnmountResult,
    UsageCategory, UsageDeleteFailure, UsageDeleteResult, UsageScanParallelismPreset,
    UsageScanResult,
};

use super::{OperationError, StorageOperations, protected_paths, shared};

#[derive(Clone, Debug)]
pub struct FilesystemsClient(Arc<StorageOperations>);

pub(crate) fn detect_filesystem_tools() -> Vec<FilesystemToolInfo> {
    [
        (
            "ext4",
            "EXT4",
            "mkfs.ext4",
            "e2fsprogs",
            cfg!(feature = "fs-ext4"),
        ),
        (
            "xfs",
            "XFS",
            "mkfs.xfs",
            "xfsprogs",
            cfg!(feature = "fs-xfs"),
        ),
        (
            "btrfs",
            "Btrfs",
            "mkfs.btrfs",
            "btrfs-progs",
            cfg!(feature = "fs-btrfs"),
        ),
        (
            "vfat",
            "FAT32",
            "mkfs.vfat",
            "dosfstools",
            cfg!(feature = "fs-vfat"),
        ),
        (
            "ntfs",
            "NTFS",
            "mkfs.ntfs",
            "ntfs-3g",
            cfg!(feature = "fs-ntfs"),
        ),
        (
            "exfat",
            "exFAT",
            "mkfs.exfat",
            "exfat-utils",
            cfg!(feature = "fs-exfat"),
        ),
    ]
    .into_iter()
    .map(
        |(fs_type, fs_name, command, package_hint, enabled)| FilesystemToolInfo {
            fs_type: fs_type.into(),
            fs_name: fs_name.into(),
            command: command.into(),
            package_hint: package_hint.into(),
            available: enabled && which::which(command).is_ok(),
        },
    )
    .collect()
}

fn scan_threads(preset: UsageScanParallelismPreset) -> usize {
    let cpus = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .max(1);
    match preset {
        UsageScanParallelismPreset::Low => cpus.div_ceil(4).max(1),
        UsageScanParallelismPreset::Balanced => cpus.div_ceil(2).max(1),
        UsageScanParallelismPreset::High => cpus,
    }
}

fn validate_selected_mounts(mounts: &[String]) -> Result<Vec<PathBuf>, OperationError> {
    if mounts.is_empty() {
        return Err(OperationError::InvalidInput(
            "At least one mount point must be selected".into(),
        ));
    }
    let available: HashSet<String> =
        storage_sys::usage::discover_local_mounts_under(Path::new("/"))
            .map_err(|error| OperationError::Failed(error.to_string()))?
            .into_iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect();
    mounts
        .iter()
        .map(|mount| {
            let path = PathBuf::from(mount);
            if !path.is_absolute() {
                return Err(OperationError::InvalidInput(
                    "Selected mount points must be absolute paths".into(),
                ));
            }
            if !available.contains(mount) {
                return Err(OperationError::InvalidInput(format!(
                    "Selected mount point is not local: {mount}"
                )));
            }
            Ok(path)
        })
        .collect()
}

fn filter_hidden_categories(result: &mut UsageScanResult) {
    for category in &mut result.categories {
        if matches!(
            category.category,
            UsageCategory::System | UsageCategory::Packages
        ) {
            category.bytes = 0;
        }
    }
    for category in &mut result.top_files_by_category {
        if matches!(
            category.category,
            UsageCategory::System | UsageCategory::Packages
        ) {
            category.files.clear();
        }
    }
    result.categories.sort_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then_with(|| left.category.cmp(&right.category))
    });
    result.total_bytes = result
        .categories
        .iter()
        .map(|category| category.bytes)
        .sum();
}

#[allow(dead_code)]
impl FilesystemsClient {
    pub async fn new() -> Result<Self, OperationError> {
        Ok(Self(shared().await?))
    }
    pub fn with_operations(operations: Arc<StorageOperations>) -> Self {
        Self(operations)
    }
    pub async fn get_filesystem_tools(&self) -> Result<Vec<FilesystemToolInfo>, OperationError> {
        Ok(self.0.filesystem_tools.clone())
    }

    pub async fn format(
        &self,
        device: &str,
        fs_type: &str,
        label: &str,
        options: FormatOptions,
    ) -> Result<(), OperationError> {
        if !self
            .0
            .filesystem_tools
            .iter()
            .any(|tool| tool.fs_type == fs_type && tool.available)
        {
            return Err(OperationError::Unsupported(format!(
                "Filesystem type '{fs_type}' is unavailable or its tools are not installed"
            )));
        }
        self.0
            .registry
            .block
            .format_filesystem(device, fs_type, label, options)
            .await
            .map_err(Into::into)
    }

    pub async fn mount(
        &self,
        device: &str,
        mount_point: &str,
        options: MountOptions,
    ) -> Result<String, OperationError> {
        self.0
            .registry
            .block
            .mount_filesystem(device, mount_point, options)
            .await
            .map_err(Into::into)
    }

    pub async fn unmount(
        &self,
        device_or_mount: &str,
        force: bool,
        kill_processes: bool,
    ) -> Result<UnmountResult, OperationError> {
        match self
            .0
            .registry
            .block
            .unmount_filesystem(device_or_mount, force)
            .await
        {
            Ok(()) => Ok(UnmountResult {
                success: true,
                error: None,
                blocking_processes: Vec::new(),
            }),
            Err(error) => {
                let message = error.message;
                let mount_point = if device_or_mount.starts_with("/dev/") {
                    self.0
                        .registry
                        .block
                        .get_mount_point(device_or_mount)
                        .await
                        .unwrap_or_else(|_| device_or_mount.into())
                } else {
                    device_or_mount.into()
                };
                if !(message.to_ascii_lowercase().contains("busy")
                    || message.to_ascii_lowercase().contains("in use"))
                {
                    return Ok(UnmountResult {
                        success: false,
                        error: Some(message),
                        blocking_processes: Vec::new(),
                    });
                }
                let processes = self
                    .0
                    .registry
                    .block
                    .blocking_processes(&mount_point)
                    .await
                    .unwrap_or_default();
                if kill_processes && !processes.is_empty() {
                    if protected_paths::is_protected_path(Path::new(&mount_point)) {
                        return Ok(UnmountResult {
                            success: false,
                            error: Some(format!(
                                "Cannot kill processes on protected system path: {mount_point}"
                            )),
                            blocking_processes: processes,
                        });
                    }
                    let pids = processes
                        .iter()
                        .map(|process| process.pid)
                        .collect::<Vec<_>>();
                    self.0.registry.block.kill_processes(&pids).await?;
                    return match self
                        .0
                        .registry
                        .block
                        .unmount_filesystem(device_or_mount, force)
                        .await
                    {
                        Ok(()) => Ok(UnmountResult {
                            success: true,
                            error: None,
                            blocking_processes: Vec::new(),
                        }),
                        Err(error) => Ok(UnmountResult {
                            success: false,
                            error: Some(error.message),
                            blocking_processes: Vec::new(),
                        }),
                    };
                }
                Ok(UnmountResult {
                    success: false,
                    error: Some(message),
                    blocking_processes: processes,
                })
            }
        }
    }

    pub async fn check(&self, device: &str, repair: bool) -> Result<String, OperationError> {
        let clean = self
            .0
            .registry
            .block
            .check_filesystem(device, repair)
            .await?;
        Ok(if clean {
            "Filesystem check completed successfully".into()
        } else {
            "Filesystem check found errors".into()
        })
    }
    pub async fn set_label(&self, device: &str, label: &str) -> Result<(), OperationError> {
        self.0
            .registry
            .block
            .set_filesystem_label(device, label)
            .await
            .map_err(Into::into)
    }

    pub async fn get_usage_scan(
        &self,
        _scan_id: &str,
        top_files: u32,
        mounts: &[String],
        show_all_files: bool,
        preset: UsageScanParallelismPreset,
    ) -> Result<UsageScanResult, OperationError> {
        let mounts = validate_selected_mounts(mounts)?;
        let estimate = storage_sys::usage::estimate_used_bytes_for_mounts(&mounts);
        let config = storage_sys::usage::ScanConfig {
            threads: Some(scan_threads(preset)),
            top_files_per_category: top_files as usize,
            show_all_files,
            caller_uid: Some(unsafe { libc::geteuid() }),
            caller_gids: None,
        };
        let mut result =
            tokio::task::spawn_blocking(move || storage_sys::usage::scan_paths(&mounts, &config))
                .await
                .map_err(|error| OperationError::Failed(error.to_string()))?
                .map_err(|error| OperationError::Failed(error.to_string()))?;
        if !show_all_files {
            filter_hidden_categories(&mut result);
        }
        result.total_free_bytes = estimate.free_bytes;
        Ok(result)
    }

    pub async fn delete_usage_files(
        &self,
        paths: &[String],
    ) -> Result<UsageDeleteResult, OperationError> {
        let mut result = UsageDeleteResult {
            deleted: Vec::new(),
            failed: Vec::new(),
        };
        for path_string in paths {
            let path = Path::new(path_string);
            let reason = if !path.is_absolute() {
                Some("Path must be absolute".into())
            } else if path == Path::new("/") {
                Some("Refusing to delete root path".into())
            } else {
                match std::fs::symlink_metadata(path) {
                    Ok(metadata) if !metadata.is_file() => {
                        Some("Only regular files can be deleted".into())
                    }
                    Ok(_) => match std::fs::remove_file(path) {
                        Ok(()) => {
                            result.deleted.push(path_string.clone());
                            None
                        }
                        Err(error) => Some(error.to_string()),
                    },
                    Err(error) => Some(error.to_string()),
                }
            };
            if let Some(reason) = reason {
                result.failed.push(UsageDeleteFailure {
                    path: path_string.clone(),
                    reason,
                });
            }
        }
        Ok(result)
    }

    pub async fn list_usage_mount_points(&self) -> Result<Vec<String>, OperationError> {
        storage_sys::usage::discover_local_mounts_under(Path::new("/"))
            .map(|mounts| {
                mounts
                    .into_iter()
                    .map(|mount| mount.to_string_lossy().to_string())
                    .collect()
            })
            .map_err(|error| OperationError::Failed(error.to_string()))
    }
    pub async fn authorize_usage_show_all_files(&self) -> Result<bool, OperationError> {
        Ok(true)
    }
    pub async fn get_mount_options(
        &self,
        device: &str,
    ) -> Result<Option<MountOptionsSettings>, OperationError> {
        self.0
            .registry
            .block
            .mount_options(device)
            .await
            .map_err(Into::into)
    }
    pub async fn default_mount_options(&self, device: &str) -> Result<(), OperationError> {
        self.0
            .registry
            .block
            .reset_mount_options(device)
            .await
            .map_err(Into::into)
    }
    pub async fn edit_mount_options(
        &self,
        device: &str,
        mount_at_startup: bool,
        show_in_ui: bool,
        require_auth: bool,
        display_name: Option<&str>,
        icon_name: Option<&str>,
        symbolic_icon_name: Option<&str>,
        other_options: &str,
        mount_point: &str,
        identify_as: &str,
        filesystem_type: &str,
    ) -> Result<(), OperationError> {
        self.0
            .registry
            .block
            .set_mount_options(
                device,
                mount_at_startup,
                show_in_ui,
                require_auth,
                display_name.map(str::to_owned),
                icon_name.map(str::to_owned),
                symbolic_icon_name.map(str::to_owned),
                other_options.into(),
                mount_point.into(),
                identify_as.into(),
                filesystem_type.into(),
            )
            .await
            .map_err(Into::into)
    }
    pub async fn take_ownership(
        &self,
        device: &str,
        recursive: bool,
    ) -> Result<(), OperationError> {
        self.0
            .registry
            .block
            .take_filesystem_ownership(device, recursive)
            .await
            .map_err(Into::into)
    }
}

// SPDX-License-Identifier: GPL-3.0-only

//! UDisks2 implementation of the application-facing backend contracts.

use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use storage_contracts::{
    BackendMetadata, DeviceEventSource, DiskDiscovery, DriveOperations, EncryptionOperations,
    FilesystemOperations, ImageDeviceOperations, PartitionOperations, StorageError,
    StorageErrorKind,
};
use storage_types::{
    BackendId, CreatePartitionInfo, DeviceEvent, FilesystemInfo, FormatOptions, MountOptions,
    MountOptionsSettings, ProcessInfo, SmartInfo, SmartSelfTestKind, StorageBackendCapabilities,
    VolumeInfo,
};

use crate::DiskManager;
use zbus::Connection;

/// The shipped UDisks2 adapter.  It owns one `DiskManager`, and therefore one
/// system-bus connection, for discovery and device-event subscription.
#[derive(Clone)]
pub struct UdisksBackend {
    manager: DiskManager,
    object_manager_epoch: Arc<AtomicU64>,
}

impl UdisksBackend {
    pub async fn new() -> Result<Self, StorageError> {
        DiskManager::new()
            .await
            .map(|manager| Self {
                manager,
                object_manager_epoch: Arc::new(AtomicU64::new(0)),
            })
            .map_err(unavailable)
    }

    pub fn from_manager(manager: DiskManager) -> Self {
        Self {
            manager,
            object_manager_epoch: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Construct the production adapter over an explicitly supplied D-Bus
    /// transport.  The caller owns choosing that transport; [`Self::new`]
    /// retains the normal system-bus production behaviour.
    pub fn from_connection(connection: Arc<Connection>) -> Self {
        Self::from_manager(DiskManager::from_connection(connection))
    }

    pub fn manager(&self) -> &DiskManager {
        &self.manager
    }

    /// Ask UDisks to load optional modules (notably the Btrfs module) through
    /// the system daemon. This is deliberately separate from any direct
    /// `btrfs` command so native operations retain UDisks/Polkit handling.
    pub async fn enable_optional_modules(&self) -> Result<(), StorageError> {
        self.manager.enable_modules().await.map_err(unavailable)
    }

    pub(crate) fn logical_epoch(&self) -> u64 {
        self.object_manager_epoch.load(Ordering::Acquire)
    }

    fn advance_logical_epoch(&self) {
        self.object_manager_epoch.fetch_add(1, Ordering::AcqRel);
    }
}

fn error(error: impl std::fmt::Display) -> StorageError {
    crate::logical::error::native_error(error)
}

fn unavailable(error: impl std::fmt::Display) -> StorageError {
    StorageError::new(StorageErrorKind::Unavailable, error.to_string())
}

fn flatten_volumes(volumes: &[VolumeInfo], parent: Option<String>, output: &mut Vec<VolumeInfo>) {
    for volume in volumes {
        let mut flattened = volume.clone();
        flattened.parent_path = parent.clone();
        flattened.children.clear();
        let current = volume.device_path.clone();
        flatten_volumes(&volume.children, current, output);
        output.push(flattened);
    }
}

fn collect_filesystems(volumes: &[VolumeInfo], output: &mut Vec<FilesystemInfo>) {
    for volume in volumes {
        if volume.has_filesystem
            && let Some(device) = &volume.device_path
        {
            output.push(FilesystemInfo {
                device: device.clone(),
                fs_type: volume.id_type.clone(),
                label: volume.label.clone(),
                uuid: String::new(),
                mount_points: volume.mount_points.clone(),
                size: volume.size,
                available: volume.usage.as_ref().map_or(0, |usage| usage.available),
            });
        }
        collect_filesystems(&volume.children, output);
    }
}

#[async_trait]
impl DiskDiscovery for UdisksBackend {
    async fn list_disks(&self) -> Result<Vec<storage_types::DiskInfo>, StorageError> {
        crate::get_disks(&self.manager).await.map_err(error)
    }

    async fn list_volumes(&self) -> Result<Vec<VolumeInfo>, StorageError> {
        let drives = crate::get_disks_with_volumes(&self.manager)
            .await
            .map_err(error)?;
        let mut volumes = Vec::new();
        for (disk, roots) in drives {
            flatten_volumes(&roots, Some(disk.device), &mut volumes);
        }
        Ok(volumes)
    }
}

#[async_trait]
impl DeviceEventSource for UdisksBackend {
    async fn device_events(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<DeviceEvent, StorageError>> + Send>>, StorageError>
    {
        let stream = self
            .manager
            .device_event_stream_signals()
            .await
            .map_err(error)?;
        let backend = self.clone();
        Ok(Box::pin(stream.map(move |event| {
            backend.advance_logical_epoch();
            Ok(match event {
                crate::DeviceEvent::Added(path) => DeviceEvent::Added(path),
                crate::DeviceEvent::Removed(path) => DeviceEvent::Removed(path),
            })
        })))
    }
}

#[async_trait]
impl DriveOperations for UdisksBackend {
    async fn smart_info(&self, device: &str) -> Result<SmartInfo, StorageError> {
        crate::smart::info::get_smart_info_by_device_with_connection(
            self.manager.connection().as_ref(),
            device,
        )
        .await
        .map_err(error)
    }

    async fn start_smart_selftest(
        &self,
        device: &str,
        kind: SmartSelfTestKind,
    ) -> Result<(), StorageError> {
        crate::smart::test::start_drive_smart_selftest_by_device_with_connection(
            self.manager.connection().as_ref(),
            device,
            kind,
        )
        .await
        .map_err(error)
    }

    async fn eject(&self, device: &str, ejectable: bool) -> Result<(), StorageError> {
        crate::disk::power::eject_drive_by_device_with_connection(
            self.manager.connection().as_ref(),
            device,
            ejectable,
        )
        .await
        .map_err(error)
    }

    async fn power_off(&self, device: &str, can_power_off: bool) -> Result<(), StorageError> {
        crate::disk::power::power_off_drive_by_device_with_connection(
            self.manager.connection().as_ref(),
            device,
            can_power_off,
        )
        .await
        .map_err(error)
    }

    async fn standby(&self, device: &str) -> Result<(), StorageError> {
        crate::disk::power::standby_drive_by_device_with_connection(
            self.manager.connection().as_ref(),
            device,
        )
        .await
        .map_err(error)
    }

    async fn wakeup(&self, device: &str) -> Result<(), StorageError> {
        crate::disk::power::wakeup_drive_by_device_with_connection(
            self.manager.connection().as_ref(),
            device,
        )
        .await
        .map_err(error)
    }

    async fn safe_remove(
        &self,
        device: &str,
        is_loop: bool,
        removable: bool,
        can_power_off: bool,
    ) -> Result<(), StorageError> {
        crate::disk::power::remove_drive_by_device_with_connection(
            self.manager.connection().as_ref(),
            device,
            is_loop,
            removable,
            can_power_off,
        )
        .await
        .map_err(error)
    }
}

#[async_trait]
impl PartitionOperations for UdisksBackend {
    async fn list_partitions(
        &self,
        disk: &str,
    ) -> Result<Vec<storage_types::PartitionInfo>, StorageError> {
        crate::get_disks_with_partitions(&self.manager)
            .await
            .map_err(error)?
            .into_iter()
            .find(|(info, _)| info.device == disk || info.id == disk)
            .map(|(_, partitions)| partitions)
            .ok_or_else(|| {
                StorageError::new(
                    StorageErrorKind::NotFound,
                    format!("Disk not found: {disk}"),
                )
            })
    }

    async fn create_partition_table(
        &self,
        disk: &str,
        table_type: &str,
    ) -> Result<(), StorageError> {
        let path = crate::disk::discovery::block_object_path_for_device_with_connection(
            self.manager.connection().as_ref(),
            disk,
        )
        .await
        .map_err(error)?;
        crate::partition::create_partition_table_with_connection(
            self.manager.connection().as_ref(),
            &path,
            table_type,
        )
        .await
        .map_err(error)
    }

    async fn create_partition(
        &self,
        disk: &str,
        offset: u64,
        size: u64,
        type_id: &str,
    ) -> Result<String, StorageError> {
        let path = crate::disk::discovery::block_object_path_for_device_with_connection(
            self.manager.connection().as_ref(),
            disk,
        )
        .await
        .map_err(error)?;
        crate::partition::create::create_partition_with_connection(
            self.manager.connection().as_ref(),
            &path,
            offset,
            size,
            type_id,
        )
        .await
        .map_err(error)
    }

    async fn create_partition_with_filesystem(
        &self,
        disk: &str,
        info: &CreatePartitionInfo,
    ) -> Result<String, StorageError> {
        let path = crate::disk::discovery::block_object_path_for_device_with_connection(
            self.manager.connection().as_ref(),
            disk,
        )
        .await
        .map_err(error)?;
        crate::partition::create::create_partition_with_filesystem_with_connection(
            self.manager.connection().as_ref(),
            &path,
            info,
        )
        .await
        .map_err(error)
    }

    async fn delete_partition(&self, partition: &str) -> Result<(), StorageError> {
        let path = crate::disk::discovery::block_object_path_for_device_with_connection(
            self.manager.connection().as_ref(),
            partition,
        )
        .await
        .map_err(error)?;
        crate::partition::delete::delete_partition_with_connection(
            self.manager.connection().as_ref(),
            &path,
        )
        .await
        .map_err(error)
    }

    async fn resize_partition(&self, partition: &str, new_size: u64) -> Result<(), StorageError> {
        let path = crate::disk::discovery::block_object_path_for_device_with_connection(
            self.manager.connection().as_ref(),
            partition,
        )
        .await
        .map_err(error)?;
        crate::partition::resize::resize_partition_with_connection(
            self.manager.connection().as_ref(),
            &path,
            new_size,
        )
        .await
        .map_err(error)
    }

    async fn set_partition_type(&self, partition: &str, type_id: &str) -> Result<(), StorageError> {
        let path = crate::disk::discovery::block_object_path_for_device_with_connection(
            self.manager.connection().as_ref(),
            partition,
        )
        .await
        .map_err(error)?;
        crate::partition::edit::set_partition_type_with_connection(
            self.manager.connection().as_ref(),
            &path,
            type_id,
        )
        .await
        .map_err(error)
    }

    async fn set_partition_flags(&self, partition: &str, flags: u64) -> Result<(), StorageError> {
        let path = crate::disk::discovery::block_object_path_for_device_with_connection(
            self.manager.connection().as_ref(),
            partition,
        )
        .await
        .map_err(error)?;
        crate::partition::edit::set_partition_flags_with_connection(
            self.manager.connection().as_ref(),
            &path,
            flags,
        )
        .await
        .map_err(error)
    }

    async fn set_partition_name(&self, partition: &str, name: &str) -> Result<(), StorageError> {
        let path = crate::disk::discovery::block_object_path_for_device_with_connection(
            self.manager.connection().as_ref(),
            partition,
        )
        .await
        .map_err(error)?;
        crate::partition::edit::set_partition_name_with_connection(
            self.manager.connection().as_ref(),
            &path,
            name,
        )
        .await
        .map_err(error)
    }
}

#[async_trait]
impl FilesystemOperations for UdisksBackend {
    async fn list_filesystems(&self) -> Result<Vec<FilesystemInfo>, StorageError> {
        let drives = crate::get_disks_with_volumes(&self.manager)
            .await
            .map_err(error)?;
        let mut filesystems = Vec::new();
        for (_, volumes) in drives {
            collect_filesystems(&volumes, &mut filesystems);
        }
        Ok(filesystems)
    }

    async fn format_filesystem(
        &self,
        device: &str,
        filesystem_type: &str,
        label: &str,
        options: FormatOptions,
    ) -> Result<(), StorageError> {
        crate::filesystem::format::format_filesystem_with_connection(
            self.manager.connection().as_ref(),
            device,
            filesystem_type,
            label,
            options,
        )
        .await
        .map_err(error)
    }

    async fn mount_filesystem(
        &self,
        device: &str,
        mount_point: &str,
        options: MountOptions,
    ) -> Result<String, StorageError> {
        let uid = unsafe { libc::geteuid() };
        crate::filesystem::mount::mount_filesystem_with_connection(
            self.manager.connection().as_ref(),
            device,
            mount_point,
            options,
            Some(uid),
        )
        .await
        .map_err(error)
    }

    async fn get_mount_point(&self, device: &str) -> Result<String, StorageError> {
        crate::filesystem::mount::get_mount_point_with_connection(
            self.manager.connection().as_ref(),
            device,
        )
        .await
        .map_err(error)
    }

    async fn unmount_filesystem(
        &self,
        device_or_mount: &str,
        force: bool,
    ) -> Result<(), StorageError> {
        crate::filesystem::mount::unmount_filesystem_with_connection(
            self.manager.connection().as_ref(),
            device_or_mount,
            force,
        )
        .await
        .map_err(error)
    }

    async fn blocking_processes(
        &self,
        mount_point: &str,
    ) -> Result<Vec<ProcessInfo>, StorageError> {
        crate::find_processes_using_mount(mount_point)
            .await
            .map_err(error)
    }

    async fn kill_processes(&self, pids: &[i32]) -> Result<(), StorageError> {
        let results = crate::kill_processes(pids);
        if let Some(failure) = results.into_iter().find(|result| !result.success) {
            return Err(StorageError::new(
                StorageErrorKind::PermissionDenied,
                failure
                    .error
                    .unwrap_or_else(|| "Could not terminate process".into()),
            ));
        }
        Ok(())
    }

    async fn check_filesystem(&self, device: &str, repair: bool) -> Result<bool, StorageError> {
        crate::filesystem::check::check_filesystem_with_connection(
            self.manager.connection().as_ref(),
            device,
            repair,
        )
        .await
        .map_err(error)
    }

    async fn filesystem_label(&self, device: &str) -> Result<String, StorageError> {
        crate::filesystem::label::get_filesystem_label_with_connection(
            self.manager.connection().as_ref(),
            device,
        )
        .await
        .map_err(error)
    }

    async fn set_filesystem_label(&self, device: &str, label: &str) -> Result<(), StorageError> {
        crate::filesystem::label::set_filesystem_label_with_connection(
            self.manager.connection().as_ref(),
            device,
            label,
        )
        .await
        .map_err(error)
    }

    async fn mount_options(
        &self,
        device: &str,
    ) -> Result<Option<MountOptionsSettings>, StorageError> {
        crate::filesystem::config::get_mount_options_with_connection(
            self.manager.connection().as_ref(),
            device,
        )
        .await
        .map_err(error)
    }

    async fn reset_mount_options(&self, device: &str) -> Result<(), StorageError> {
        crate::filesystem::config::reset_mount_options_with_connection(
            self.manager.connection().as_ref(),
            device,
        )
        .await
        .map_err(error)
    }

    async fn set_mount_options(
        &self,
        device: &str,
        mount_at_startup: bool,
        show_in_ui: bool,
        require_auth: bool,
        display_name: Option<String>,
        icon_name: Option<String>,
        symbolic_icon_name: Option<String>,
        options: String,
        mount_point: String,
        identify_as: String,
        filesystem_type: String,
    ) -> Result<(), StorageError> {
        crate::filesystem::config::set_mount_options_with_connection(
            self.manager.connection().as_ref(),
            device,
            mount_at_startup,
            show_in_ui,
            require_auth,
            display_name,
            icon_name,
            symbolic_icon_name,
            options,
            mount_point,
            identify_as,
            filesystem_type,
        )
        .await
        .map_err(error)
    }

    async fn take_filesystem_ownership(
        &self,
        device: &str,
        recursive: bool,
    ) -> Result<(), StorageError> {
        crate::filesystem::ownership::take_filesystem_ownership_with_connection(
            self.manager.connection().as_ref(),
            device,
            recursive,
        )
        .await
        .map_err(error)
    }
}

#[async_trait]
impl EncryptionOperations for UdisksBackend {
    async fn list_luks_devices(&self) -> Result<Vec<storage_types::LuksInfo>, StorageError> {
        crate::encryption::list::list_luks_devices_with_connection(
            self.manager.connection().as_ref(),
        )
        .await
        .map_err(error)
    }

    async fn format_luks(
        &self,
        device: &str,
        passphrase: &str,
        version: &str,
    ) -> Result<(), StorageError> {
        crate::encryption::format::format_luks_with_connection(
            self.manager.connection().as_ref(),
            device,
            passphrase,
            version,
        )
        .await
        .map_err(error)
    }

    async fn unlock_luks(&self, device: &str, passphrase: &str) -> Result<String, StorageError> {
        crate::encryption::unlock::unlock_luks_with_connection(
            self.manager.connection().as_ref(),
            device,
            passphrase,
        )
        .await
        .map_err(error)
    }

    async fn lock_luks(&self, device: &str) -> Result<(), StorageError> {
        crate::encryption::lock::lock_luks_with_connection(
            self.manager.connection().as_ref(),
            device,
        )
        .await
        .map_err(error)
    }

    async fn change_luks_passphrase(
        &self,
        device: &str,
        current: &str,
        next: &str,
    ) -> Result<(), StorageError> {
        crate::encryption::passphrase::change_luks_passphrase_with_connection(
            self.manager.connection().as_ref(),
            device,
            current,
            next,
        )
        .await
        .map_err(error)
    }

    async fn encryption_options(
        &self,
        device: &str,
    ) -> Result<Option<storage_types::EncryptionOptionsSettings>, StorageError> {
        crate::encryption::config::get_encryption_options_with_connection(
            self.manager.connection().as_ref(),
            device,
        )
        .await
        .map_err(error)
    }

    async fn set_encryption_options(
        &self,
        device: &str,
        settings: &storage_types::EncryptionOptionsSettings,
    ) -> Result<(), StorageError> {
        crate::encryption::config::set_encryption_options_with_connection(
            self.manager.connection().as_ref(),
            device,
            settings,
        )
        .await
        .map_err(error)
    }

    async fn clear_encryption_options(&self, device: &str) -> Result<(), StorageError> {
        crate::encryption::config::clear_encryption_options_with_connection(
            self.manager.connection().as_ref(),
            device,
        )
        .await
        .map_err(error)
    }
}

#[async_trait]
impl ImageDeviceOperations for UdisksBackend {
    async fn open_for_backup(&self, device: &str) -> Result<std::os::fd::OwnedFd, StorageError> {
        crate::disk::device_apis::open_for_backup_by_device_with_connection(
            self.manager.connection().as_ref(),
            device,
        )
        .await
        .map_err(error)
    }

    async fn open_for_restore(&self, device: &str) -> Result<std::os::fd::OwnedFd, StorageError> {
        crate::disk::device_apis::open_for_restore_by_device_with_connection(
            self.manager.connection().as_ref(),
            device,
        )
        .await
        .map_err(error)
    }

    async fn loop_setup(&self, image_path: &str) -> Result<String, StorageError> {
        crate::disk::device_apis::loop_setup_device_path_with_connection(
            self.manager.connection().as_ref(),
            image_path,
        )
        .await
        .map_err(error)
    }
}

impl BackendMetadata for UdisksBackend {
    fn id(&self) -> BackendId {
        BackendId::new("udisks2")
    }

    fn capabilities(&self) -> StorageBackendCapabilities {
        StorageBackendCapabilities {
            drive_power_management: true,
            partitioning: true,
            filesystem_operations: true,
            encryption_operations: true,
            image_operations: true,
            logical_storage: true,
        }
    }
}

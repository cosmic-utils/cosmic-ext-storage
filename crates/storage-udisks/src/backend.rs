// SPDX-License-Identifier: GPL-3.0-only

//! UDisks2 implementation of the application-facing backend contracts.

use std::pin::Pin;

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

/// The shipped UDisks2 adapter.  It owns one `DiskManager`, and therefore one
/// system-bus connection, for discovery and device-event subscription.
#[derive(Clone)]
pub struct UdisksBackend {
    manager: DiskManager,
}

impl UdisksBackend {
    pub async fn new() -> Result<Self, StorageError> {
        DiskManager::new()
            .await
            .map(|manager| Self { manager })
            .map_err(unavailable)
    }

    pub fn from_manager(manager: DiskManager) -> Self {
        Self { manager }
    }

    pub fn manager(&self) -> &DiskManager {
        &self.manager
    }
}

fn error(error: impl std::fmt::Display) -> StorageError {
    StorageError::new(StorageErrorKind::Internal, error.to_string())
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
        Ok(Box::pin(stream.map(|event| {
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
        crate::get_smart_info_by_device(device).await.map_err(error)
    }

    async fn start_smart_selftest(
        &self,
        device: &str,
        kind: SmartSelfTestKind,
    ) -> Result<(), StorageError> {
        crate::start_drive_smart_selftest_by_device(device, kind)
            .await
            .map_err(error)
    }

    async fn eject(&self, device: &str, ejectable: bool) -> Result<(), StorageError> {
        crate::eject_drive_by_device(device, ejectable)
            .await
            .map_err(error)
    }

    async fn power_off(&self, device: &str, can_power_off: bool) -> Result<(), StorageError> {
        crate::power_off_drive_by_device(device, can_power_off)
            .await
            .map_err(error)
    }

    async fn standby(&self, device: &str) -> Result<(), StorageError> {
        crate::standby_drive_by_device(device).await.map_err(error)
    }

    async fn wakeup(&self, device: &str) -> Result<(), StorageError> {
        crate::wakeup_drive_by_device(device).await.map_err(error)
    }

    async fn safe_remove(
        &self,
        device: &str,
        is_loop: bool,
        removable: bool,
        can_power_off: bool,
    ) -> Result<(), StorageError> {
        crate::remove_drive_by_device(device, is_loop, removable, can_power_off)
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
        let path = crate::block_object_path_for_device(disk)
            .await
            .map_err(error)?;
        crate::create_partition_table(&path, table_type)
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
        let path = crate::block_object_path_for_device(disk)
            .await
            .map_err(error)?;
        crate::create_partition(&path, offset, size, type_id)
            .await
            .map_err(error)
    }

    async fn create_partition_with_filesystem(
        &self,
        disk: &str,
        info: &CreatePartitionInfo,
    ) -> Result<String, StorageError> {
        let path = crate::block_object_path_for_device(disk)
            .await
            .map_err(error)?;
        crate::create_partition_with_filesystem(&path, info)
            .await
            .map_err(error)
    }

    async fn delete_partition(&self, partition: &str) -> Result<(), StorageError> {
        crate::delete_partition(partition).await.map_err(error)
    }

    async fn resize_partition(&self, partition: &str, new_size: u64) -> Result<(), StorageError> {
        crate::resize_partition(partition, new_size)
            .await
            .map_err(error)
    }

    async fn set_partition_type(&self, partition: &str, type_id: &str) -> Result<(), StorageError> {
        crate::set_partition_type(partition, type_id)
            .await
            .map_err(error)
    }

    async fn set_partition_flags(&self, partition: &str, flags: u64) -> Result<(), StorageError> {
        crate::set_partition_flags(partition, flags)
            .await
            .map_err(error)
    }

    async fn set_partition_name(&self, partition: &str, name: &str) -> Result<(), StorageError> {
        crate::set_partition_name(partition, name)
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
        crate::format_filesystem(device, filesystem_type, label, options)
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
        crate::mount_filesystem(device, mount_point, options, Some(uid))
            .await
            .map_err(error)
    }

    async fn get_mount_point(&self, device: &str) -> Result<String, StorageError> {
        crate::get_mount_point(device).await.map_err(error)
    }

    async fn unmount_filesystem(
        &self,
        device_or_mount: &str,
        force: bool,
    ) -> Result<(), StorageError> {
        crate::unmount_filesystem(device_or_mount, force)
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
        crate::check_filesystem(device, repair).await.map_err(error)
    }

    async fn filesystem_label(&self, device: &str) -> Result<String, StorageError> {
        crate::get_filesystem_label(device).await.map_err(error)
    }

    async fn set_filesystem_label(&self, device: &str, label: &str) -> Result<(), StorageError> {
        crate::set_filesystem_label(device, label)
            .await
            .map_err(error)
    }

    async fn mount_options(
        &self,
        device: &str,
    ) -> Result<Option<MountOptionsSettings>, StorageError> {
        crate::get_mount_options(device).await.map_err(error)
    }

    async fn reset_mount_options(&self, device: &str) -> Result<(), StorageError> {
        crate::reset_mount_options(device).await.map_err(error)
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
        crate::set_mount_options(
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
        crate::take_filesystem_ownership(device, recursive)
            .await
            .map_err(error)
    }
}

#[async_trait]
impl EncryptionOperations for UdisksBackend {
    async fn list_luks_devices(&self) -> Result<Vec<storage_types::LuksInfo>, StorageError> {
        crate::list_luks_devices().await.map_err(error)
    }

    async fn format_luks(
        &self,
        device: &str,
        passphrase: &str,
        version: &str,
    ) -> Result<(), StorageError> {
        crate::format_luks(device, passphrase, version)
            .await
            .map_err(error)
    }

    async fn unlock_luks(&self, device: &str, passphrase: &str) -> Result<String, StorageError> {
        crate::unlock_luks(device, passphrase).await.map_err(error)
    }

    async fn lock_luks(&self, device: &str) -> Result<(), StorageError> {
        crate::lock_luks(device).await.map_err(error)
    }

    async fn change_luks_passphrase(
        &self,
        device: &str,
        current: &str,
        next: &str,
    ) -> Result<(), StorageError> {
        crate::change_luks_passphrase(device, current, next)
            .await
            .map_err(error)
    }

    async fn encryption_options(
        &self,
        device: &str,
    ) -> Result<Option<storage_types::EncryptionOptionsSettings>, StorageError> {
        crate::get_encryption_options(device).await.map_err(error)
    }

    async fn set_encryption_options(
        &self,
        device: &str,
        settings: &storage_types::EncryptionOptionsSettings,
    ) -> Result<(), StorageError> {
        crate::set_encryption_options(device, settings)
            .await
            .map_err(error)
    }

    async fn clear_encryption_options(&self, device: &str) -> Result<(), StorageError> {
        crate::clear_encryption_options(device).await.map_err(error)
    }
}

#[async_trait]
impl ImageDeviceOperations for UdisksBackend {
    async fn open_for_backup(&self, device: &str) -> Result<std::os::fd::OwnedFd, StorageError> {
        crate::open_for_backup_by_device(device)
            .await
            .map_err(error)
    }

    async fn open_for_restore(&self, device: &str) -> Result<std::os::fd::OwnedFd, StorageError> {
        crate::open_for_restore_by_device(device)
            .await
            .map_err(error)
    }

    async fn loop_setup(&self, image_path: &str) -> Result<String, StorageError> {
        crate::loop_setup_device_path(image_path)
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
        }
    }
}

// SPDX-License-Identifier: GPL-3.0-only

//! UI model for a disk drive with shared clients and hierarchical volumes

use super::{UiVolume, build_volume_tree};
use crate::operations::{DisksClient, PartitionsClient, error::OperationError};
use std::ops::Deref;
use std::sync::Arc;
use storage_types::{DiskInfo, PartitionInfo};

/// Recursively collect all volumes from a slice of roots into a flat list (each without children).
fn collect_volumes_flat_slice(
    roots: &[UiVolume],
    fs_client: Arc<crate::operations::FilesystemsClient>,
) -> Vec<UiVolume> {
    let mut out = Vec::new();
    for root in roots {
        collect_volumes_flat_one(root, &mut out, Arc::clone(&fs_client));
    }
    out
}

fn collect_volumes_flat_one(
    volume: &UiVolume,
    out: &mut Vec<UiVolume>,
    fs_client: Arc<crate::operations::FilesystemsClient>,
) {
    if let Ok(flat_vol) =
        UiVolume::with_children(volume.volume.clone(), Vec::new(), Arc::clone(&fs_client))
    {
        out.push(flat_vol);
    }
    for child in &volume.children {
        collect_volumes_flat_one(child, out, Arc::clone(&fs_client));
    }
}

/// UI model wrapping DiskInfo with owned client and volume tree
///
/// This model:
/// - Owns a DisksClient for data refresh
/// - Builds hierarchical volume trees from flat lists using parent_path
/// - Provides helper methods for finding volumes
/// - Supports atomic updates for performance
#[derive(Debug)]
pub struct UiDrive {
    /// Disk information from in-process operations
    pub disk: DiskInfo,

    /// Hierarchical volume tree (roots only - children nested inside)
    pub volumes: Vec<UiVolume>,

    /// Flat list of partitions on this disk
    pub partitions: Vec<PartitionInfo>,

    /// Flat list of all volumes (non-hierarchical) for bulk operations
    pub volumes_flat: Vec<UiVolume>,

    /// Shared client for refreshing disk data
    client: Arc<DisksClient>,

    /// Shared client for refreshing partition data
    partitions_client: Arc<PartitionsClient>,

    /// Shared client for filesystem operations (used when building volume tree)
    filesystems_client: Arc<crate::operations::FilesystemsClient>,
}

#[allow(dead_code)]
impl UiDrive {
    /// Create a new UiDrive, loading initial data from in-process operations
    ///
    /// This performs the initial population of volumes and partitions.
    ///
    /// # Example
    /// ```no_run
    /// let disk_info = disks_client.get_disk_info("/dev/sda").await?;
    /// let ui_drive = UiDrive::new(disk_info).await?;
    /// ```
    pub async fn new(disk: DiskInfo) -> Result<Self, OperationError> {
        let client = Arc::new(DisksClient::new().await?);
        let partitions_client = Arc::new(PartitionsClient::new().await?);
        let filesystems_client = Arc::new(crate::operations::FilesystemsClient::new().await?);

        let mut drive = Self {
            disk,
            volumes: Vec::new(),
            partitions: Vec::new(),
            volumes_flat: Vec::new(),
            client,
            partitions_client,
            filesystems_client,
        };

        drive.refresh().await?;
        Ok(drive)
    }

    /// Full refresh of all data (disk info, volumes, partitions)
    ///
    /// This is the baseline operation used when atomic updates aren't applicable.
    pub async fn refresh(&mut self) -> Result<(), OperationError> {
        self.refresh_disk().await?;
        self.refresh_volumes().await?;
        self.refresh_partitions().await?;
        Ok(())
    }

    /// Refresh disk information only
    pub async fn refresh_disk(&mut self) -> Result<(), OperationError> {
        self.disk = self.client.get_disk_info(&self.disk.device).await?;
        Ok(())
    }

    /// Refresh volumes, rebuilding the entire tree
    ///
    /// Uses list_volumes() and builds tree from parent_path references.
    pub async fn refresh_volumes(&mut self) -> Result<(), OperationError> {
        let all_volumes = self.client.list_volumes().await?;
        self.volumes = build_volume_tree(
            &self.disk.device,
            all_volumes,
            Arc::clone(&self.filesystems_client),
        )?;

        // Also populate flat list by collecting all volumes recursively
        self.volumes_flat =
            collect_volumes_flat_slice(&self.volumes, Arc::clone(&self.filesystems_client));

        Ok(())
    }

    /// Refresh partitions list
    pub async fn refresh_partitions(&mut self) -> Result<(), OperationError> {
        self.partitions = self
            .partitions_client
            .list_partitions(&self.disk.device)
            .await?;
        Ok(())
    }

    /// Add a partition to the tree after creation
    ///
    /// This supports atomic updates - after creating a partition, just add it
    /// to the tree without a full refresh.
    pub fn add_partition(&mut self, partition: PartitionInfo, volume: UiVolume) {
        self.partitions.push(partition);
        self.volumes.push(volume);
    }

    /// Remove a partition from the tree after deletion
    ///
    /// Returns true if the partition was found and removed.
    pub fn remove_partition(&mut self, device: &str) -> bool {
        // Remove from partitions list
        let partition_removed =
            if let Some(idx) = self.partitions.iter().position(|p| p.device == device) {
                self.partitions.remove(idx);
                true
            } else {
                false
            };

        // Remove from volumes tree
        let old_len = self.volumes.len();
        self.volumes
            .retain(|v| v.volume.device_path.as_ref().is_none_or(|d| d != device));
        let volume_removed = self.volumes.len() < old_len;

        partition_removed || volume_removed
    }

    /// Find a volume by device path (recursive search)
    pub fn find_volume(&self, device: &str) -> Option<&UiVolume> {
        for root in &self.volumes {
            if let Some(vol) = root.find_by_device(device) {
                return Some(vol);
            }
        }
        None
    }

    /// Find a volume by device path (mutable, recursive search)
    pub fn find_volume_mut(&mut self, device: &str) -> Option<&mut UiVolume> {
        for root in &mut self.volumes {
            if let Some(vol) = root.find_by_device_mut(device) {
                return Some(vol);
            }
        }
        None
    }

    /// Get device path for this drive
    pub fn device(&self) -> &str {
        &self.disk.device
    }

    /// Get a human-readable display name for the drive
    pub fn name(&self) -> String {
        self.disk.display_name()
    }
}

/// Deref to DiskInfo to expose all disk fields directly
impl Deref for UiDrive {
    type Target = DiskInfo;

    fn deref(&self) -> &Self::Target {
        &self.disk
    }
}

/// Clone is cheap: only clones Arc pointers and data, no new D-Bus clients or runtime.
impl Clone for UiDrive {
    fn clone(&self) -> Self {
        Self {
            disk: self.disk.clone(),
            volumes: self.volumes.clone(),
            partitions: self.partitions.clone(),
            volumes_flat: self.volumes_flat.clone(),
            client: Arc::clone(&self.client),
            partitions_client: Arc::clone(&self.partitions_client),
            filesystems_client: Arc::clone(&self.filesystems_client),
        }
    }
}

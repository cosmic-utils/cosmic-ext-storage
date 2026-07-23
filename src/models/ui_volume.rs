// SPDX-License-Identifier: GPL-3.0-only

//! UI model for a volume with hierarchical children and shared client

use crate::operations::{FilesystemsClient, error::OperationError};
use std::ops::Deref;
use std::sync::Arc;
use storage_types::VolumeInfo;

/// UI model wrapping VolumeInfo with shared client and hierarchical children
///
/// This model:
/// - Shares an Arc<FilesystemsClient> for filesystem operations
/// - Maintains hierarchical children relationships
/// - Provides helper methods for recursive searches
/// - Supports atomic updates
#[derive(Debug)]
pub struct UiVolume {
    /// Volume information from in-process operations
    pub volume: VolumeInfo,

    /// Child volumes (e.g., unlocked LUKS containers, nested partitions)
    pub children: Vec<UiVolume>,

    /// Shared client for filesystem operations
    client: Arc<FilesystemsClient>,
}

#[allow(dead_code)]
impl UiVolume {
    /// Create a new UiVolume from VolumeInfo
    ///
    /// Note: children is empty initially - call build_volume_tree() to populate.
    pub async fn new(volume: VolumeInfo) -> Result<Self, OperationError> {
        let client = Arc::new(FilesystemsClient::new().await?);
        Ok(Self {
            volume,
            children: Vec::new(),
            client,
        })
    }

    /// Create a UiVolume with children (used by tree builder)
    ///
    /// Uses the shared filesystems client so Clone is cheap.
    pub fn with_children(
        volume: VolumeInfo,
        children: Vec<UiVolume>,
        client: Arc<FilesystemsClient>,
    ) -> Result<Self, OperationError> {
        Ok(Self {
            volume,
            children,
            client,
        })
    }

    /// Find a volume by device path (recursive search)
    ///
    /// # Example
    /// ```no_run
    /// if let Some(volume) = drive.volumes[0].find_by_device("/dev/sda1") {
    ///     println!("Found: {:?}", volume.volume.label);
    /// }
    /// ```
    pub fn find_by_device(&self, device: &str) -> Option<&UiVolume> {
        // Check self
        if self
            .volume
            .device_path
            .as_ref()
            .is_some_and(|d| d == device)
        {
            return Some(self);
        }

        // Search children recursively
        for child in &self.children {
            if let Some(found) = child.find_by_device(device) {
                return Some(found);
            }
        }

        None
    }

    /// Find a volume by device path (mutable, recursive search)
    pub fn find_by_device_mut(&mut self, device: &str) -> Option<&mut UiVolume> {
        // Check self
        if self
            .volume
            .device_path
            .as_ref()
            .is_some_and(|d| d == device)
        {
            return Some(self);
        }

        // Search children recursively
        for child in &mut self.children {
            if let Some(found) = child.find_by_device_mut(device) {
                return Some(found);
            }
        }

        None
    }

    /// Collect all mounted descendants (recursive)
    ///
    /// Used for operations that need to unmount a volume tree
    /// (e.g., before deleting a partition).
    ///
    /// # Example
    /// ```no_run
    /// let mounted = volume.collect_mounted_descendants();
    /// for descendant in mounted {
    ///     fs_client.unmount(&descendant, false, false).await?;
    /// }
    /// ```
    pub fn collect_mounted_descendants(&self) -> Vec<String> {
        let mut result = Vec::new();

        // Check self
        if !self.volume.mount_points.is_empty()
            && let Some(device) = &self.volume.device_path
        {
            result.push(device.clone());
        }

        // Collect from children recursively
        for child in &self.children {
            result.extend(child.collect_mounted_descendants());
        }

        result
    }

    /// Update this volume or a child with new VolumeInfo (atomic update)
    ///
    /// Returns true if the volume was found and updated.
    ///
    /// # Example
    /// ```no_run
    /// // After mounting /dev/sda1
    /// if !root_volume.update_volume("/dev/sda1", &updated_info) {
    ///     // Volume not found in this subtree
    /// }
    /// ```
    pub fn update_volume(&mut self, device: &str, updated_info: &VolumeInfo) -> bool {
        // Check if this is the target volume
        if self
            .volume
            .device_path
            .as_ref()
            .is_some_and(|d| d == device)
        {
            // Update volume info, preserving children
            let old_children = std::mem::take(&mut self.volume.children);
            self.volume = updated_info.clone();
            self.volume.children = old_children;
            return true;
        }

        // Search children
        for child in &mut self.children {
            if child.update_volume(device, updated_info) {
                return true;
            }
        }

        false
    }

    /// Add a child volume (used for atomic tree mutations)
    ///
    /// # Example
    /// ```no_run
    /// // After unlocking a LUKS container
    /// locked_partition.add_child(unlocked_volume);
    /// ```
    pub fn add_child(&mut self, child: UiVolume) {
        self.children.push(child);
    }

    /// Remove a child volume by device path
    ///
    /// Returns true if the child was found and removed.
    pub fn remove_child(&mut self, device: &str) -> bool {
        // Try to remove direct child
        if let Some(idx) = self
            .children
            .iter()
            .position(|c| c.volume.device_path.as_ref().is_some_and(|d| d == device))
        {
            self.children.remove(idx);
            return true;
        }

        // Try to remove from nested children
        for child in &mut self.children {
            if child.remove_child(device) {
                return true;
            }
        }

        false
    }

    /// Get device path if available
    pub fn device(&self) -> Option<&str> {
        self.volume.device_path.as_deref()
    }

    /// Get filesystem client for operations
    pub fn filesystem_client(&self) -> &FilesystemsClient {
        self.client.as_ref()
    }

    /// Check if this volume can be mounted
    pub fn can_mount(&self) -> bool {
        self.volume.can_mount()
    }

    /// Check if this volume is mounted
    pub fn is_mounted(&self) -> bool {
        self.volume.is_mounted()
    }

    /// Get device path for this volume (e.g. /dev/sda1, /dev/mapper/luks-xxx)
    pub fn device_path(&self) -> Option<String> {
        self.volume.device_path.clone()
    }
}

/// Deref to VolumeInfo to expose all volume fields directly
impl Deref for UiVolume {
    type Target = VolumeInfo;

    fn deref(&self) -> &Self::Target {
        &self.volume
    }
}

/// Clone is cheap: only clones Arc and data, no new D-Bus client or runtime.
impl Clone for UiVolume {
    fn clone(&self) -> Self {
        Self {
            volume: self.volume.clone(),
            children: self.children.clone(),
            client: Arc::clone(&self.client),
        }
    }
}

//! Volume information - hierarchical representation
//!
//! VolumeInfo provides a recursive tree structure that can represent:
//! - Partitions
//! - LUKS encrypted containers
//! - Filesystems
//! - LVM physical/logical volumes
//! - Nested combinations of the above

use serde::{Deserialize, Serialize};

use crate::Usage;

/// Volume classification (high-level type)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolumeType {
    /// Container type (encrypted volumes, LVM)
    Container,
    /// Partition type
    Partition,
    /// Filesystem type (mountable)
    Filesystem,
}

/// Type of volume
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolumeKind {
    /// Standard partition
    Partition,

    /// LUKS encrypted container
    CryptoContainer,

    /// Filesystem (mountable)
    Filesystem,

    /// LVM physical volume
    LvmPhysicalVolume,

    /// LVM logical volume
    LvmLogicalVolume,

    /// Generic block device
    Block,
}

/// Hierarchical volume information (recursive tree structure)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolumeInfo {
    /// Type of this volume
    pub kind: VolumeKind,

    /// Human-readable label
    pub label: String,

    /// Size in bytes
    pub size: u64,

    /// Offset from start of disk in bytes (for partitions only)
    pub offset: u64,

    /// Partition number (1-based, 0 if not a partition)
    pub partition_number: u32,

    /// Filesystem/ID type (e.g., "ext4", "crypto_LUKS", "LVM2_member")
    pub id_type: String,

    /// Device path (e.g., "/dev/sda1", "/dev/mapper/luks-xxx")
    pub device_path: Option<String>,

    /// Parent device path for building hierarchy
    /// - For partition: parent disk device (e.g., "/dev/sda")
    /// - For unlocked LUKS: parent partition (e.g., "/dev/sda1")
    /// - For LVM LV: parent VG path
    /// - None for root-level volumes
    pub parent_path: Option<String>,

    /// Whether this volume has a filesystem interface
    pub has_filesystem: bool,

    /// Current mount points (empty if not mounted)
    pub mount_points: Vec<String>,

    /// Filesystem usage statistics (if mounted)
    pub usage: Option<Usage>,

    /// Whether this volume is locked (for LUKS containers)
    pub locked: bool,

    /// Child volumes (recursive structure)
    pub children: Vec<VolumeInfo>,
}

impl VolumeInfo {
    /// Check if this volume is currently mounted
    pub fn is_mounted(&self) -> bool {
        self.has_filesystem && !self.mount_points.is_empty()
    }

    /// Check if this volume can be mounted
    pub fn can_mount(&self) -> bool {
        self.has_filesystem && !self.is_mounted()
    }

    /// Check if this volume can be unlocked (LUKS)
    pub fn can_unlock(&self) -> bool {
        self.kind == VolumeKind::CryptoContainer && self.locked
    }

    /// Check if this volume can be locked (LUKS)
    pub fn can_lock(&self) -> bool {
        self.kind == VolumeKind::CryptoContainer && !self.locked
    }

    /// Recursively count total number of volumes in tree
    pub fn volume_count(&self) -> usize {
        1 + self
            .children
            .iter()
            .map(|c| c.volume_count())
            .sum::<usize>()
    }

    /// Find a volume by device path (recursive search)
    pub fn find_by_device(&self, device: &str) -> Option<&VolumeInfo> {
        if self.device_path.as_deref() == Some(device) {
            return Some(self);
        }

        for child in &self.children {
            if let Some(found) = child.find_by_device(device) {
                return Some(found);
            }
        }

        None
    }

    /// Get display name for this volume
    pub fn name(&self) -> String {
        if !self.label.is_empty() {
            self.label.clone()
        } else if let Some(device) = &self.device_path {
            device.clone()
        } else {
            "Unknown".to_string()
        }
    }
}

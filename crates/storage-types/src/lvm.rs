//! LVM (Logical Volume Manager) types
//!
//! Types for LVM volume group, logical volume, and physical volume management.

use serde::{Deserialize, Serialize};

/// Volume group information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolumeGroupInfo {
    /// Volume group name
    pub name: String,

    /// Volume group UUID
    pub uuid: String,

    /// Total size in bytes
    pub size: u64,

    /// Free space in bytes
    pub free: u64,

    /// Number of physical volumes
    pub pv_count: u32,

    /// Number of logical volumes
    pub lv_count: u32,
}

impl VolumeGroupInfo {
    /// Get used space in bytes
    pub fn used(&self) -> u64 {
        self.size.saturating_sub(self.free)
    }

    /// Get usage percentage (0-100)
    pub fn usage_percent(&self) -> u32 {
        if self.size == 0 {
            0
        } else {
            ((self.used() as f64 / self.size as f64) * 100.0) as u32
        }
    }
}

/// Logical volume information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalVolumeInfo {
    /// Logical volume name
    pub name: String,

    /// Parent volume group name
    pub vg_name: String,

    /// Logical volume UUID
    pub uuid: String,

    /// Size in bytes
    pub size: u64,

    /// Device path (e.g., "/dev/vg0/lv0" or "/dev/mapper/vg0-lv0")
    pub device_path: String,

    /// Whether the logical volume is active
    pub active: bool,
}

impl LogicalVolumeInfo {
    /// Get a display name for this logical volume
    pub fn display_name(&self) -> String {
        // Prefer short form: vg/lv
        if !self.vg_name.is_empty() && !self.name.is_empty() {
            format!("{}/{}", self.vg_name, self.name)
        } else if let Some(stripped) = self.device_path.strip_prefix("/dev/") {
            stripped.to_string()
        } else {
            self.device_path.clone()
        }
    }
}

/// Physical volume information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalVolumeInfo {
    /// Device path (e.g., "/dev/sda1")
    pub device: String,

    /// Volume group name (None if not assigned)
    pub vg_name: Option<String>,

    /// Total size in bytes
    pub size: u64,

    /// Free space in bytes
    pub free: u64,
}

impl PhysicalVolumeInfo {
    /// Check if this PV is assigned to a VG
    pub fn is_assigned(&self) -> bool {
        self.vg_name.is_some()
    }

    /// Get used space in bytes
    pub fn used(&self) -> u64 {
        self.size.saturating_sub(self.free)
    }
}

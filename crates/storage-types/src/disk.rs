//! Disk and SMART data models
//!
//! These types represent the canonical domain model for disk information.
//! All layers (storage-udisks, in-process operations, COSMIC Storage application) use these as the single source of truth.

use serde::{Deserialize, Serialize};

use crate::ByteRange;

/// Stable identifier for an application-facing storage backend.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BackendId(pub String);

impl BackendId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// Features exposed by a block-storage backend.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageBackendCapabilities {
    pub drive_power_management: bool,
    pub partitioning: bool,
    pub filesystem_operations: bool,
    pub encryption_operations: bool,
    pub image_operations: bool,
    pub logical_storage: bool,
}

/// Typed local-device notification emitted by a backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceEvent {
    Added(String),
    Removed(String),
    /// A coherent topology replacement requires one fresh discovery pass.
    Refresh,
}

/// Complete disk information (single source of truth)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiskInfo {
    // === Identity ===
    /// Device path (e.g., "/dev/sda")
    pub device: String,

    /// UDisks2 drive identifier
    pub id: String,

    /// Disk model name
    pub model: String,

    /// Serial number
    pub serial: String,

    /// Vendor/manufacturer name
    pub vendor: String,

    /// Firmware revision
    pub revision: String,

    // === Physical Properties ===
    /// Total size in bytes
    pub size: u64,

    /// Connection bus type (e.g., "usb", "ata", "nvme", "scsi", "loop")
    pub connection_bus: String,

    /// Rotation rate in RPM (None for SSDs or unknown)
    pub rotation_rate: Option<u16>,

    // === Media Properties ===
    /// Whether the disk is removable
    pub removable: bool,

    /// Whether the disk can be ejected
    pub ejectable: bool,

    /// Whether the media is removable (vs. the entire drive)
    pub media_removable: bool,

    /// Whether media is currently present
    pub media_available: bool,

    /// Whether this is an optical drive
    pub optical: bool,

    /// Whether optical media is blank
    pub optical_blank: bool,

    /// Whether the drive can be powered off
    pub can_power_off: bool,

    // === Loop Device Specific ===
    /// Whether this is a loop device
    pub is_loop: bool,

    /// Backing file for loop device
    pub backing_file: Option<String>,

    // === Partitioning ===
    /// Partition table type ("gpt", "dos", or None)
    pub partition_table_type: Option<String>,

    /// GPT usable byte range (if GPT)
    pub gpt_usable_range: Option<ByteRange>,
}

impl DiskInfo {
    /// Check if the drive supports power management (spin down/standby).
    /// Returns true for spinning disks (rotation_rate > 0), false for SSDs and NVMe drives.
    pub fn supports_power_management(&self) -> bool {
        // Loop devices don't support power management
        if self.is_loop {
            return false;
        }

        // Only rotating media (HDDs) support power management
        // None = unknown or SSD, Some(0) = explicitly SSD, Some(>0) = HDD with known RPM
        matches!(self.rotation_rate, Some(rpm) if rpm > 0)
    }

    /// Get a human-readable display name for the disk
    pub fn display_name(&self) -> String {
        if !self.model.is_empty() {
            self.model.clone()
        } else if !self.vendor.is_empty() {
            format!("{} Disk", self.vendor)
        } else {
            self.device
                .split('/')
                .next_back()
                .unwrap_or(&self.device)
                .to_string()
        }
    }
}

/// SMART health status for a disk
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SmartStatus {
    /// Device path
    pub device: String,

    /// Overall health status (true = healthy)
    pub healthy: bool,

    /// Current temperature in Celsius
    pub temperature_celsius: Option<i16>,

    /// Total power-on hours
    pub power_on_hours: Option<u64>,

    /// Number of power cycles
    pub power_cycle_count: Option<u64>,

    /// Whether a self-test is currently running
    pub test_running: bool,

    /// Self-test completion percentage (0-100)
    pub test_percent_remaining: Option<u8>,
}

/// Individual SMART attribute
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SmartAttribute {
    /// Attribute ID (1-255)
    pub id: u8,

    /// Attribute name (e.g., "Reallocated_Sector_Ct")
    pub name: String,

    /// Current normalized value (1-255, 100 is ideal)
    pub current: u8,

    /// Worst value seen (1-255)
    pub worst: u8,

    /// Failure threshold (when current <= threshold, attribute is failing)
    pub threshold: u8,

    /// Raw value (interpretation depends on attribute)
    pub raw_value: u64,

    /// Whether this attribute is currently failing
    pub failing: bool,
}

/// Disk hotplug event type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiskEvent {
    /// A disk was added to the system
    Added,

    /// A disk was removed from the system
    Removed,

    /// Disk properties changed
    Changed,
}

#[path = "../tests/unit/disk_tests.rs"]
#[cfg(test)]
mod tests;

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
}

/// Typed local-device notification emitted by a backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceEvent {
    Added(String),
    Removed(String),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_info_serialization() {
        let disk = DiskInfo {
            device: "/dev/sda".to_string(),
            id: "ata-Samsung_SSD_970_EVO_S1234567890".to_string(),
            model: "Samsung SSD 970 EVO".to_string(),
            serial: "S1234567890".to_string(),
            vendor: "Samsung".to_string(),
            revision: "1B2Q".to_string(),
            size: 1000000000000,
            connection_bus: "nvme".to_string(),
            rotation_rate: None,
            removable: false,
            ejectable: false,
            media_removable: false,
            media_available: true,
            optical: false,
            optical_blank: false,
            can_power_off: false,
            is_loop: false,
            backing_file: None,
            partition_table_type: Some("gpt".to_string()),
            gpt_usable_range: None,
        };

        let json = serde_json::to_string(&disk).unwrap();
        let deserialized: DiskInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(disk, deserialized);
    }

    #[test]
    fn test_smart_status_serialization() {
        let status = SmartStatus {
            device: "/dev/sda".to_string(),
            healthy: true,
            temperature_celsius: Some(35),
            power_on_hours: Some(1234),
            power_cycle_count: Some(567),
            test_running: false,
            test_percent_remaining: None,
        };

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: SmartStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(status, deserialized);
    }

    #[test]
    fn test_smart_attribute_serialization() {
        let attr = SmartAttribute {
            id: 5,
            name: "Reallocated_Sector_Ct".to_string(),
            current: 100,
            worst: 100,
            threshold: 10,
            raw_value: 0,
            failing: false,
        };

        let json = serde_json::to_string(&attr).unwrap();
        let deserialized: SmartAttribute = serde_json::from_str(&json).unwrap();

        assert_eq!(attr, deserialized);
    }
}

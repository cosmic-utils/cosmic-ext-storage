//! Disk-level operations
//!
//! This module provides operations that work at the disk/drive level:
//! - Discovery and enumeration
//! - Formatting (creating partition tables)
//! - Power management (eject, standby, etc.)
//! - Device-path convenience APIs for in-process operations

pub(crate) mod block_index;
pub mod device_apis;
pub mod discovery;
pub mod format;
pub mod image;
pub mod power;
pub(crate) mod resolve;
pub(crate) mod volume_tree;

// Re-export key functions and types
pub use device_apis::{
    loop_setup_device_path, open_for_backup_by_device, open_for_restore_by_device,
};
pub use discovery::{
    block_object_path_for_device, get_disk_info_for_drive_path, get_disks,
    get_disks_with_partitions, get_disks_with_volumes,
};
pub use format::format_disk;
pub use image::{open_for_backup, open_for_restore};
pub use power::{
    eject_drive, eject_drive_by_device, power_off_drive, power_off_drive_by_device, remove_drive,
    remove_drive_by_device, standby_drive, standby_drive_by_device, wakeup_drive,
    wakeup_drive_by_device,
};

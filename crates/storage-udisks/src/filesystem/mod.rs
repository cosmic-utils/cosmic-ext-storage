//! Filesystem operations
//!
//! This module provides operations for managing filesystems:
//! - Formatting filesystems
//! - Mounting and unmounting
//! - Checking and repairing
//! - Label management
//! - Ownership management

pub(crate) mod check;
pub mod config;
pub(crate) mod format;
pub(crate) mod label;
pub(crate) mod mount;
pub(crate) mod ownership;

// Re-export settings type and mount options config
pub use config::{MountOptionsSettings, get_mount_options, reset_mount_options, set_mount_options};

pub use check::{check_filesystem, repair_filesystem};
pub use format::format_filesystem;
pub use label::{get_filesystem_label, set_filesystem_label};
pub use mount::{get_mount_point, mount_filesystem, unmount_filesystem};
pub use ownership::take_filesystem_ownership;

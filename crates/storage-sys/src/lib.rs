// SPDX-License-Identifier: GPL-3.0-only

//! Low-level system operations for storage management
//!
//! This crate provides direct system call interfaces for operations that
//! don't go through D-Bus, such as:
//! - File descriptor management
//! - Direct file I/O for disk imaging
//! - Process management utilities
//! - RClone CLI operations
//!
//! These operations require elevated privileges and should only be called
//! from privileged services (like in-process operations).

pub mod error;
pub mod image;
pub mod rclone;
pub mod usage;

pub use error::{Result, SysError};
pub use image::{copy_file_to_image, copy_image_to_file, open_for_backup, open_for_restore};
pub use rclone::{RCloneCli, RcloneNetworkBackend, is_mount_on_login_enabled, set_mount_on_login};

//! Filesystem operation types
//!
//! Types for filesystem management: formatting, mounting, checking, etc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Filesystem information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemInfo {
    /// Device path
    pub device: String,

    /// Filesystem type (e.g., "ext4", "xfs", "btrfs", "vfat")
    pub fs_type: String,

    /// Filesystem label
    pub label: String,

    /// Filesystem UUID
    pub uuid: String,

    /// Current mount points (empty if not mounted)
    pub mount_points: Vec<String>,

    /// Total size in bytes
    pub size: u64,

    /// Available space in bytes (if mounted)
    pub available: u64,
}

impl FilesystemInfo {
    /// Check if this filesystem is currently mounted
    pub fn is_mounted(&self) -> bool {
        !self.mount_points.is_empty()
    }
}

/// Options for formatting a filesystem
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FormatOptions {
    /// Filesystem label
    pub label: String,

    /// Force formatting even if filesystem appears to exist
    pub force: bool,

    /// Erase/wipe the device before formatting (secure erase)
    pub erase: bool,

    /// Enable discard/TRIM support
    pub discard: bool,

    /// Filesystem-specific options (key-value pairs)
    pub fs_specific: HashMap<String, String>,
}

/// Options for mounting a filesystem
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MountOptions {
    /// Mount read-only
    pub read_only: bool,

    /// Disallow execution of binaries
    pub no_exec: bool,

    /// Disallow setuid/setgid
    pub no_suid: bool,

    /// Other mount options as strings
    pub other: Vec<String>,
}

/// Persistent mount options (fstab-style configuration)
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountOptionsSettings {
    /// How to identify the device (e.g. device path or UUID=...)
    pub identify_as: String,
    /// Mount point path
    pub mount_point: String,
    /// Filesystem type (e.g. ext4, auto)
    pub filesystem_type: String,
    /// Mount at startup
    pub mount_at_startup: bool,
    /// Require authentication to mount
    pub require_auth: bool,
    /// Show in file manager / UI
    pub show_in_ui: bool,
    /// Other fstab options string
    pub other_options: String,
    /// Display name (x-gvfs-name)
    pub display_name: String,
    /// Icon name (x-gvfs-icon)
    pub icon_name: String,
    /// Symbolic icon name (x-gvfs-symbolic-icon)
    pub symbolic_icon_name: String,
}

/// Result of filesystem check (fsck)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckResult {
    /// Device that was checked
    pub device: String,

    /// Whether the filesystem is clean
    pub clean: bool,

    /// Number of errors corrected
    pub errors_corrected: u32,

    /// Number of errors that could not be corrected
    pub errors_uncorrected: u32,

    /// Full output from fsck command
    pub output: String,
}

/// Result of unmount operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnmountResult {
    /// Whether unmount succeeded
    pub success: bool,

    /// Error message (if failed)
    pub error: Option<String>,

    /// Processes blocking the unmount
    pub blocking_processes: Vec<ProcessInfo>,
}

/// Information about a process
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessInfo {
    /// Process ID
    pub pid: i32,

    /// Command/executable name
    pub command: String,

    /// User ID
    pub uid: u32,

    /// Username
    pub username: String,
}

/// Result of killing a process
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KillResult {
    /// Process ID that was targeted
    pub pid: i32,

    /// Whether the kill succeeded
    pub success: bool,

    /// Error message (if failed)
    pub error: Option<String>,
}

/// Supported filesystem types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilesystemType {
    /// ext4 filesystem
    Ext4,

    /// XFS filesystem
    Xfs,

    /// Btrfs filesystem
    Btrfs,

    /// FAT32 filesystem
    Fat32,

    /// NTFS filesystem
    Ntfs,

    /// exFAT filesystem
    Exfat,

    /// Other/unknown filesystem
    Other,
}

impl FilesystemType {
    /// Convert to mkfs command name
    pub fn mkfs_command(&self) -> &'static str {
        match self {
            Self::Ext4 => "mkfs.ext4",
            Self::Xfs => "mkfs.xfs",
            Self::Btrfs => "mkfs.btrfs",
            Self::Fat32 => "mkfs.vfat",
            Self::Ntfs => "mkfs.ntfs",
            Self::Exfat => "mkfs.exfat",
            Self::Other => "",
        }
    }

    /// Parse from filesystem type string
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "ext4" => Self::Ext4,
            "xfs" => Self::Xfs,
            "btrfs" => Self::Btrfs,
            "vfat" | "fat32" => Self::Fat32,
            "ntfs" => Self::Ntfs,
            "exfat" => Self::Exfat,
            _ => Self::Other,
        }
    }
}

/// Information about a filesystem formatting tool
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemToolInfo {
    /// Filesystem type identifier (e.g., "ext4", "btrfs")
    pub fs_type: String,

    /// Human-readable filesystem name (e.g., "EXT4", "Btrfs")
    pub fs_name: String,

    /// Command to check for availability (e.g., "mkfs.ext4")
    pub command: String,

    /// Package name hint for installation (e.g., "e2fsprogs")
    pub package_hint: String,

    /// Whether the tool is currently available on this system
    pub available: bool,
}

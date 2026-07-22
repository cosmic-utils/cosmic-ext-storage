//! Common utility types shared across models

use anyhow::Result;
use num_format::{Locale, ToFormattedString};
use serde::{Deserialize, Serialize};

/// GPT alignment boundary (1 MiB) - standard for modern disks
pub const GPT_ALIGNMENT_BYTES: u64 = 1024 * 1024;

/// A byte range representing a contiguous region (used for GPT usable space, etc.)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByteRange {
    /// Start byte (inclusive)
    pub start: u64,

    /// End byte (exclusive)
    pub end: u64,
}

impl ByteRange {
    /// Check if this range is valid for a disk of the given size
    pub fn is_valid_for_disk(&self, disk_size: u64) -> bool {
        self.start < self.end && self.end <= disk_size
    }

    /// Clamp this range to fit within a disk of the given size
    pub fn clamp_to_disk(&self, disk_size: u64) -> Self {
        let start = self.start.min(disk_size);
        let end = self.end.min(disk_size);
        Self { start, end }
    }

    /// Get the size of this range in bytes
    pub fn size(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }
}

/// Filesystem usage statistics
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    /// Filesystem type (e.g., "ext4", "btrfs", "xfs")
    pub filesystem: String,

    /// Total blocks
    pub blocks: u64,

    /// Used blocks
    pub used: u64,

    /// Available blocks
    pub available: u64,

    /// Usage percentage (0-100)
    pub percent: u32,

    /// Mount point where this filesystem is mounted
    pub mount_point: String,
}

impl Usage {
    /// Get total size in bytes (assuming standard 4K block size)
    pub fn total_bytes(&self) -> u64 {
        self.blocks * 4096
    }

    /// Get used size in bytes (assuming standard 4K block size)
    pub fn used_bytes(&self) -> u64 {
        self.used * 4096
    }

    /// Get available size in bytes (assuming standard 4K block size)
    pub fn available_bytes(&self) -> u64 {
        self.available * 4096
    }
}

/// Format utilities for converting between bytes and human-readable strings
/// Convert bytes to human-readable format (e.g., "1.50 GB")
pub fn bytes_to_pretty(bytes: &u64, add_bytes: bool) -> String {
    let mut steps = 0;
    let mut val: f64 = *bytes as f64;

    while val > 1024. && steps <= 8 {
        val /= 1024.;
        steps += 1;
    }

    let unit = match steps {
        0 => "B",
        1 => "KB",
        2 => "MB",
        3 => "GB",
        4 => "TB",
        5 => "PB",
        6 => "EB",
        7 => "ZB",
        8 => "YB",
        _ => "Not Supported",
    };

    if add_bytes {
        let bytes_str = bytes.to_formatted_string(&Locale::en);
        format!("{:.2} {} ({} bytes)", val, unit, bytes_str)
    } else {
        format!("{:.2} {}", val, unit)
    }
}

/// Parse human-readable format to bytes (e.g., "1.5 GB" -> bytes)
pub fn pretty_to_bytes(pretty: &str) -> Result<u64> {
    let split = pretty.split_whitespace().collect::<Vec<&str>>();
    let string_value = split
        .first()
        .ok_or_else(|| anyhow::anyhow!("Invalid input"))?;

    let mut val: f64 = string_value.parse()?;
    let unit = *split
        .last()
        .ok_or_else(|| anyhow::anyhow!("Invalid input"))?;

    let mut steps = match unit {
        "B" => 0,
        "KB" => 1,
        "MB" => 2,
        "GB" => 3,
        "TB" => 4,
        "PB" => 5,
        "EB" => 6,
        "ZB" => 7,
        "YB" => 8,
        _ => return Err(anyhow::anyhow!("Invalid unit: {}", unit)),
    };

    while steps > 0 {
        val *= 1024.;
        steps -= 1;
    }

    Ok(val as u64)
}

/// Get numeric value that would be displayed in bytes_to_pretty
pub fn get_numeric(bytes: &u64) -> f64 {
    let mut steps = 0;
    let mut val: f64 = *bytes as f64;

    while val > 1024. && steps <= 8 {
        val /= 1024.;
        steps += 1;
    }

    val
}

/// Return decent step value for numeric boxes based on displayed value
pub fn get_step(bytes: &u64) -> f64 {
    let mut denomination = 0;
    let mut val: f64 = *bytes as f64;

    while val > 1024. && denomination <= 8 {
        val /= 1024.;
        denomination += 1;
    }

    1024_f64.powi(denomination)
}

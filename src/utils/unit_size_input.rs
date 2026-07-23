// SPDX-License-Identifier: GPL-3.0-only

//! Unit-aware size input component for partition size inputs
//!
//! Provides conversions between bytes and human-readable units (KB, MB, GB, TB)
//! and helpers for rendering size input widgets with unit selectors.

use std::fmt;

/// Cached unit labels for dropdown to avoid repeated allocations
static UNIT_LABELS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

/// Size units for displaying and inputting partition sizes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeUnit {
    Bytes,
    Kilobytes,
    Megabytes,
    Gigabytes,
    Terabytes,
}

impl SizeUnit {
    /// Convert a value in this unit to bytes
    pub fn to_bytes(self, value: f64) -> u64 {
        // Validate input: require finite value
        if !value.is_finite() {
            return 0;
        }

        // Compute the value in bytes as f64
        let bytes = match self {
            SizeUnit::Bytes => value,
            SizeUnit::Kilobytes => value * 1024.0,
            SizeUnit::Megabytes => value * 1024.0 * 1024.0,
            SizeUnit::Gigabytes => value * 1024.0 * 1024.0 * 1024.0,
            SizeUnit::Terabytes => value * 1024.0 * 1024.0 * 1024.0 * 1024.0,
        };

        // Clamp to non-negative range
        if bytes <= 0.0 {
            return 0;
        }

        // Explicitly handle values that exceed u64::MAX
        let max_u64_as_f64 = u64::MAX as f64;
        if bytes >= max_u64_as_f64 {
            return u64::MAX;
        }

        // Explicit truncation toward zero before converting to u64
        bytes.trunc() as u64
    }

    /// Convert bytes to a value in this unit
    #[allow(clippy::wrong_self_convention)]
    pub fn from_bytes(&self, bytes: u64) -> f64 {
        match self {
            SizeUnit::Bytes => bytes as f64,
            SizeUnit::Kilobytes => bytes as f64 / 1024.0,
            SizeUnit::Megabytes => bytes as f64 / (1024.0 * 1024.0),
            SizeUnit::Gigabytes => bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            SizeUnit::Terabytes => bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0),
        }
    }

    /// Get the display label for this unit
    pub fn label(&self) -> &'static str {
        match self {
            SizeUnit::Bytes => "B",
            SizeUnit::Kilobytes => "KB",
            SizeUnit::Megabytes => "MB",
            SizeUnit::Gigabytes => "GB",
            SizeUnit::Terabytes => "TB",
        }
    }

    /// Get all unit labels as a static slice
    pub fn labels() -> &'static [&'static str] {
        UNIT_LABELS
    }

    /// Create a unit from a dropdown index
    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => SizeUnit::Bytes,
            1 => SizeUnit::Kilobytes,
            2 => SizeUnit::Megabytes,
            3 => SizeUnit::Gigabytes,
            4 => SizeUnit::Terabytes,
            _ => SizeUnit::Megabytes, // default to MB
        }
    }

    /// Get the dropdown index for this unit
    pub fn to_index(self) -> usize {
        match self {
            SizeUnit::Bytes => 0,
            SizeUnit::Kilobytes => 1,
            SizeUnit::Megabytes => 2,
            SizeUnit::Gigabytes => 3,
            SizeUnit::Terabytes => 4,
        }
    }

    /// Pick an appropriate default unit for a given size in bytes
    pub fn auto_select(bytes: u64) -> Self {
        if bytes < 1024 {
            SizeUnit::Bytes
        } else if bytes < 1024 * 1024 {
            SizeUnit::Kilobytes
        } else if bytes < 1024 * 1024 * 1024 {
            SizeUnit::Megabytes
        } else if bytes < 1024u64 * 1024 * 1024 * 1024 {
            SizeUnit::Gigabytes
        } else {
            SizeUnit::Terabytes
        }
    }
}

impl fmt::Display for SizeUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_identity() {
        let bytes = 12345u64;
        assert_eq!(SizeUnit::Bytes.to_bytes(bytes as f64), bytes);
        assert_eq!(SizeUnit::Bytes.from_bytes(bytes), bytes as f64);
    }

    #[test]
    fn test_megabytes_to_bytes() {
        let mb = 100.0;
        let expected = 104857600u64; // 100 * 1024 * 1024
        assert_eq!(SizeUnit::Megabytes.to_bytes(mb), expected);
    }

    #[test]
    fn test_bytes_to_megabytes() {
        let bytes = 104857600u64; // 100 MB
        let mb = SizeUnit::Megabytes.from_bytes(bytes);
        assert!((mb - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_gigabytes_to_bytes() {
        let gb = 5.0;
        let expected = 5368709120u64; // 5 * 1024^3
        assert_eq!(SizeUnit::Gigabytes.to_bytes(gb), expected);
    }

    #[test]
    fn test_bytes_to_gigabytes() {
        let bytes = 1073741824u64; // 1 GB
        let gb = SizeUnit::Gigabytes.from_bytes(bytes);
        assert!((gb - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_terabytes_conversion() {
        let tb = 2.5;
        let bytes = SizeUnit::Terabytes.to_bytes(tb);
        let back_to_tb = SizeUnit::Terabytes.from_bytes(bytes);
        assert!((back_to_tb - tb).abs() < 0.001);
    }

    #[test]
    fn test_unit_index_roundtrip() {
        for unit in [
            SizeUnit::Bytes,
            SizeUnit::Kilobytes,
            SizeUnit::Megabytes,
            SizeUnit::Gigabytes,
            SizeUnit::Terabytes,
        ] {
            let idx = unit.to_index();
            let recovered = SizeUnit::from_index(idx);
            assert_eq!(unit, recovered);
        }
    }

    #[test]
    fn test_auto_select() {
        assert_eq!(SizeUnit::auto_select(512), SizeUnit::Bytes);
        assert_eq!(SizeUnit::auto_select(2048), SizeUnit::Kilobytes);
        assert_eq!(SizeUnit::auto_select(5 * 1024 * 1024), SizeUnit::Megabytes);
        assert_eq!(
            SizeUnit::auto_select(3 * 1024 * 1024 * 1024),
            SizeUnit::Gigabytes
        );
        assert_eq!(
            SizeUnit::auto_select(2 * 1024u64 * 1024 * 1024 * 1024),
            SizeUnit::Terabytes
        );
    }

    #[test]
    fn test_labels() {
        let labels = SizeUnit::labels();
        assert_eq!(labels.len(), 5);
        assert_eq!(labels[0], "B");
        assert_eq!(labels[1], "KB");
        assert_eq!(labels[2], "MB");
        assert_eq!(labels[3], "GB");
        assert_eq!(labels[4], "TB");
    }
}

// SPDX-License-Identifier: GPL-3.0-only

//! SMART (Self-Monitoring, Analysis and Reporting Technology) types
//!
//! Types for device health monitoring and SMART data.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Kind of SMART self-test to run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmartSelfTestKind {
    /// Short self-test (usually a few minutes)
    Short,
    /// Extended self-test (can take hours for large drives)
    Extended,
}

impl SmartSelfTestKind {
    /// Convert to UDisks2 string representation
    pub fn as_udisks_str(self) -> &'static str {
        match self {
            Self::Short => "short",
            Self::Extended => "extended",
        }
    }

    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "short" => Some(Self::Short),
            "extended" | "long" => Some(Self::Extended),
            _ => None,
        }
    }
}

/// SMART/health information for a storage device
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SmartInfo {
    /// Human-readable identifier for the backend interface providing SMART.
    /// Examples: "NVMe", "ATA".
    pub device_type: String,

    /// Seconds since epoch (UTC) when SMART data was last updated (if available).
    pub updated_at: Option<u64>,

    /// Temperature in Celsius (if available).
    pub temperature_c: Option<u64>,

    /// Power-on hours (if available).
    pub power_on_hours: Option<u64>,

    /// Self-test status (if available).
    pub selftest_status: Option<String>,

    /// Additional attributes (key → stringified value), ordered by key.
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

// SPDX-License-Identifier: GPL-3.0-only

//! SMART types - re-exported from storage-types
//!
//! Domain types are defined in storage-types; this module re-exports them
//! and provides internal helpers for UDisks2-specific conversions.

// Re-export canonical domain types from storage-types
pub use storage_types::{SmartInfo, SmartSelfTestKind};

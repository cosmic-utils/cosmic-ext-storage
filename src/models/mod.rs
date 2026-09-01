// SPDX-License-Identifier: GPL-3.0-only

//! UI-specific models with owned clients and hierarchical structure
//!
//! These models wrap storage-types models (DiskInfo, VolumeInfo, PartitionInfo)
//! and add UI-specific functionality:
//! - Owned D-Bus clients for refreshing data
//! - Tree-building from flat lists using parent_path
//! - Helper methods for navigation and querying
//! - Atomic update support for performance

pub mod helpers;
pub mod load;
pub mod ui_drive;
pub mod ui_volume;

pub use helpers::build_volume_tree;
pub use load::{build_drive_timed, load_all_drives, load_drive_candidates};
pub use ui_drive::UiDrive;
pub use ui_volume::UiVolume;

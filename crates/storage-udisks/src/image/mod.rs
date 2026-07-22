//! Image and backup operations
//!
//! This module provides operations for working with disk images:
//! - Loop device setup
//! - Image mounting
//! - Backup and restore operations

pub mod backup;
pub mod loop_setup;
pub(crate) mod udisks_call;

pub use backup::*;
pub use loop_setup::*;

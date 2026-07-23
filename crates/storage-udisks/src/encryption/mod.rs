//! LUKS encryption operations
//!
//! This module provides operations for managing LUKS encrypted devices:
//! - Formatting LUKS devices
//! - Unlocking and locking
//! - Passphrase management
//! - Listing encrypted devices

pub mod config;
mod format;
mod list;
mod lock;
mod passphrase;
mod unlock;

pub use config::EncryptionOptionsSettings;
pub use format::format_luks;
pub use list::list_luks_devices;
pub use lock::lock_luks;
pub use passphrase::change_luks_passphrase;
pub use unlock::{get_cleartext_device, unlock_luks};

//! Encryption types (LUKS)
//!
//! Types for LUKS encrypted volume management.

use serde::{Deserialize, Serialize};

/// LUKS version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuksVersion {
    /// LUKS version 1
    Luks1,

    /// LUKS version 2
    Luks2,
}

impl LuksVersion {
    /// Convert to UDisks2/cryptsetup string format
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Luks1 => "luks1",
            Self::Luks2 => "luks2",
        }
    }

    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "luks1" | "1" => Some(Self::Luks1),
            "luks2" | "2" => Some(Self::Luks2),
            _ => None,
        }
    }
}

/// LUKS encrypted volume information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LuksInfo {
    /// Device path of encrypted container (e.g., "/dev/sda1")
    pub device: String,

    /// LUKS version
    pub version: LuksVersion,

    /// Cipher algorithm (e.g., "aes-xts-plain64")
    pub cipher: String,

    /// Key size in bits
    pub key_size: u32,

    /// Whether the container is currently unlocked
    pub unlocked: bool,

    /// Cleartext device path (e.g., "/dev/mapper/luks-xxx") if unlocked
    pub cleartext_device: Option<String>,

    /// Number of keyslots
    pub keyslot_count: u8,
}

impl LuksInfo {
    /// Check if this LUKS container can be unlocked
    pub fn can_unlock(&self) -> bool {
        !self.unlocked
    }

    /// Check if this LUKS container can be locked
    pub fn can_lock(&self) -> bool {
        self.unlocked
    }
}

/// Encryption options (e.g. crypttab) for a LUKS volume
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptionOptionsSettings {
    /// Mapper name (e.g. for /dev/mapper/name)
    pub name: String,
    /// Unlock at system startup
    pub unlock_at_startup: bool,
    /// Require authentication to unlock
    pub require_auth: bool,
    /// Other crypttab options (e.g. "nofail")
    pub other_options: String,
    /// Optional passphrase to store (e.g. in /etc/luks-keys/) for unlock at startup
    pub passphrase: Option<String>,
}

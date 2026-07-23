// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

/// Information about a BTRFS subvolume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtrfsSubvolume {
    pub id: u64,
    pub path: String,
    pub parent_id: Option<u64>,
    pub uuid: String,
    pub parent_uuid: Option<String>,
    pub received_uuid: Option<String>,
    pub generation: u64,
    pub ctransid: u64,
    pub otransid: u64,
    pub stransid: Option<u64>,
    pub rtransid: Option<u64>,
    pub ctime: i64,         // Unix timestamp
    pub otime: i64,         // Unix timestamp
    pub stime: Option<i64>, // Unix timestamp
    pub rtime: Option<i64>, // Unix timestamp
    pub flags: u64,
}

/// Filesystem usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemUsage {
    pub used_bytes: u64,
}

/// Response containing subvolumes and default ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubvolumeList {
    pub subvolumes: Vec<BtrfsSubvolume>,
    pub default_id: u64,
}

/// Deleted subvolume entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletedSubvolume {
    pub id: u64,
    pub path: String,
}

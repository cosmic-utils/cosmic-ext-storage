//! Partition type catalog and utilities
//!
//! Provides partition type information for GPT and DOS/MBR partition tables.
//! Used by both UI (for type selection) and service (for type validation).

mod catalog;
mod query;

use serde::Deserialize;

pub use catalog::{COMMON_DOS_TYPES, COMMON_GPT_TYPES, PARTITION_TYPES};
pub use query::{get_all_partition_type_infos, get_valid_partition_names};

/// Flags describing a partition type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PartitionTypeInfoFlags {
    /// No flags set.
    #[default]
    None = 0,
    /// Partition type is used for swap.
    Swap = (1 << 0),
    /// Partition type is used for RAID/LVM or similar.
    Raid = (1 << 1),
    /// Partition type indicates the partition is hidden
    /// (e.g. 'dos' type 0x1b PartitionType::new("Hidden W95 FAT32").
    /// Note that this is not the same as user-toggleable
    /// attributes/flags for a partition.
    Hidden = (1 << 2),
    /// Partition type can only be used when creating a partition
    /// and e.g. should not be selectable in a "change partition type"
    /// user interface (e.g. 'dos' type 0x05, 0x0f and 0x85
    /// for extended partitions).
    CreateOnly = (1 << 3),
    /// Partition type indicates the partition is part of the system / bootloader (e.g. 'dos' types 0xee, 0xff, 'gpt' types for 'EFI System partition' and 'BIOS Boot partition').
    System = (1 << 4),
}

impl<'de> Deserialize<'de> for PartitionTypeInfoFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "" => Ok(PartitionTypeInfoFlags::None),
            "None" => Ok(PartitionTypeInfoFlags::None),
            "Swap" => Ok(PartitionTypeInfoFlags::Swap),
            "Raid" => Ok(PartitionTypeInfoFlags::Raid),
            "Hidden" => Ok(PartitionTypeInfoFlags::Hidden),
            "CreateOnly" => Ok(PartitionTypeInfoFlags::CreateOnly),
            "System" => Ok(PartitionTypeInfoFlags::System),
            _ => Err(serde::de::Error::custom(format!("Unknown flag: {}", s))),
        }
    }
}

/// Detailed information about a partition type.
///
/// `table_subtype` is used to break the set of partition types for
/// `table_type` into a logical subsets. It is typically only used in
/// user interfaces where the partition type is selected.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct PartitionTypeInfo {
    /// A partition table type e.g. `dos` or `gpt`
    pub table_type: String,
    /// A partition table sub-type
    pub table_subtype: String,
    /// A partition type
    pub ty: String,
    /// Name of the partition
    pub name: String,
    /// Flags describing the partition type
    pub flags: PartitionTypeInfoFlags,
    /// Default filesystem type for this partition type
    pub filesystem_type: String,
}

impl PartitionTypeInfo {
    pub fn find_by_id(type_id: String) -> Option<PartitionTypeInfo> {
        query::find_by_id(type_id)
    }
}

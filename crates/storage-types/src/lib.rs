// SPDX-License-Identifier: GPL-3.0-only

//! Canonical domain models for COSMIC Ext Storage management
//!
//! This crate defines the single source of truth for all storage domain types.
//! These models are used throughout the stack:
//!
//! - **storage-udisks**: Returns these types directly from its public API
//! - **in-process operations**: Serializes/deserializes these types for D-Bus transport
//! - **COSMIC Storage application**: Consumes these types, optionally wrapping them for UI state
//!
//! ## Architecture
//!
//! The type system supports two hierarchies:
//!
//! ### Flat Hierarchy (for operations)
//! - `DiskInfo` → physical disk metadata
//! - `PartitionInfo` → partition metadata
//! - `FilesystemInfo` → filesystem details
//!
//! ### Tree Hierarchy (for UI display)
//! - `VolumeInfo` → recursive tree structure containing any `VolumeKind`
//!
//! This eliminates circular conversions and ensures data consistency across all components.

pub mod btrfs;
pub mod common;
pub mod disk;
pub mod encryption;
pub mod filesystem;
pub mod lvm;
pub mod network;
pub mod partition;
pub mod partition_types;
pub mod rclone;
pub mod smart;
pub mod usage_scan;
pub mod volume;

pub use btrfs::{BtrfsSubvolume, DeletedSubvolume, FilesystemUsage, SubvolumeList};
pub use common::{
    ByteRange, GPT_ALIGNMENT_BYTES, Usage, bytes_to_pretty, get_numeric, get_step, pretty_to_bytes,
};
pub use disk::{
    BackendId, DeviceEvent, DiskEvent, DiskInfo, SmartAttribute, SmartStatus,
    StorageBackendCapabilities,
};
pub use encryption::{EncryptionOptionsSettings, LuksInfo, LuksVersion};
pub use filesystem::{
    CheckResult, FilesystemInfo, FilesystemToolInfo, FilesystemType, FormatOptions, KillResult,
    MountOptions, MountOptionsSettings, ProcessInfo, UnmountResult,
};
pub use lvm::{LogicalVolumeInfo, PhysicalVolumeInfo, VolumeGroupInfo};
pub use network::{
    NetworkBackendAvailability, NetworkBackendId, NetworkDriveCapabilities, NetworkDriveConfig,
    NetworkDriveConfigurationSchema, NetworkDriveField, NetworkDriveFieldExample,
    NetworkDriveFieldInputKind, NetworkDriveList, NetworkDriveMount, NetworkDriveProviderSchema,
    NetworkDriveStatus, NetworkDriveTestResult,
};
pub use partition::{
    CreatePartitionInfo, PartitionInfo, PartitionTableInfo, PartitionTableType,
    make_partition_flags_bits,
};
pub use partition_types::{
    COMMON_DOS_TYPES, COMMON_GPT_TYPES, PARTITION_TYPES, PartitionTypeInfo, PartitionTypeInfoFlags,
    get_all_partition_type_infos, get_valid_partition_names,
};
pub use rclone::{
    ConfigScope, MountStatus, MountStatusResult, MountType, NetworkMount, RcloneProvider,
    RcloneProviderOption, RcloneProviderOptionExample, RemoteConfig, RemoteConfigList, TestResult,
    rclone_provider, rclone_providers, supported_remote_types,
};
pub use smart::{SmartInfo, SmartSelfTestKind};
pub use usage_scan::{
    UsageCategory, UsageCategoryTopFiles, UsageCategoryTotal, UsageDeleteFailure,
    UsageDeleteResult, UsageScanParallelismPreset, UsageScanRequest, UsageScanResult,
    UsageTopFileEntry,
};
pub use volume::{VolumeInfo, VolumeKind, VolumeType};

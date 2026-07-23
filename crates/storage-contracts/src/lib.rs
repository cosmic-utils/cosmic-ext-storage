// SPDX-License-Identifier: GPL-3.0-only

pub mod protocol;
pub mod traits;

pub use protocol::{
    OperationEvent, OperationId, OperationKind, OperationProgress, StorageError, StorageErrorKind,
};
pub use traits::{
    BackendMetadata, BlockStorageBackend, BtrfsBackend, BtrfsOperations, BtrfsResizeRequest,
    ConfigurationCleanupPolicy, DeviceEventSource, DiskDiscovery, DriveOperations,
    EncryptionOperations, FilesystemOperations, ImageDeviceOperations, LogicalAction,
    LogicalActionOutcome, LogicalOperations, LogicalTopologySource, LvmWipePolicy,
    MdRaidCreateProfile, MdRaidLevel, MdRaidMemberWipePolicy, MdRaidName, MdRaidSyncAction,
    NetworkDriveBackend, PartitionOperations,
};

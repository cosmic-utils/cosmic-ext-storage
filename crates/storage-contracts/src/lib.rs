// SPDX-License-Identifier: GPL-3.0-only

pub mod protocol;
pub mod traits;

pub use protocol::{
    OperationEvent, OperationId, OperationKind, OperationProgress, StorageError, StorageErrorKind,
};
pub use traits::{
    BackendMetadata, BlockStorageBackend, BtrfsBackend, BtrfsOperations, BtrfsResizeRequest,
    ByteSizeConstraint, CandidateBlockReason, ConfigurationCleanupPolicy, ConfirmedLogicalAction,
    DestructiveScopePolicy, DeviceEventSource, DiskDiscovery, DriveOperations,
    EncryptionOperations, FilesystemOperations, ImageDeviceOperations, LogicalAction,
    LogicalActionKind, LogicalActionOutcome, LogicalCandidateDisplay, LogicalDeviceCandidate,
    LogicalInputConstraints, LogicalOperations, LogicalPreflight, LogicalPreflightAvailability,
    LogicalPreflightKey, LogicalPreflightRequest, LogicalPreflightRequestKey,
    LogicalPreflightTarget, LogicalReviewData, LogicalTopologySource, LvmWipePolicy,
    MdRaidCreateProfile, MdRaidLevel, MdRaidMemberWipePolicy, MdRaidName, MdRaidProfileOption,
    MdRaidSyncAction, MemberRemovalPolicy, NetworkDriveBackend, PartitionOperations,
};

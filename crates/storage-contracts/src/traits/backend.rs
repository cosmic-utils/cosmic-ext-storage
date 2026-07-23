// SPDX-License-Identifier: GPL-3.0-only

use storage_types::{BackendId, StorageBackendCapabilities};

use super::{
    BtrfsOperations, DeviceEventSource, DiskDiscovery, DriveOperations, EncryptionOperations,
    FilesystemOperations, ImageDeviceOperations, PartitionOperations,
};

pub trait BackendMetadata: Send + Sync {
    fn id(&self) -> BackendId;
    fn capabilities(&self) -> StorageBackendCapabilities;
}

pub trait BlockStorageBackend:
    BackendMetadata
    + DiskDiscovery
    + DeviceEventSource
    + DriveOperations
    + PartitionOperations
    + FilesystemOperations
    + EncryptionOperations
    + ImageDeviceOperations
    + Send
    + Sync
{
}

impl<T> BlockStorageBackend for T where
    T: BackendMetadata
        + DiskDiscovery
        + DeviceEventSource
        + DriveOperations
        + PartitionOperations
        + FilesystemOperations
        + EncryptionOperations
        + ImageDeviceOperations
        + Send
        + Sync
{
}

pub trait BtrfsBackend: BackendMetadata + BtrfsOperations + Send + Sync {}

impl<T> BtrfsBackend for T where T: BackendMetadata + BtrfsOperations + Send + Sync {}

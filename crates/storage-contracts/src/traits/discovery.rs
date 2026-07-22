// SPDX-License-Identifier: GPL-3.0-only

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use storage_types::{DeviceEvent, DiskInfo, VolumeInfo};

use crate::StorageError;

/// Discover local drives and their typed volume trees.
#[async_trait]
pub trait DiskDiscovery: Send + Sync {
    async fn list_disks(&self) -> Result<Vec<DiskInfo>, StorageError>;
    async fn list_volumes(&self) -> Result<Vec<VolumeInfo>, StorageError>;
}

/// Subscribe to external device changes without exposing the transport used to
/// deliver them.
#[async_trait]
pub trait DeviceEventSource: Send + Sync {
    async fn device_events(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<DeviceEvent, StorageError>> + Send>>, StorageError>;
}

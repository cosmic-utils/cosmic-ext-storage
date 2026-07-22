// SPDX-License-Identifier: GPL-3.0-only

use std::os::fd::OwnedFd;

use async_trait::async_trait;

use crate::StorageError;

#[async_trait]
pub trait ImageDeviceOperations: Send + Sync {
    async fn open_for_backup(&self, device: &str) -> Result<OwnedFd, StorageError>;
    async fn open_for_restore(&self, device: &str) -> Result<OwnedFd, StorageError>;
    async fn loop_setup(&self, image_path: &str) -> Result<String, StorageError>;
}

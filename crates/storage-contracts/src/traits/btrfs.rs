// SPDX-License-Identifier: GPL-3.0-only

use async_trait::async_trait;
use storage_types::{DeletedSubvolume, FilesystemUsage, SubvolumeList};

use crate::StorageError;

#[async_trait]
pub trait BtrfsOperations: Send + Sync {
    async fn list_subvolumes(&self, mountpoint: &str) -> Result<SubvolumeList, StorageError>;
    async fn create_subvolume(&self, mountpoint: &str, name: &str) -> Result<(), StorageError>;
    async fn create_snapshot(
        &self,
        mountpoint: &str,
        source: &str,
        destination: &str,
        readonly: bool,
    ) -> Result<(), StorageError>;
    async fn delete_subvolume(
        &self,
        mountpoint: &str,
        path: &str,
        recursive: bool,
    ) -> Result<(), StorageError>;
    async fn set_readonly(
        &self,
        mountpoint: &str,
        path: &str,
        readonly: bool,
    ) -> Result<(), StorageError>;
    async fn set_default(&self, mountpoint: &str, path: &str) -> Result<(), StorageError>;
    async fn default_subvolume(&self, mountpoint: &str) -> Result<u64, StorageError>;
    async fn deleted_subvolumes(
        &self,
        mountpoint: &str,
    ) -> Result<Vec<DeletedSubvolume>, StorageError>;
    async fn filesystem_usage(&self, mountpoint: &str) -> Result<FilesystemUsage, StorageError>;
}

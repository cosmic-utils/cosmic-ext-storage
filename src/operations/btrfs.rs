// SPDX-License-Identifier: GPL-3.0-only

use super::{OperationError, StorageOperations, shared};
use std::sync::Arc;
use storage_types::btrfs::{DeletedSubvolume, FilesystemUsage, SubvolumeList};

#[derive(Clone, Debug)]
pub struct BtrfsClient(Arc<StorageOperations>);
impl BtrfsClient {
    pub async fn new() -> Result<Self, OperationError> {
        Ok(Self(shared().await?))
    }
    fn backend(&self) -> Result<&Arc<dyn storage_contracts::BtrfsBackend>, OperationError> {
        self.0
            .registry
            .btrfs
            .as_ref()
            .ok_or_else(|| OperationError::Unsupported("Btrfs tooling is unavailable".into()))
    }
    pub async fn list_subvolumes(&self, mount: &str) -> Result<SubvolumeList, OperationError> {
        self.backend()?
            .list_subvolumes(mount)
            .await
            .map_err(Into::into)
    }
    pub async fn create_subvolume(&self, mount: &str, name: &str) -> Result<(), OperationError> {
        self.backend()?
            .create_subvolume(mount, name)
            .await
            .map_err(Into::into)
    }
    pub async fn create_snapshot(
        &self,
        mount: &str,
        source: &str,
        destination: &str,
        readonly: bool,
    ) -> Result<(), OperationError> {
        self.backend()?
            .create_snapshot(mount, source, destination, readonly)
            .await
            .map_err(Into::into)
    }
    pub async fn delete_subvolume(
        &self,
        mount: &str,
        path: &str,
        recursive: bool,
    ) -> Result<(), OperationError> {
        self.backend()?
            .delete_subvolume(mount, path, recursive)
            .await
            .map_err(Into::into)
    }
    pub async fn set_readonly(
        &self,
        mount: &str,
        path: &str,
        readonly: bool,
    ) -> Result<(), OperationError> {
        self.backend()?
            .set_readonly(mount, path, readonly)
            .await
            .map_err(Into::into)
    }
    pub async fn set_default(&self, mount: &str, path: &str) -> Result<(), OperationError> {
        self.backend()?
            .set_default(mount, path)
            .await
            .map_err(Into::into)
    }
    pub async fn get_default(&self, mount: &str) -> Result<u64, OperationError> {
        self.backend()?
            .default_subvolume(mount)
            .await
            .map_err(Into::into)
    }
    pub async fn list_deleted(&self, mount: &str) -> Result<Vec<DeletedSubvolume>, OperationError> {
        self.backend()?
            .deleted_subvolumes(mount)
            .await
            .map_err(Into::into)
    }
    pub async fn get_usage(&self, mount: &str) -> Result<FilesystemUsage, OperationError> {
        self.backend()?
            .filesystem_usage(mount)
            .await
            .map_err(Into::into)
    }
}

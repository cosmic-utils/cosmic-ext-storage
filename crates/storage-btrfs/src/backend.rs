// SPDX-License-Identifier: GPL-3.0-only

//! Btrfs-util implementation of the backend-neutral Btrfs contract.

use std::path::Path;

use async_trait::async_trait;
use storage_contracts::{BackendMetadata, BtrfsOperations, StorageError, StorageErrorKind};
use storage_types::{
    BackendId, DeletedSubvolume, FilesystemUsage, StorageBackendCapabilities, SubvolumeList,
};

use crate::{SubvolumeManager, get_filesystem_usage};

#[derive(Debug, Default)]
pub struct BtrfsUtilBackend;

impl BtrfsUtilBackend {
    pub fn new() -> Self {
        Self
    }

    fn manager(mountpoint: &str) -> Result<SubvolumeManager, StorageError> {
        SubvolumeManager::new(mountpoint).map_err(Self::error)
    }

    fn error(error: impl std::fmt::Display) -> StorageError {
        StorageError::new(StorageErrorKind::Internal, error.to_string())
    }
}

#[async_trait]
impl BtrfsOperations for BtrfsUtilBackend {
    async fn list_subvolumes(&self, mountpoint: &str) -> Result<SubvolumeList, StorageError> {
        let manager = Self::manager(mountpoint)?;
        let subvolumes = manager.list_all().map_err(Self::error)?;
        let default_id = manager.get_default().map_err(Self::error)?;
        Ok(SubvolumeList {
            subvolumes,
            default_id,
        })
    }

    async fn create_subvolume(&self, mountpoint: &str, name: &str) -> Result<(), StorageError> {
        Self::manager(mountpoint)?.create(name).map_err(Self::error)
    }

    async fn create_snapshot(
        &self,
        mountpoint: &str,
        source: &str,
        destination: &str,
        readonly: bool,
    ) -> Result<(), StorageError> {
        Self::manager(mountpoint)?
            .snapshot(Path::new(source), Path::new(destination), readonly, false)
            .map_err(Self::error)
    }

    async fn delete_subvolume(
        &self,
        mountpoint: &str,
        path: &str,
        recursive: bool,
    ) -> Result<(), StorageError> {
        Self::manager(mountpoint)?
            .delete(Path::new(path), recursive)
            .map_err(Self::error)
    }

    async fn set_readonly(
        &self,
        mountpoint: &str,
        path: &str,
        readonly: bool,
    ) -> Result<(), StorageError> {
        Self::manager(mountpoint)?
            .set_readonly(Path::new(path), readonly)
            .map_err(Self::error)
    }

    async fn set_default(&self, mountpoint: &str, path: &str) -> Result<(), StorageError> {
        Self::manager(mountpoint)?
            .set_default(Path::new(path))
            .map_err(Self::error)
    }

    async fn default_subvolume(&self, mountpoint: &str) -> Result<u64, StorageError> {
        Self::manager(mountpoint)?
            .get_default()
            .map_err(Self::error)
    }

    async fn deleted_subvolumes(
        &self,
        mountpoint: &str,
    ) -> Result<Vec<DeletedSubvolume>, StorageError> {
        Ok(Self::manager(mountpoint)?
            .list_deleted()
            .map_err(Self::error)?
            .into_iter()
            .map(|subvolume| DeletedSubvolume {
                id: subvolume.id,
                path: subvolume.path,
            })
            .collect())
    }

    async fn filesystem_usage(&self, mountpoint: &str) -> Result<FilesystemUsage, StorageError> {
        get_filesystem_usage(Path::new(mountpoint)).map_err(Self::error)
    }
}

impl BackendMetadata for BtrfsUtilBackend {
    fn id(&self) -> BackendId {
        BackendId::new("btrfsutil")
    }

    fn capabilities(&self) -> StorageBackendCapabilities {
        StorageBackendCapabilities::default()
    }
}

// SPDX-License-Identifier: GPL-3.0-only

use super::{OperationError, StorageOperations, shared};
use std::{num::NonZeroU32, sync::Arc};
use storage_contracts::LogicalAction;
use storage_types::{
    LogicalEntity, LogicalEntityId, LogicalEntityKind, LogicalOperation, LogicalSource,
    LogicalTopology,
    btrfs::{BtrfsSubvolume, FilesystemUsage, SubvolumeList},
};

const UDISKS_BTRFS_UNAVAILABLE: &str = "UDisks Btrfs support is unavailable for this filesystem. Install or enable the udisks2 Btrfs module.";

fn native_btrfs_filesystem<'a>(
    topology: &'a LogicalTopology,
    device_path: &str,
) -> Result<&'a LogicalEntity, OperationError> {
    if let Some(filesystem) = topology.entities.iter().find(|entity| {
        entity.kind == LogicalEntityKind::BtrfsFilesystem
            && entity.device_path.as_deref() == Some(device_path)
            && entity.capabilities.is_supported(LogicalOperation::Create)
    }) {
        return Ok(filesystem);
    }

    if let Some(reason) = topology
        .sources
        .iter()
        .find(|status| status.source == LogicalSource::Udisks)
        .and_then(|status| status.availability.reason())
    {
        return Err(OperationError::Failed(reason.into()));
    }

    Err(OperationError::Failed(UDISKS_BTRFS_UNAVAILABLE.into()))
}

#[derive(Clone, Debug)]
pub struct BtrfsClient(Arc<StorageOperations>);

impl BtrfsClient {
    pub async fn new() -> Result<Self, OperationError> {
        Ok(Self(shared().await?))
    }

    fn usage_backend(&self) -> Result<&Arc<dyn storage_contracts::BtrfsBackend>, OperationError> {
        self.0.registry.btrfs.as_ref().ok_or_else(|| {
            OperationError::Unsupported("Btrfs usage tooling is unavailable.".into())
        })
    }

    /// Resolve the filesystem through the UDisks Btrfs plugin.  All Btrfs
    /// mutations and subvolume discovery use this route so UDisks can perform
    /// its normal Polkit authorization; the legacy direct `btrfs` CLI is not
    /// used as an unprivileged fallback.
    async fn native_filesystem(
        &self,
        device_path: &str,
    ) -> Result<LogicalEntityId, OperationError> {
        let topology = self.0.load_logical_topology().await?;
        native_btrfs_filesystem(&topology, device_path).map(|entity| entity.id.clone())
    }

    /// Read Btrfs subvolumes from UDisks' Btrfs module rather than invoking
    /// the host `btrfs` command.
    pub async fn list_native_subvolumes(
        &self,
        device_path: &str,
    ) -> Result<SubvolumeList, OperationError> {
        let topology = self.0.load_logical_topology().await?;
        let filesystem = native_btrfs_filesystem(&topology, device_path)?;
        let default_id = filesystem
            .metadata
            .get("default_subvolume_id")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or_default();
        let mut subvolumes = topology
            .entities
            .iter()
            .filter(|entity| {
                entity.kind == LogicalEntityKind::BtrfsSubvolume
                    && entity.parent_id.as_ref() == Some(&filesystem.id)
            })
            .filter_map(|entity| {
                entity
                    .metadata
                    .get("subvolume_id")
                    .and_then(|id| id.parse::<u64>().ok())
                    .map(|id| BtrfsSubvolume {
                        id,
                        path: entity.name.clone(),
                        parent_id: None,
                        uuid: String::new(),
                        parent_uuid: None,
                        received_uuid: None,
                        generation: 0,
                        ctransid: 0,
                        otransid: 0,
                        stransid: None,
                        rtransid: None,
                        ctime: 0,
                        otime: 0,
                        stime: None,
                        rtime: None,
                        flags: 0,
                    })
            })
            .collect::<Vec<_>>();
        subvolumes.sort_by_key(|subvolume| subvolume.id);
        Ok(SubvolumeList {
            subvolumes,
            default_id,
        })
    }

    pub async fn create_subvolume(
        &self,
        device_path: &str,
        name: &str,
    ) -> Result<(), OperationError> {
        let filesystem = self.native_filesystem(device_path).await?;
        self.0
            .execute_logical_action(LogicalAction::CreateBtrfsSubvolume {
                filesystem,
                name: name.into(),
            })
            .await
            .map(|_| ())
    }

    pub async fn create_snapshot(
        &self,
        device_path: &str,
        source: &str,
        destination: &str,
        readonly: bool,
    ) -> Result<(), OperationError> {
        let filesystem = self.native_filesystem(device_path).await?;
        self.0
            .execute_logical_action(LogicalAction::CreateBtrfsSnapshot {
                filesystem,
                source: source.into(),
                destination: destination.into(),
                readonly,
            })
            .await
            .map(|_| ())
    }

    pub async fn delete_subvolume(
        &self,
        device_path: &str,
        path: &str,
        recursive: bool,
    ) -> Result<(), OperationError> {
        if recursive {
            return Err(OperationError::Unsupported(
                "Recursive Btrfs subvolume deletion has no native UDisks mapping.".into(),
            ));
        }
        let filesystem = self.native_filesystem(device_path).await?;
        self.0
            .execute_logical_action(LogicalAction::DeleteBtrfsSubvolume {
                filesystem,
                path: path.into(),
            })
            .await
            .map(|_| ())
    }

    pub async fn set_default(
        &self,
        device_path: &str,
        subvolume_id: u64,
    ) -> Result<(), OperationError> {
        let subvolume_id = u32::try_from(subvolume_id)
            .ok()
            .and_then(NonZeroU32::new)
            .ok_or_else(|| OperationError::InvalidInput("Invalid Btrfs subvolume ID.".into()))?;
        let filesystem = self.native_filesystem(device_path).await?;
        self.0
            .execute_logical_action(LogicalAction::SetBtrfsDefaultSubvolume {
                filesystem,
                subvolume_id,
            })
            .await
            .map(|_| ())
    }

    /// Usage is a local `statvfs` read and requires neither an elevated CLI
    /// call nor a Polkit request.
    pub async fn get_usage(&self, mount: &str) -> Result<FilesystemUsage, OperationError> {
        self.usage_backend()?
            .filesystem_usage(mount)
            .await
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_message_is_single_actionable_sentence() {
        assert_eq!(
            UDISKS_BTRFS_UNAVAILABLE,
            "UDisks Btrfs support is unavailable for this filesystem. Install or enable the udisks2 Btrfs module."
        );
    }

    #[test]
    fn missing_native_plugin_does_not_fall_back_to_the_btrfs_cli() {
        let error = native_btrfs_filesystem(&LogicalTopology::default(), "/dev/mapper/root")
            .expect_err("an empty topology has no native Btrfs interface");
        assert_eq!(error.to_string(), UDISKS_BTRFS_UNAVAILABLE);
    }
}

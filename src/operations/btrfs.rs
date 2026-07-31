// SPDX-License-Identifier: GPL-3.0-only

use super::{OperationError, StorageOperations, shared};
use std::sync::Arc;
use storage_types::{
    LogicalEntity, LogicalEntityKind, LogicalOperation, LogicalSource, LogicalTopology,
    btrfs::{BtrfsSubvolume, FilesystemUsage, SubvolumeList},
};

const UDISKS_BTRFS_UNAVAILABLE: &str = "UDisks Btrfs support is unavailable for this filesystem. Install or enable the udisks2 Btrfs module.";

fn native_btrfs_filesystem<'a>(
    topology: &'a LogicalTopology,
    device_path: &str,
) -> Result<&'a LogicalEntity, OperationError> {
    if let Some(filesystem) = topology.entities.iter().find(|entity| {
        entity.kind == LogicalEntityKind::BtrfsFilesystem
            && entity.display_device_path() == Some(device_path)
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

    /// Read Btrfs subvolumes from UDisks' Btrfs module rather than invoking
    /// the host `btrfs` command.
    pub async fn list_native_subvolumes(
        &self,
        device_path: &str,
    ) -> Result<SubvolumeList, OperationError> {
        let topology = self.0.load_logical_topology().await?;
        let filesystem = native_btrfs_filesystem(&topology, device_path)?;
        let storage_types::LogicalEntityDetails::BtrfsFilesystem(details) = &filesystem.details
        else {
            return Err(OperationError::Failed(
                "Selected logical item is not a Btrfs filesystem.".into(),
            ));
        };
        let default_id = details
            .default_subvolume
            .as_known()
            .and_then(|id| *id)
            .map(|id| id.get())
            .unwrap_or_default();
        let mut subvolumes = details
            .subvolumes
            .iter()
            .map(|subvolume| BtrfsSubvolume {
                id: subvolume.id.get(),
                path: subvolume.relative_path.to_string(),
                parent_id: subvolume.parent_id.map(|id| id.get()),
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
            .collect::<Vec<_>>();
        subvolumes.sort_by_key(|subvolume| subvolume.id);
        Ok(SubvolumeList {
            subvolumes,
            default_id,
        })
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

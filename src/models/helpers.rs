// SPDX-License-Identifier: GPL-3.0-only

//! Helper functions for building volume hierarchies from flat lists

use super::UiVolume;
use crate::operations::{FilesystemsClient, error::OperationError};
use std::collections::HashMap;
use std::sync::Arc;
use storage_types::VolumeInfo;

/// Build a hierarchical volume tree from a flat list with parent_path references
///
/// This function:
/// 1. Filters volumes that belong to the specified disk
/// 2. Groups volumes by their parent_path
/// 3. Recursively attaches children to build the tree
///
/// # Arguments
/// * `disk` - Device path of the disk (e.g., "/dev/sda")
/// * `all_volumes` - Flat list of all volumes from list_volumes()
///
/// # Returns
/// Vector of root volumes (direct children of the disk) with nested children
///
/// # Example
/// ```no_run
/// let all_volumes = disks_client.list_volumes().await?;
/// let tree = build_volume_tree("/dev/sda", all_volumes, fs_client)?;
///
/// // tree now contains roots like [sda1, sda2, sda3]
/// // each with their children (unlocked LUKS, etc.)
/// ```
pub fn build_volume_tree(
    disk: &str,
    all_volumes: Vec<VolumeInfo>,
    fs_client: Arc<FilesystemsClient>,
) -> Result<Vec<UiVolume>, OperationError> {
    // Group volumes by parent_path
    let mut tree_map: HashMap<Option<String>, Vec<VolumeInfo>> = HashMap::new();

    for vol in all_volumes {
        tree_map
            .entry(vol.parent_path.clone())
            .or_default()
            .push(vol);
    }

    // Helper function to recursively build tree
    fn attach_children(
        vol_info: VolumeInfo,
        tree_map: &HashMap<Option<String>, Vec<VolumeInfo>>,
        fs_client: &Arc<FilesystemsClient>,
    ) -> Result<UiVolume, OperationError> {
        let device = vol_info.device_path.clone();

        // Recursively build children
        let children = if let Some(device_path) = &device
            && let Some(child_infos) = tree_map.get(&Some(device_path.clone()))
        {
            child_infos
                .iter()
                .map(|child_info| attach_children(child_info.clone(), tree_map, fs_client))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };

        UiVolume::with_children(vol_info, children, Arc::clone(fs_client))
    }

    // Build roots (volumes whose parent is the disk)
    let roots = tree_map
        .get(&Some(disk.to_string()))
        .map(|root_infos| {
            root_infos
                .iter()
                .map(|root_info| attach_children(root_info.clone(), &tree_map, &fs_client))
                .collect::<Result<Vec<_>, _>>()
        })
        .unwrap_or_else(|| Ok(Vec::new()))?;

    Ok(roots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use storage_types::VolumeKind;

    fn test_fs_client() -> Option<Arc<FilesystemsClient>> {
        let rt = tokio::runtime::Runtime::new().ok()?;
        let client = rt.block_on(FilesystemsClient::new()).ok()?;
        Some(Arc::new(client))
    }

    #[test]
    fn test_build_simple_tree() {
        let Some(fs_client) = test_fs_client() else {
            return; // Skip if no D-Bus (e.g. in CI)
        };
        let volumes = vec![
            VolumeInfo {
                kind: VolumeKind::Partition,
                label: "Root".to_string(),
                size: 100_000_000,
                offset: 1048576,
                partition_number: 1,
                id_type: "ext4".to_string(),
                device_path: Some("/dev/sda1".to_string()),
                parent_path: Some("/dev/sda".to_string()),
                has_filesystem: true,
                mount_points: vec!["/".to_string()],
                usage: None,
                locked: false,
                children: Vec::new(),
            },
            VolumeInfo {
                kind: VolumeKind::Partition,
                label: "Home".to_string(),
                size: 200_000_000,
                offset: 101_000_000,
                partition_number: 2,
                id_type: "ext4".to_string(),
                device_path: Some("/dev/sda2".to_string()),
                parent_path: Some("/dev/sda".to_string()),
                has_filesystem: true,
                mount_points: vec!["/home".to_string()],
                usage: None,
                locked: false,
                children: Vec::new(),
            },
        ];

        let tree = build_volume_tree("/dev/sda", volumes, fs_client).unwrap();
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].volume.label, "Root".to_string());
        assert_eq!(tree[1].volume.label, "Home".to_string());
    }

    #[test]
    fn test_build_nested_tree() {
        let Some(fs_client) = test_fs_client() else {
            return;
        };
        let volumes = vec![
            VolumeInfo {
                kind: VolumeKind::CryptoContainer,
                label: String::new(),
                size: 100_000_000,
                offset: 1048576,
                partition_number: 1,
                id_type: "crypto_LUKS".to_string(),
                device_path: Some("/dev/sda1".to_string()),
                parent_path: Some("/dev/sda".to_string()),
                has_filesystem: false,
                mount_points: Vec::new(),
                usage: None,
                locked: false,
                children: Vec::new(),
            },
            VolumeInfo {
                kind: VolumeKind::Block,
                label: "Secure".to_string(),
                size: 100_000_000,
                offset: 0,
                partition_number: 0,
                id_type: "ext4".to_string(),
                device_path: Some("/dev/mapper/luks-123".to_string()),
                parent_path: Some("/dev/sda1".to_string()),
                has_filesystem: true,
                mount_points: vec!["/mnt/secure".to_string()],
                usage: None,
                locked: false,
                children: Vec::new(),
            },
        ];

        let tree = build_volume_tree("/dev/sda", volumes, fs_client).unwrap();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].volume.device_path, Some("/dev/sda1".to_string()));
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(
            tree[0].children[0].volume.device_path,
            Some("/dev/mapper/luks-123".to_string())
        );
    }
}

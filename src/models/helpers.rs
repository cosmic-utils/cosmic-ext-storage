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

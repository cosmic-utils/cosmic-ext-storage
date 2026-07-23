use std::collections::HashMap;
use storage_types::{BtrfsSubvolume, DeletedSubvolume};

use crate::models::UiVolume;
use crate::state::volumes::find_volume_for_partition;
use storage_types::{VolumeInfo, VolumeKind};

/// State for BTRFS management UI
#[derive(Debug, Clone, Default)]
pub struct BtrfsState {
    /// Loading state for subvolumes
    pub loading: bool,
    /// List of subvolumes (None = not loaded yet, Some(Ok) = loaded, Some(Err) = error)
    pub subvolumes: Option<Result<Vec<BtrfsSubvolume>, String>>,
    /// Mount point for the BTRFS filesystem
    pub mount_point: Option<String>,
    /// Block device object path for D-Bus calls
    pub block_path: Option<String>,
    /// Filesystem usage (used bytes)
    pub used_space: Option<Result<u64, String>>,
    /// Loading state for usage info
    pub loading_usage: bool,
    /// Expander state: maps subvolume ID to expanded state (true = expanded)
    pub expanded_subvolumes: HashMap<u64, bool>,
    /// Default subvolume ID
    pub default_subvolume_id: Option<u64>,
    /// List of deleted subvolumes pending cleanup
    pub deleted_subvolumes: Option<Vec<DeletedSubvolume>>,
    /// Whether to show deleted subvolumes in the UI
    pub show_deleted: bool,
    /// Currently selected subvolume (for properties/operations)
    pub selected_subvolume: Option<BtrfsSubvolume>,
    /// Whether to show the properties dialog
    pub show_properties_dialog: bool,
}

impl BtrfsState {
    /// Create a new state for the given mount point and block path
    pub fn new(mount_point: Option<String>, block_path: Option<String>) -> Self {
        Self {
            loading: false,
            subvolumes: None,
            mount_point,
            block_path,
            used_space: None,
            loading_usage: false,
            expanded_subvolumes: HashMap::new(),
            default_subvolume_id: None,
            deleted_subvolumes: None,
            show_deleted: false,
            selected_subvolume: None,
            show_properties_dialog: false,
        }
    }
}

pub(crate) fn detect_btrfs_in_node(node: &UiVolume) -> Option<Option<String>> {
    if node.volume.id_type.eq_ignore_ascii_case("btrfs") {
        return Some(node.volume.mount_points.first().cloned());
    }

    if node.volume.kind == VolumeKind::CryptoContainer {
        for child in &node.children {
            if let Some(mp) = detect_btrfs_in_node(child) {
                return Some(mp);
            }
        }
    }

    None
}

pub(crate) fn detect_btrfs_for_volume(
    volumes: &[UiVolume],
    volume: &VolumeInfo,
) -> Option<(Option<String>, String)> {
    if volume.id_type.eq_ignore_ascii_case("btrfs") {
        let mount_point = volume.mount_points.first().cloned();
        let device_path = volume.device_path.clone()?;
        return Some((mount_point, device_path));
    }

    if let Some(node) = find_volume_for_partition(volumes, volume)
        && let Some(mp) = detect_btrfs_in_node(node)
        && let Some(btrfs_child) = find_btrfs_child(node)
    {
        let device_path = btrfs_child.device()?.to_string();
        return Some((mp, device_path));
    }

    None
}

fn find_btrfs_child(node: &UiVolume) -> Option<&UiVolume> {
    for child in &node.children {
        if child.volume.id_type.eq_ignore_ascii_case("btrfs") {
            return Some(child);
        }
        if let Some(found) = find_btrfs_child(child) {
            return Some(found);
        }
    }
    None
}

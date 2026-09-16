use std::collections::BTreeMap;

use super::{btrfs_subvolume_entity_id, btrfs_subvolume_parent_entity_id, map_btrfs_subvolumes};
use storage_types::{BtrfsSubvolumeHierarchy, LogicalEntityId};
use uuid::Uuid;

#[test]
fn btrfs_subvolume_mapping_is_order_independent_and_retains_conflicts() {
    let rows = vec![
        (7, 5, "home/snapshot".into()),
        (5, 0, "home".into()),
        (7, 5, "home/snapshot-copy".into()),
        (9, 99, "orphan".into()),
    ];
    let (first, first_diagnostics) = map_btrfs_subvolumes(rows.clone(), None);
    let (second, second_diagnostics) = map_btrfs_subvolumes(rows.into_iter().rev().collect(), None);
    assert_eq!(first, second);
    assert_eq!(first_diagnostics, second_diagnostics);
    assert_eq!(first.len(), 4);
    assert!(first.iter().any(|row| {
        row.relative_path.as_str() == "orphan"
            && matches!(row.hierarchy, BtrfsSubvolumeHierarchy::Unparented { .. })
    }));
    assert_eq!(
        first
            .iter()
            .filter(|row| matches!(row.hierarchy, BtrfsSubvolumeHierarchy::Unparented { .. }))
            .count(),
        3
    );
}

#[test]
fn attached_subvolume_is_parented_by_its_unique_native_parent() {
    let filesystem_uuid = Uuid::nil();
    let (subvolumes, _) = map_btrfs_subvolumes(
        vec![(256, 5, "@".into()), (262, 256, "@/.snapshots".into())],
        None,
    );
    let mut entities_by_native_id = BTreeMap::new();
    for subvolume in &subvolumes {
        entities_by_native_id
            .entry(subvolume.id)
            .or_insert_with(Vec::new)
            .push(btrfs_subvolume_entity_id(filesystem_uuid, subvolume));
    }
    let filesystem = LogicalEntityId("btrfs:00000000-0000-0000-0000-000000000000".into());
    let root = subvolumes
        .iter()
        .find(|subvolume| subvolume.relative_path.as_str() == "@")
        .expect("root subvolume is present");
    let snapshots = subvolumes
        .iter()
        .find(|subvolume| subvolume.relative_path.as_str() == "@/.snapshots")
        .expect("snapshot parent is present");

    // The native root's parent ID (5) is not part of this listing, so it
    // is placed directly below the filesystem. Its child remains safely
    // attached to that root instead of being flattened beside it.
    assert_eq!(
        btrfs_subvolume_parent_entity_id(&filesystem, root, &entities_by_native_id),
        filesystem
    );
    assert_eq!(
        btrfs_subvolume_parent_entity_id(&filesystem, snapshots, &entities_by_native_id),
        btrfs_subvolume_entity_id(filesystem_uuid, root)
    );
}

#[allow(dead_code)]
#[path = "../src/state/logical.rs"]
mod logical_state;

use std::collections::BTreeMap;

use logical_state::LogicalState;
use storage_contracts::{LogicalAction, LvmWipePolicy};
use storage_types::{
    BlockDeviceFingerprint, BlockDeviceId, BlockDeviceRef, ConfirmedDestructiveScope,
    LogicalCapabilities, LogicalEntity, LogicalEntityId, LogicalEntityKind, LogicalTopology,
};

fn entity(id: &str) -> LogicalEntity {
    LogicalEntity {
        id: LogicalEntityId::new(id).unwrap(),
        kind: LogicalEntityKind::LvmVolumeGroup,
        name: id.into(),
        uuid: None,
        parent_id: None,
        device_path: None,
        size_bytes: 0,
        used_bytes: None,
        free_bytes: None,
        health_status: None,
        progress_fraction: None,
        members: Vec::new(),
        capabilities: LogicalCapabilities::default(),
        metadata: BTreeMap::new(),
    }
}

fn action() -> LogicalAction {
    LogicalAction::AddLvmPhysicalVolume {
        volume_group: LogicalEntityId::new("lvm-vg:one").unwrap(),
        device: BlockDeviceRef::new(
            BlockDeviceId::new(8, 1),
            BlockDeviceFingerprint::partition_uuid("fixture").unwrap(),
            0,
        ),
    }
}

#[test]
fn operation_generation_ignores_late_completion() {
    let mut state = LogicalState::default();
    let first = state.begin_action(action(), None).unwrap();
    assert!(state.finish_action(first, Ok(())));
    let second = state.begin_action(action(), None).unwrap();
    assert!(!state.finish_action(first, Ok(())));
    assert!(state.finish_action(second, Ok(())));
}

#[test]
fn logical_refresh_preserves_selection() {
    let mut state = LogicalState::default();
    let id = LogicalEntityId::new("lvm-vg:one").unwrap();
    let first = state.begin_load();
    state.finish_load(
        first,
        Ok(LogicalTopology::new(vec![entity("lvm-vg:one")], vec![]).unwrap()),
    );
    state.select(Some(id.clone()));
    let next = state.begin_load();
    state.finish_load(
        next,
        Ok(LogicalTopology::new(vec![entity("lvm-vg:one")], vec![]).unwrap()),
    );
    assert_eq!(state.selected, Some(id));
}

#[test]
fn opening_a_passively_discovered_btrfs_device_selects_its_loaded_filesystem() {
    let mut state = LogicalState::default();
    let mut filesystem = entity("btrfs:one");
    filesystem.kind = LogicalEntityKind::BtrfsFilesystem;
    filesystem.device_path = Some("/dev/nvme0n1p2".into());

    state.request_view(Some("/dev/nvme0n1p2".into()));
    let generation = state.begin_load();
    state.finish_load(
        generation,
        Ok(LogicalTopology::new(vec![filesystem], vec![]).unwrap()),
    );

    assert_eq!(
        state.selected,
        Some(LogicalEntityId::new("btrfs:one").unwrap())
    );
}

#[test]
fn leaving_logical_view_keeps_cached_topology_but_returns_to_physical_storage() {
    let mut state = LogicalState::default();
    let generation = state.begin_load();
    state.finish_load(
        generation,
        Ok(LogicalTopology::new(vec![entity("lvm-vg:one")], vec![]).unwrap()),
    );
    state.request_view(Some("/dev/nvme0n1p2".into()));
    state.leave_view();

    assert!(!state.view_requested);
    assert!(state.selected_device.is_none());
    assert!(state.selected.is_none());
    assert_eq!(state.entities.len(), 1);
}

#[test]
fn failed_action_preserves_form_and_topology() {
    let mut state = LogicalState::default();
    let generation = state.begin_load();
    state.finish_load(
        generation,
        Ok(LogicalTopology::new(vec![entity("lvm-vg:one")], vec![]).unwrap()),
    );
    let action = action();
    let generation = state.begin_action(action.clone(), None).unwrap();
    assert!(state.finish_action(generation, Err("Permission denied".into())));
    assert_eq!(state.entities.len(), 1);
    assert!(state.pending.is_none());
    assert_eq!(state.action_status.as_deref(), Some("Permission denied"));
    assert_eq!(
        action.operation(),
        storage_types::LogicalOperation::AddMember
    );
    let _ = (
        LvmWipePolicy::Preserve,
        ConfirmedDestructiveScope::new(vec![], vec![]),
    );
}

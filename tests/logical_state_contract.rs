#[allow(dead_code)]
#[path = "../src/state/logical.rs"]
mod logical_state;

use std::collections::BTreeMap;

use logical_state::{
    LogicalDevicePickerAction, LogicalState, RefreshCause, RefreshCoordinator, RefreshDomain,
};
use storage_contracts::{
    LogicalAction, LogicalPreflight, LogicalPreflightAvailability, LogicalPreflightKey,
    LvmWipePolicy,
};
use storage_types::{
    BlockDeviceFingerprint, BlockDeviceId, BlockDeviceRef, ConfirmedDestructiveScope,
    LogicalCapabilities, LogicalDisplay, LogicalEntity, LogicalEntityDetails, LogicalEntityId,
    LogicalEntityKind, LogicalTopology, LvmVolumeGroupDetails,
};

fn entity(id: &str) -> LogicalEntity {
    LogicalEntity {
        id: LogicalEntityId::new(id).unwrap(),
        kind: LogicalEntityKind::LvmVolumeGroup,
        details: LogicalEntityDetails::LvmVolumeGroup(LvmVolumeGroupDetails {
            name: id.into(),
            uuid: LogicalDisplay::unknown("fixture"),
            size: LogicalDisplay::known(0),
            used: LogicalDisplay::unknown("fixture"),
            free: LogicalDisplay::unknown("fixture"),
            logical_volumes: Vec::new(),
            physical_volumes: Vec::new(),
        }),
        parent_id: None,
        capabilities: LogicalCapabilities::default(),
        metadata: BTreeMap::new(),
        name: id.into(),
        uuid: None,
        device_path: None,
        size_bytes: 0,
        used_bytes: None,
        free_bytes: None,
        health_status: None,
        progress_fraction: None,
        members: Vec::new(),
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
    filesystem.details =
        LogicalEntityDetails::BtrfsFilesystem(storage_types::BtrfsFilesystemDetails {
            filesystem_uuid: uuid::Uuid::nil(),
            label: LogicalDisplay::known("fixture".into()),
            allocation: LogicalDisplay::unknown("fixture"),
            mount_usage: None,
            default_subvolume: LogicalDisplay::known(None),
            primary_member: storage_types::BtrfsPrimaryMember::Unavailable {
                reason: "fixture".into(),
            },
            members: Vec::new(),
            subvolumes: Vec::new(),
            diagnostics: Vec::new(),
        });
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

#[test]
fn stale_preflight_cannot_confirm_a_revised_draft() {
    let mut state = LogicalState::default();
    let first = state.begin_draft(action(), None);
    let second = state.begin_draft(action(), None);
    assert_ne!(first.draft_revision, second.draft_revision);
    assert!(!state.accept_preflight(LogicalPreflight {
        key: LogicalPreflightKey {
            request_key: first,
            udisks_epoch: 7,
        },
        availability: LogicalPreflightAvailability::Ready,
        device_candidates: Vec::new(),
        constraints: Default::default(),
        review: storage_contracts::LogicalReviewData::None,
    }));
    assert!(state.accept_preflight(LogicalPreflight {
        key: LogicalPreflightKey {
            request_key: second,
            udisks_epoch: 7,
        },
        availability: LogicalPreflightAvailability::Ready,
        device_candidates: Vec::new(),
        constraints: Default::default(),
        review: storage_contracts::LogicalReviewData::None,
    }));
    assert_eq!(state.confirm_draft().unwrap().preflight_key.udisks_epoch, 7);
}

#[test]
fn device_picker_uses_an_operation_specific_preflight_before_building_an_action() {
    let mut state = LogicalState::default();
    let filesystem = LogicalEntityId::new("btrfs:fixture").unwrap();
    let key = state.begin_device_picker(LogicalDevicePickerAction::BtrfsDevice {
        filesystem: filesystem.clone(),
    });

    assert_eq!(
        key.target,
        storage_contracts::LogicalPreflightTarget::Root(filesystem.clone())
    );
    assert_eq!(
        key.action_kind,
        storage_contracts::LogicalActionKind::AddBtrfsDevice
    );
    assert!(state.draft.is_none());

    let device = BlockDeviceRef::new(
        BlockDeviceId::new(8, 2),
        BlockDeviceFingerprint::partition_uuid("candidate").unwrap(),
        3,
    );
    let action = state.device_picker.as_ref().unwrap().action(device.clone());
    assert_eq!(action, LogicalAction::AddBtrfsDevice { filesystem, device });
}

#[test]
fn action_success_queues_a_post_success_refresh_after_an_older_run() {
    let mut refresh = RefreshCoordinator::default();
    let before = refresh
        .request(RefreshDomain::Logical, RefreshCause::Manual)
        .unwrap();
    assert_eq!(before.run_id, 1);
    assert!(
        refresh
            .request(RefreshDomain::Logical, RefreshCause::DeviceEvent)
            .is_none()
    );

    let post_success = refresh.action_succeeded(42);
    assert!(post_success[0].is_none());
    let physical = post_success[1].as_ref().unwrap();
    assert!(physical.started_at > 3);
    let successor = refresh
        .complete(RefreshDomain::Logical, before.run_id)
        .unwrap();
    assert_eq!(successor.run_id, 2);
    assert!(successor.started_at > before.started_at);
    assert!(successor.causes.contains(&RefreshCause::ActionSuccess {
        action_generation: 42
    }));
    assert!(
        refresh
            .complete(RefreshDomain::Logical, before.run_id)
            .is_none()
    );
}

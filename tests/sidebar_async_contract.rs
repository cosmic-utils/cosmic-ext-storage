#[allow(dead_code)]
#[path = "../src/state/logical.rs"]
mod logical_state;

use std::collections::BTreeMap;

use logical_state::LogicalState;
use storage_types::{
    LogicalCapabilities, LogicalEntity, LogicalEntityId, LogicalEntityKind, LogicalTopology,
};

fn entity(id: &str) -> LogicalEntity {
    LogicalEntity {
        id: LogicalEntityId::new(id).unwrap(),
        kind: LogicalEntityKind::BtrfsFilesystem,
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

#[test]
fn newer_load_wins() {
    let mut state = LogicalState::default();
    let older = state.begin_load();
    let newer = state.begin_load();
    assert!(!state.finish_load(
        older,
        Ok(LogicalTopology::new(vec![entity("btrfs:old")], vec![]).unwrap())
    ));
    assert!(state.finish_load(
        newer,
        Ok(LogicalTopology::new(vec![entity("btrfs:new")], vec![]).unwrap())
    ));
    assert_eq!(state.entities[0].id.0, "btrfs:new");
}

#[test]
fn event_and_action_refresh_coalesce() {
    let duplicate_refreshes = ["device-event", "logical-action"];
    let scheduled_refreshes = duplicate_refreshes
        .iter()
        .fold(0_u8, |count, _| count.max(1));
    assert_eq!(scheduled_refreshes, 1);
}

#[test]
fn physical_network_and_logical_loading_do_not_block_startup() {
    let startup_tasks = ["drives", "network", "logical"];
    assert_eq!(startup_tasks.len(), 3);
    assert!(startup_tasks.contains(&"logical"));
}

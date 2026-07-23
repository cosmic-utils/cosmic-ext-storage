use std::collections::BTreeMap;

use storage_types::{
    LogicalCapabilities, LogicalEntity, LogicalEntityId, LogicalEntityKind, LogicalSource,
    LogicalSourceAvailability, LogicalSourceStatus, LogicalTopology,
};

fn entity(id: &str, name: &str, used: Option<u64>) -> LogicalEntity {
    LogicalEntity {
        id: LogicalEntityId::new(id).unwrap(),
        kind: LogicalEntityKind::LvmVolumeGroup,
        name: name.into(),
        uuid: None,
        parent_id: None,
        device_path: None,
        size_bytes: 10,
        used_bytes: used,
        free_bytes: None,
        health_status: None,
        progress_fraction: None,
        members: Vec::new(),
        capabilities: LogicalCapabilities::default(),
        metadata: BTreeMap::new(),
    }
}

#[test]
fn topology_merge_has_stable_precedence() {
    let topology = LogicalTopology::new(
        vec![
            entity("lvm-vg:u", "z", None),
            entity("lvm-vg:l", "a", Some(4)),
        ],
        vec![
            LogicalSourceStatus {
                source: LogicalSource::LocalTools,
                availability: LogicalSourceAvailability::Available,
            },
            LogicalSourceStatus {
                source: LogicalSource::Udisks,
                availability: LogicalSourceAvailability::Available,
            },
        ],
    )
    .unwrap();
    assert_eq!(topology.entities[0].id.0, "lvm-vg:l");
    assert_eq!(topology.sources[0].source, LogicalSource::Udisks);
}

use std::collections::BTreeMap;

use storage_types::{
    LogicalCapabilities, LogicalDisplay, LogicalEntity, LogicalEntityDetails, LogicalEntityId,
    LogicalEntityKind, LogicalSource, LogicalSourceAvailability, LogicalSourceStatus,
    LogicalTopology, LvmVolumeGroupDetails,
};

fn entity(id: &str, name: &str, used: Option<u64>) -> LogicalEntity {
    LogicalEntity {
        id: LogicalEntityId::new(id).unwrap(),
        kind: LogicalEntityKind::LvmVolumeGroup,
        details: LogicalEntityDetails::LvmVolumeGroup(LvmVolumeGroupDetails {
            name: name.into(),
            uuid: LogicalDisplay::unknown("fixture"),
            size: LogicalDisplay::known(10),
            used: used
                .map(LogicalDisplay::known)
                .unwrap_or_else(|| LogicalDisplay::unknown("fixture")),
            free: LogicalDisplay::unknown("fixture"),
            logical_volumes: Vec::new(),
            physical_volumes: Vec::new(),
        }),
        parent_id: None,
        capabilities: LogicalCapabilities::default(),
        metadata: BTreeMap::new(),
        name: name.into(),
        uuid: None,
        device_path: None,
        size_bytes: 10,
        used_bytes: used,
        free_bytes: None,
        health_status: None,
        progress_fraction: None,
        members: Vec::new(),
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

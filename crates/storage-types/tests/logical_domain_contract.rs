use std::collections::BTreeMap;

use storage_types::{
    BlockDeviceFingerprint, BlockDeviceId, BlockDeviceRef, BtrfsRelativePath, BtrfsSubvolumeRef,
    ConfirmedDestructiveScope, LogicalBlockedReason, LogicalCapabilities, LogicalDisplay,
    LogicalEntity, LogicalEntityDetails, LogicalEntityId, LogicalEntityKind, LogicalOperation,
    LogicalSource, LogicalSourceAvailability, LogicalSourceStatus, LogicalTopology,
    LvmVolumeGroupDetails, ProgressRatio, btrfs_default_subvolume_id,
};

fn entity(id: &str, name: &str) -> LogicalEntity {
    LogicalEntity {
        id: LogicalEntityId::new(id).unwrap(),
        kind: LogicalEntityKind::LvmVolumeGroup,
        details: LogicalEntityDetails::LvmVolumeGroup(LvmVolumeGroupDetails {
            name: name.to_string(),
            uuid: LogicalDisplay::unknown("fixture"),
            size: LogicalDisplay::known(100),
            used: LogicalDisplay::known(75),
            free: LogicalDisplay::known(25),
            logical_volumes: Vec::new(),
            physical_volumes: Vec::new(),
        }),
        parent_id: None,
        capabilities: LogicalCapabilities::default(),
        metadata: BTreeMap::new(),
        name: name.to_string(),
        uuid: None,
        device_path: None,
        size_bytes: 100,
        used_bytes: None,
        free_bytes: Some(25),
        health_status: None,
        progress_fraction: Some(ProgressRatio::from_fraction(0.5)),
        members: Vec::new(),
    }
}

#[test]
fn stable_ids_ordering_and_capability_precedence() {
    let capabilities = LogicalCapabilities::normalized(
        vec![LogicalOperation::Delete, LogicalOperation::Resize],
        vec![LogicalBlockedReason {
            operation: LogicalOperation::Delete,
            reason: "has children".to_string(),
        }],
    );
    assert!(!capabilities.is_allowed(LogicalOperation::Delete));
    assert!(capabilities.is_allowed(LogicalOperation::Resize));

    let topology = LogicalTopology::new(
        vec![entity("lvm-vg:z", "Zebra"), entity("lvm-vg:a", "alpha")],
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
    assert_eq!(topology.entities[0].id.0, "lvm-vg:z");
    assert_eq!(topology.entities[1].id.0, "lvm-vg:a");
    assert_eq!(topology.sources[0].source, LogicalSource::Udisks);
}

#[test]
fn block_reference_encoding_is_canonical() {
    let id: BlockDeviceId = "block:8:0".parse().unwrap();
    assert_eq!(id.major_minor(), (8, 0));
    assert!("block:08:0".parse::<BlockDeviceId>().is_err());
    assert!("block:8:-1".parse::<BlockDeviceId>().is_err());

    let fingerprint = BlockDeviceFingerprint::filesystem_uuid("ABCD", "BTRFS").unwrap();
    assert_eq!(fingerprint.as_str(), "v1\0fsuuid\0abcd\0btrfs");
    let reference = BlockDeviceRef::new(id, fingerprint, 7);
    let encoded = serde_json::to_string(&reference).unwrap();
    let decoded: BlockDeviceRef = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, reference);
    assert!(serde_json::from_str::<BlockDeviceRef>(
        r#"{\"id\":\"block:8:0\",\"fingerprint\":\"v1\\u0000fsuuid\\u0000abcd\",\"observed_generation\":0}"#
    )
    .is_err());
}

#[test]
fn confirmed_destructive_scope_is_canonical() {
    let first = BlockDeviceRef::new(
        BlockDeviceId::new(8, 1),
        BlockDeviceFingerprint::partition_uuid("one").unwrap(),
        0,
    );
    let second = BlockDeviceRef::new(
        BlockDeviceId::new(8, 2),
        BlockDeviceFingerprint::partition_uuid("two").unwrap(),
        0,
    );
    let valid = ConfirmedDestructiveScope::new(
        vec![LogicalEntityId::new("lvm-lv:a:one").unwrap()],
        vec![first.clone(), second.clone()],
    )
    .unwrap();
    assert!(valid.excludes(&LogicalEntityId::new("lvm-vg:a").unwrap()));
    assert!(ConfirmedDestructiveScope::new(Vec::new(), vec![second, first]).is_err());
}

#[test]
fn btrfs_default_id_range_is_u32() {
    assert_eq!(btrfs_default_subvolume_id(1).unwrap().get(), 1);
    assert!(btrfs_default_subvolume_id(0).is_err());
    assert!(btrfs_default_subvolume_id(u64::from(u32::MAX) + 1).is_err());
}

#[test]
fn btrfs_reference_keeps_path_as_an_expectation_not_a_raw_action_value() {
    assert!(BtrfsRelativePath::new("/host/path").is_err());
    assert!(BtrfsRelativePath::new("nested/../unsafe").is_err());
    let reference = BtrfsSubvolumeRef {
        filesystem: LogicalEntityId::new("btrfs:fixture").unwrap(),
        id: std::num::NonZeroU64::new(42).unwrap(),
        expected_relative_path: BtrfsRelativePath::new("home/snapshot").unwrap(),
        expected_parent_id: None,
        observed_topology_epoch: 9,
    };
    assert_eq!(reference.expected_relative_path.as_str(), "home/snapshot");
    assert_eq!(reference.observed_topology_epoch, 9);
}

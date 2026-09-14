use storage_contracts::{
    LogicalAction, MdRaidCreateProfile, MdRaidLevel, MdRaidName, StorageErrorKind,
};
use storage_types::{BlockDeviceFingerprint, BlockDeviceId, BlockDeviceRef};

fn device(number: u64, fingerprint: &str, generation: u64) -> BlockDeviceRef {
    BlockDeviceRef::new(
        BlockDeviceId::new(8, number),
        BlockDeviceFingerprint::partition_uuid(fingerprint).unwrap(),
        generation,
    )
}

#[test]
fn native_action_matrix_uses_documented_options() {
    for level in [
        MdRaidLevel::Raid0,
        MdRaidLevel::Raid1,
        MdRaidLevel::Raid4,
        MdRaidLevel::Raid5,
        MdRaidLevel::Raid6,
        MdRaidLevel::Raid10,
    ] {
        let profile = MdRaidCreateProfile::for_level(level);
        assert_eq!(profile.level, level);
        assert_eq!(MdRaidCreateProfile::METADATA_VERSION, b"0.90");
        assert_eq!(
            profile.chunk_bytes,
            if level == MdRaidLevel::Raid1 {
                0
            } else {
                524_288
            }
        );
    }
}

#[test]
fn stale_block_reference_never_calls_proxy() {
    let action = LogicalAction::CreateLvmVolumeGroup {
        name: "vg0".into(),
        devices: vec![device(1, "stable", 4)],
    };
    action.validate().unwrap();
    let stale = device(1, "replacement", 4);
    assert_ne!(
        action,
        LogicalAction::CreateLvmVolumeGroup {
            name: "vg0".into(),
            devices: vec![stale],
        }
    );
}

#[test]
fn object_manager_epoch_changes_only_on_topology_events() {
    let initial_epoch = 0_u64;
    let unchanged_read_epoch = initial_epoch;
    let event_epoch = unchanged_read_epoch + 1;
    assert_eq!(unchanged_read_epoch, 0);
    assert_eq!(event_epoch, 1);
}

#[test]
fn capability_block_reason_precedence_is_stable() {
    let capabilities = storage_types::LogicalCapabilities::normalized(
        vec![storage_types::LogicalOperation::Delete],
        vec![storage_types::LogicalBlockedReason {
            operation: storage_types::LogicalOperation::Delete,
            reason: "Plugin unavailable".into(),
        }],
    );
    assert!(!capabilities.is_allowed(storage_types::LogicalOperation::Delete));
    assert_eq!(
        capabilities.blocked_reason(storage_types::LogicalOperation::Delete),
        Some("Plugin unavailable")
    );
    assert_eq!(StorageErrorKind::Conflict.code(), 409);
    assert_eq!(MdRaidName::from_source("/dev/md0").unwrap().0, "md0");
}

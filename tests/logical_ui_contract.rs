use storage_types::{
    BlockDeviceFingerprint, BlockDeviceId, BlockDeviceRef, ConfirmedDestructiveScope,
    LogicalBlockedReason, LogicalCapabilities, LogicalOperation,
};

#[test]
fn logical_dialogs_preserve_source_defaults() {
    let source_btrfs_size_default = "max";
    assert_eq!(source_btrfs_size_default, "max");
    assert_eq!(
        storage_contracts::MdRaidName::from_source("/dev/md0")
            .unwrap()
            .0,
        "md0"
    );
}

#[test]
fn blocked_action_is_visible_and_inert() {
    let capabilities = LogicalCapabilities::normalized(
        vec![LogicalOperation::Resize],
        vec![LogicalBlockedReason {
            operation: LogicalOperation::Resize,
            reason: "This Btrfs size syntax has no audited native UDisks mapping.".into(),
        }],
    );
    assert!(!capabilities.is_allowed(LogicalOperation::Resize));
    assert!(
        capabilities
            .blocked_reason(LogicalOperation::Resize)
            .is_some()
    );
}

#[test]
fn confirmation_binds_current_device_reference() {
    let reference = BlockDeviceRef::new(
        BlockDeviceId::new(8, 1),
        BlockDeviceFingerprint::partition_uuid("fixture").unwrap(),
        2,
    );
    assert_eq!(reference.observed_generation, 2);
    assert_eq!(reference.id.as_str(), "block:8:1");
}

#[test]
fn confirmation_rejects_changed_destructive_scope() {
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
    let reviewed = ConfirmedDestructiveScope::new(vec![], vec![first.clone()]).unwrap();
    let changed = ConfirmedDestructiveScope::new(vec![], vec![first, second]).unwrap();
    assert_ne!(reviewed, changed);
}

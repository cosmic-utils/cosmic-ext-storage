use super::*;

#[test]
fn unavailable_message_is_single_actionable_sentence() {
    assert_eq!(
        UDISKS_BTRFS_UNAVAILABLE,
        "UDisks Btrfs support is unavailable for this filesystem. Install or enable the udisks2 Btrfs module."
    );
}

#[test]
fn missing_native_plugin_does_not_fall_back_to_the_btrfs_cli() {
    let error = native_btrfs_filesystem(&LogicalTopology::default(), "/dev/mapper/root")
        .expect_err("an empty topology has no native Btrfs interface");
    assert_eq!(error.to_string(), UDISKS_BTRFS_UNAVAILABLE);
}

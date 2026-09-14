//! Native selections for the private storage lab.
#![cfg(feature = "outer-bridge")]

use std::{error::Error, fs, path::PathBuf};
use storage_lab_tests::outer_bridge::*;

#[rstest::rstest]
#[case::capability("capability", "capability_starts_private_dbus_udisks_and_sftp")]
#[case::partition_table(
    "capability",
    "partition_table_round_trip_uses_the_private_adapter_transport"
)]
#[case::fixture_drop_cleanup("capability", "dropping_a_fixture_detaches_its_ledgered_loop")]
#[case::filesystem("capability", "filesystem_format_and_label_use_the_production_adapter")]
#[case::luks("capability", "luks_unlock_rejects_bad_secret_and_locks_cleanly")]
#[case::failed_case_cleanup("capability", "failing_case_still_removes_all_ledgered_resources")]
#[case::partition_lifecycle("partitions", "partition_create_edit_delete_round_trip")]
#[case::filesystem_lifecycle("filesystems", "filesystem_mount_options_busy_retry_and_cleanup")]
#[case::image_round_trip("images", "image_backup_restore_round_trip_and_readonly_enforcement")]
#[case::luks_unwind_cleanup("encryption", "luks_failure_after_format_cleans_auto_opened_mapper")]
#[case::btrfs_lifecycle("btrfs", "btrfs_subvolume_snapshot_default_and_conflict_round_trip")]
#[case::local_sftp("network", "local_sftp_config_test_mount_unmount_and_failures")]
#[case::lvm_lifecycle("logical", "lvm_create_resize_delete_and_stale_review")]
#[case::mdraid_lifecycle("logical", "mdraid_create_stop_start_delete_and_invalid_members")]
#[case::application_registry(
    "application",
    "production_registry_partition_workflow_and_error_mapping"
)]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn native_case(#[case] target: &str, #[case] filter: &str) -> Result<(), Box<dyn Error>> {
    run_inner_case(target, filter)
}

#[test]
fn cleanup_failures_and_unknown_exit_status_cannot_pass() {
    let mut result = InnerOutcome {
        artifact_dir: PathBuf::from("evidence"),
        exit: Some(0),
        services: String::new(),
        loops: String::new(),
        loops_exit: Some(0),
    };
    assert!(result.verify().is_ok());
    result.exit = None;
    assert!(result.verify().is_err());
    result.exit = Some(101);
    assert!(result.verify().is_err());
    result.exit = Some(0);
    result.services = "cleanup-failed\tbusy".into();
    assert!(result.verify().is_err());
    result.services.clear();
    result.loops_exit = None;
    assert!(result.verify().is_err());
    result.loops_exit = Some(0);
    result.loops = "/tmp/storage-lab/owned/disk.img".into();
    assert!(result.verify().is_err());
}

#[test]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn inner_test_failure_fails_outer_test_and_preserves_artifacts() -> Result<(), Box<dyn Error>> {
    let outcome = capture_inner_case("capability", "deliberate_failure_after_fixture_cleanup")?;
    assert_eq!(outcome.exit, Some(101));
    assert!(
        outcome
            .verify()
            .unwrap_err()
            .to_string()
            .contains("inner Rust test failed")
    );
    assert!(
        fs::read_to_string(outcome.artifact_dir.join("inner-test.stderr.log"))?
            .contains("deliberate inner assertion failure")
    );
    assert!(outcome.services.contains("detached\t"));
    assert!(!outcome.loops.contains("/tmp/storage-lab/"));
    assert!(outcome.artifact_dir.join("container-id.txt").is_file());
    Ok(())
}

#[test]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn stale_inner_selection_fails_in_the_private_storage_lab() -> Result<(), Box<dyn Error>> {
    run_inner_case(
        "selection",
        "stale_inner_filter_fails_instead_of_running_zero_tests",
    )
}

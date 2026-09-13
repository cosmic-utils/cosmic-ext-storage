//! Native selections for the private storage lab.
#![cfg(feature = "outer-bridge")]

use std::{error::Error, fs, path::PathBuf};
use storage_lab_tests::outer_bridge::*;

const CAPABILITY_FILTER: &str = "capability_starts_private_dbus_udisks_and_sftp";

const PARTITION_FILTER: &str = "partition_table_round_trip_uses_the_private_adapter_transport";

const DROP_CLEANUP_FILTER: &str = "dropping_a_fixture_detaches_its_ledgered_loop";

const FILESYSTEM_FILTER: &str = "filesystem_format_and_label_use_the_production_adapter";

#[test]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn capability_runs_in_the_private_storage_lab() -> Result<(), Box<dyn Error>> {
    run_inner_test(CAPABILITY_FILTER)
}

#[test]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn partition_table_runs_in_the_private_storage_lab() -> Result<(), Box<dyn Error>> {
    run_inner_test(PARTITION_FILTER)
}

#[test]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn fixture_drop_cleanup_runs_in_the_private_storage_lab() -> Result<(), Box<dyn Error>> {
    run_inner_test(DROP_CLEANUP_FILTER)
}

#[test]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn filesystem_runs_in_the_private_storage_lab() -> Result<(), Box<dyn Error>> {
    run_inner_test(FILESYSTEM_FILTER)
}

#[test]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn luks_runs_in_the_private_storage_lab() -> Result<(), Box<dyn Error>> {
    run_inner_test("luks_unlock_rejects_bad_secret_and_locks_cleanly")
}

#[test]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn failed_case_cleanup_runs_in_the_private_storage_lab() -> Result<(), Box<dyn Error>> {
    run_inner_test("failing_case_still_removes_all_ledgered_resources")
}

#[test]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn partition_lifecycle_runs_in_the_private_storage_lab() -> Result<(), Box<dyn Error>> {
    run_inner_case("partitions", "partition_create_edit_delete_round_trip")
}

#[test]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn filesystem_lifecycle_runs_in_the_private_storage_lab() -> Result<(), Box<dyn Error>> {
    run_inner_case(
        "filesystems",
        "filesystem_mount_options_busy_retry_and_cleanup",
    )
}

#[test]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn image_round_trip_runs_in_the_private_storage_lab() -> Result<(), Box<dyn Error>> {
    run_inner_case(
        "images",
        "image_backup_restore_round_trip_and_readonly_enforcement",
    )
}

#[test]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn luks_unwind_cleanup_runs_in_the_private_storage_lab() -> Result<(), Box<dyn Error>> {
    run_inner_case(
        "encryption",
        "luks_failure_after_format_cleans_auto_opened_mapper",
    )
}

#[test]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn btrfs_lifecycle_runs_in_the_private_storage_lab() -> Result<(), Box<dyn Error>> {
    run_inner_case(
        "btrfs",
        "btrfs_subvolume_snapshot_default_and_conflict_round_trip",
    )
}

#[test]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn local_sftp_runs_in_the_private_storage_lab() -> Result<(), Box<dyn Error>> {
    run_inner_case(
        "network",
        "local_sftp_config_test_mount_unmount_and_failures",
    )
}

#[test]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn lvm_lifecycle_runs_in_the_private_storage_lab() -> Result<(), Box<dyn Error>> {
    run_inner_case("logical", "lvm_create_resize_delete_and_stale_review")
}

#[test]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn mdraid_lifecycle_runs_in_the_private_storage_lab() -> Result<(), Box<dyn Error>> {
    run_inner_case(
        "logical",
        "mdraid_create_stop_start_delete_and_invalid_members",
    )
}

#[test]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn application_registry_runs_in_the_private_storage_lab() -> Result<(), Box<dyn Error>> {
    run_inner_case(
        "application",
        "production_registry_partition_workflow_and_error_mapping",
    )
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

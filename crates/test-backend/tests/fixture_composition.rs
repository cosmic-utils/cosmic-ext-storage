mod common;

use common::{paths::scratch, scenario};
use rstest::{fixture, rstest};
use std::{fs, path::Path, process::Command};
use storage_contracts::{DiskDiscovery, PartitionOperations};
use tempfile::TempDir;
use test_backend::ScenarioRuntime;

#[rstest]
#[case::partition("physical/partition-format.toml", 1)]
#[case::empty("empty.toml", 0)]
#[tokio::test(flavor = "current_thread")]
async fn async_fixture_override(
    #[case] _file: &str,
    #[case] expected_disks: usize,
    #[future(awt)]
    #[with(_file)]
    scenario: ScenarioRuntime,
) {
    assert_eq!(
        scenario.backend().list_disks().await.unwrap().len(),
        expected_disks
    );
}

#[rstest]
#[case::first(1_048_576)]
#[case::second(2_097_152)]
#[tokio::test(flavor = "current_thread")]
async fn independent_mutable_scenarios(
    #[case] offset: u64,
    #[future(awt)]
    #[with("physical/partition-format.toml")]
    scenario: ScenarioRuntime,
) {
    let backend = scenario.backend();
    let other = common::scenario("physical/partition-format.toml").await;
    assert!(
        backend
            .list_partitions("/dev/ui-disk0")
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        backend
            .create_partition("/dev/ui-disk0", offset, 4096, "linux")
            .await
            .unwrap(),
        "/dev/ui-disk0p1"
    );
    assert_eq!(
        scenario
            .backend()
            .list_partitions("/dev/ui-disk0")
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        other
            .backend()
            .list_partitions("/dev/ui-disk0")
            .await
            .unwrap()
            .is_empty()
    );
}

#[rstest]
fn owned_fixture_drops_normally(scratch: TempDir) {
    let path = scratch.path().to_owned();
    assert!(path.exists());
    drop(scratch);
    assert!(!path.exists());
}

#[fixture]
fn dependent_scratch(scratch: TempDir) -> TempDir {
    scratch
}

#[rstest]
fn fixture_dependencies_are_factories(scratch: TempDir, dependent_scratch: TempDir) {
    assert_ne!(scratch.path(), dependent_scratch.path());
}

#[fixture]
fn recorded_root(scratch: TempDir) -> TempDir {
    let record = std::env::var_os("STORAGE_RSTEST_RECORD").expect("parent record path");
    fs::write(record, scratch.path().as_os_str().as_encoded_bytes()).unwrap();
    scratch
}

#[fixture]
fn failing_setup(recorded_root: TempDir) -> TempDir {
    assert!(recorded_root.path().exists());
    panic!("deliberate fixture setup failure");
}

#[rstest]
#[ignore = "deliberate failure selected only by the cleanup parent"]
fn setup_failure_probe(failing_setup: TempDir) {
    // A marker distinguishes a real setup failure from a failing test body.
    let record = std::env::var_os("STORAGE_RSTEST_RECORD").unwrap();
    fs::write(Path::new(&record).with_extension("body"), "entered").unwrap();
    drop(failing_setup);
}

#[rstest]
#[ignore = "deliberate failure selected only by the cleanup parent"]
fn panic_cleanup_probe(recorded_root: TempDir) {
    assert!(recorded_root.path().exists());
    panic!("deliberate test body failure");
}

#[rstest]
#[case::setup("setup_failure_probe", "deliberate fixture setup failure")]
#[case::body("panic_cleanup_probe", "deliberate test body failure")]
fn failure_is_reported_and_resources_unwind(
    #[case] helper: &str,
    #[case] message: &str,
    scratch: TempDir,
) {
    let executable = std::env::current_exe().unwrap();
    let listing = Command::new(&executable)
        .args(["--ignored", "--exact", helper, "--list"])
        .output()
        .unwrap();
    assert!(listing.status.success());
    assert_eq!(
        String::from_utf8_lossy(&listing.stdout)
            .lines()
            .filter(|line| *line == format!("{helper}: test"))
            .count(),
        1
    );
    let record = scratch.path().join("owned-path");
    let output = Command::new(executable)
        .args(["--exact", helper, "--ignored"])
        .env("STORAGE_RSTEST_RECORD", &record)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(101));
    assert!(String::from_utf8_lossy(&output.stdout).contains(message));
    let removed = fs::read_to_string(&record).unwrap();
    assert!(!Path::new(&removed).exists());
    assert!(!record.with_extension("body").exists());
}

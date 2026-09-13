//! The host-side Testcontainers bridge for the private storage lab.
//!
//! Assertions about storage behaviour run in the baked Rust binary inside the
//! lab. This outer test owns only the container lifecycle and evidence capture.

#![cfg(feature = "outer-bridge")]

use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use testcontainers::{
    GenericImage, ImageExt,
    core::{ExecCommand, WaitFor},
    runners::SyncRunner,
};

const IMAGE_NAME: &str = "cosmic-storage-lab";
const IMAGE_TAG: &str = "local";
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

fn run_inner_test(filter: &str) -> Result<(), Box<dyn Error>> {
    if std::env::var("STORAGE_LAB").as_deref() != Ok("1") {
        return Err("STORAGE_LAB=1 is required to execute the private storage lab".into());
    }

    let artifact_dir = unique_artifact_dir()?;
    let image = GenericImage::new(IMAGE_NAME, IMAGE_TAG)
        .with_wait_for(WaitFor::message_on_stdout("STORAGE_LAB_READY"))
        .with_network("none")
        .with_privileged(true)
        .with_label("com.cosmic.storage.lab", "testing-v2");
    let container = image.start()?;

    write_artifact(&artifact_dir, "container-id.txt", container.id())?;
    let (devices, devices_stderr, _) = execute(
        &container,
        [
            "sh",
            "-ec",
            "cat /proc/devices; ls -l /dev/dm-* /dev/mapper/control; dmsetup info -c; lsblk -o NAME,MAJ:MIN,TYPE,MOUNTPOINTS",
        ],
    )?;
    write_artifact(&artifact_dir, "devices-before.stdout.log", &devices)?;
    write_artifact(&artifact_dir, "devices-before.stderr.log", &devices_stderr)?;
    let mut test = container.exec(
        ExecCommand::new(["/usr/local/bin/storage-lab-run-tests"])
            .with_env_vars([("STORAGE_LAB_TEST_FILTER", filter)]),
    )?;
    let test_stdout = String::from_utf8(test.stdout_to_vec()?)?;
    let test_stderr = String::from_utf8(test.stderr_to_vec()?)?;
    let test_exit = test.exit_code()?;
    write_artifact(&artifact_dir, "inner-test.stdout.log", &test_stdout)?;
    write_artifact(&artifact_dir, "inner-test.stderr.log", &test_stderr)?;

    let (service_stdout, service_stderr, service_exit) = execute(
        &container,
        [
            "sh",
            "-ec",
            "cat /tmp/storage-lab/polkitd.log /tmp/storage-lab/udisksd.log /tmp/storage-lab/sshd.log /tmp/storage-lab-evidence/*.ledger 2>/dev/null || true",
        ],
    )?;
    write_artifact(&artifact_dir, "services.stdout.log", &service_stdout)?;
    write_artifact(&artifact_dir, "services.stderr.log", &service_stderr)?;
    write_artifact(
        &artifact_dir,
        "services.exit-code.txt",
        &format!("{service_exit:?}\n"),
    )?;

    let (loops_stdout, loops_stderr, loops_exit) = execute(
        &container,
        ["losetup", "--list", "--noheadings", "--output", "BACK-FILE"],
    )?;
    write_artifact(&artifact_dir, "post-test-loops.stdout.log", &loops_stdout)?;
    write_artifact(&artifact_dir, "post-test-loops.stderr.log", &loops_stderr)?;
    write_artifact(
        &artifact_dir,
        "post-test-loops.exit-code.txt",
        &format!("{loops_exit:?}\n"),
    )?;

    assert_eq!(
        test_exit,
        Some(0),
        "the inner Rust test failed; inspect {}",
        artifact_dir.display()
    );
    assert!(
        !service_stdout.contains("cleanup-failed\t"),
        "fixture cleanup failed; inspect {}",
        artifact_dir.display()
    );
    assert!(
        loops_exit == Some(0),
        "post-test leak check failed; inspect {}",
        artifact_dir.display()
    );
    assert!(
        !loops_stdout.contains("/tmp/storage-lab/"),
        "a lab-backed loop survived the inner test; inspect {}",
        artifact_dir.display()
    );
    Ok(())
}

fn execute<I>(
    container: &testcontainers::Container<GenericImage>,
    command: I,
) -> Result<(String, String, Option<i64>), Box<dyn Error>>
where
    I: IntoIterator<Item = &'static str>,
{
    let mut result = container.exec(ExecCommand::new(command))?;
    let stdout = String::from_utf8(result.stdout_to_vec()?)?;
    let stderr = String::from_utf8(result.stderr_to_vec()?)?;
    let exit_code = result.exit_code()?;
    Ok((stdout, stderr, exit_code))
}

fn unique_artifact_dir() -> Result<PathBuf, Box<dyn Error>> {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let directory = workspace_target_dir()
        .join("storage-lab-artifacts")
        .join(format!("run-{nonce}"));
    fs::create_dir_all(&directory)?;
    Ok(directory)
}

fn workspace_target_dir() -> PathBuf {
    if let Some(directory) = std::env::var_os("STORAGE_LAB_ARTIFACT_ROOT") {
        return PathBuf::from(directory);
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target")
}

fn write_artifact(directory: &Path, name: &str, contents: &str) -> Result<(), Box<dyn Error>> {
    fs::write(directory.join(name), contents)?;
    Ok(())
}

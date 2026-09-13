//! The host-side Testcontainers bridge for the private storage lab.
//!
//! Assertions about storage behaviour run in the baked Rust binary inside the
//! lab. This outer test owns only the container lifecycle and evidence capture.

#![cfg(feature = "outer-bridge")]

use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};
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

#[test]
#[ignore = "requires STORAGE_LAB=1 and the locally built privileged storage-lab image"]
fn partition_lifecycle_runs_in_the_private_storage_lab() -> Result<(), Box<dyn Error>> {
    run_inner_case("partitions", "partition_create_edit_delete_round_trip")
}

fn run_inner_test(filter: &str) -> Result<(), Box<dyn Error>> {
    run_inner_case("capability", filter)
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

fn run_inner_case(target: &str, filter: &str) -> Result<(), Box<dyn Error>> {
    capture_inner_case(target, filter)?.verify()
}

struct InnerOutcome {
    artifact_dir: PathBuf,
    exit: Option<i64>,
    services: String,
    loops: String,
    loops_exit: Option<i64>,
}

impl InnerOutcome {
    fn verify(&self) -> Result<(), Box<dyn Error>> {
        let failure = if self.exit != Some(0) {
            Some("inner Rust test failed")
        } else if self.services.contains("cleanup-failed\t") {
            Some("fixture cleanup failed")
        } else if self.loops_exit != Some(0) {
            Some("post-test leak check failed")
        } else if self.loops.contains("/tmp/storage-lab/") {
            Some("a lab-backed loop survived the inner test")
        } else {
            None
        };
        if let Some(failure) = failure {
            return Err(format!("{failure}; inspect {}", self.artifact_dir.display()).into());
        }
        Ok(())
    }
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

fn capture_inner_case(target: &str, filter: &str) -> Result<InnerOutcome, Box<dyn Error>> {
    if std::env::var("STORAGE_LAB").as_deref() != Ok("1") {
        return Err("STORAGE_LAB=1 is required to execute the private storage lab".into());
    }

    let artifact_dir = unique_artifact_dir()?;
    let mut image = GenericImage::new(IMAGE_NAME, IMAGE_TAG)
        .with_wait_for(WaitFor::message_on_stdout("STORAGE_LAB_READY"))
        .with_network("none")
        .with_privileged(true)
        .with_label("com.cosmic.storage.lab", "testing-v2");
    if target == "btrfs" {
        // libblockdev checks libkmod's on-disk index even for a running or
        // built-in driver. Supply immutable copies of the current kernel's
        // real metadata (and only its Btrfs module when loadable), never a
        // host bind mount, fabricated module, or replacement command.
        let release = Command::new("uname").arg("-r").output()?;
        if !release.status.success() {
            return Err("cannot read kernel release".into());
        }
        let release = String::from_utf8(release.stdout)?.trim().to_owned();
        if release.is_empty()
            || !release
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._+-".contains(&byte))
        {
            return Err("invalid kernel release".into());
        }
        let module_root = PathBuf::from(format!("/lib/modules/{release}")).canonicalize()?;
        let mut sources = Vec::new();
        for name in [
            "modules.dep",
            "modules.dep.bin",
            "modules.builtin",
            "modules.builtin.bin",
            "modules.builtin.modinfo",
            "modules.builtin.alias.bin",
        ] {
            let source = module_root.join(name);
            if source.is_file() {
                sources.push(source);
            }
        }
        let module = Command::new("modinfo")
            .args(["-F", "filename", "btrfs"])
            .output()?;
        if !module.status.success() {
            return Err("Btrfs kernel metadata is unavailable".into());
        }
        let module = String::from_utf8(module.stdout)?.trim().to_owned();
        if module != "(builtin)" {
            sources.push(PathBuf::from(module).canonicalize()?);
        }
        let mut manifest = format!("kernel\t{release}\n");
        for source in sources {
            let canonical = source.canonicalize()?;
            let relative = canonical.strip_prefix(&module_root)?;
            if fs::metadata(&canonical)?.len() > 64 * 1024 * 1024 {
                return Err("kernel payload exceeds the bounded input size".into());
            }
            let bytes = fs::read(&canonical)?;
            let destination = format!("/lib/modules/{release}/{}", relative.display());
            manifest.push_str(&format!("{destination}\t{:x}\n", Sha256::digest(&bytes)));
            image = image.with_copy_to(
                testcontainers::CopyTargetOptions::new(destination).with_mode(0o444),
                bytes,
            );
        }
        write_artifact(&artifact_dir, "kernel-payload.txt", &manifest)?;
        image = image.with_env_var("STORAGE_LAB_KERNEL_RELEASE", release);
    }
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
        ExecCommand::new(["/usr/local/bin/storage-lab-run-tests"]).with_env_vars([
            ("STORAGE_LAB_TEST_FILTER", filter),
            ("STORAGE_LAB_TEST_TARGET", target),
            (
                "LLVM_PROFILE_FILE",
                "/tmp/storage-lab-profiles/test-%m-%p.profraw",
            ),
        ]),
    )?;
    let test_stdout = String::from_utf8(test.stdout_to_vec()?)?;
    let test_stderr = String::from_utf8(test.stderr_to_vec()?)?;
    let test_exit = test.exit_code()?;
    write_artifact(&artifact_dir, "inner-test.stdout.log", &test_stdout)?;
    write_artifact(&artifact_dir, "inner-test.stderr.log", &test_stderr)?;
    write_artifact(
        &artifact_dir,
        "inner-test.exit-code.txt",
        &format!("{test_exit:?}\n"),
    )?;
    write_artifact(
        &artifact_dir,
        "inner-test.selection.txt",
        &format!("{target}\t{filter}\n"),
    )?;

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

    if std::env::var("STORAGE_LAB_COVERAGE").as_deref() == Ok("1") {
        // Collect after service/cleanup evidence but before interpreting status,
        // including 101. No bind mount or Docker CLI copy is permitted.
        let profiles = artifact_dir.join("profiles");
        fs::create_dir(&profiles)?;
        let archive_command = format!(
            "test \"$(cat /opt/storage-lab/bin/coverage-mode)\" = 1 && tar -czf - -C / tmp/storage-lab-profiles opt/storage-lab/bin/storage-lab-{target}"
        );
        // `target` comes only from literal bridge tests, never user input.
        let mut archive = container.exec(ExecCommand::new(["sh", "-ec", &archive_command]))?;
        fs::write(profiles.join("inner.tar.gz"), archive.stdout_to_vec()?)?;
        fs::write(
            profiles.join("archive.stderr.log"),
            archive.stderr_to_vec()?,
        )?;
        if archive.exit_code()? != Some(0) {
            return Err(format!("coverage archive failed; inspect {}", profiles.display()).into());
        }
    }
    Ok(InnerOutcome {
        artifact_dir,
        exit: test_exit,
        services: service_stdout,
        loops: loops_stdout,
        loops_exit,
    })
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

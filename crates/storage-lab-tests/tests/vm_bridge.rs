//! Disposable-QEMU capability spike for the Testcontainers storage lab.
//!
//! This is intentionally a separate integration-test target. The normal lab
//! remains hermetic on a GitHub-hosted Docker runner, where the host kernel
//! cannot create usable device-mapper mappings. This bridge is run *inside* a
//! guest VM and asks whether that guest kernel makes the exact same image and
//! Testcontainers lifecycle capable of exercising the LUKS path.

#![cfg(feature = "vm-spike")]

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
const LUKS_FILTER: &str = "luks_unlock_rejects_bad_secret_and_locks_cleanly";

#[test]
#[ignore = "runs only inside the disposable-QEMU capability spike"]
fn luks_runs_in_a_guest_vm_through_testcontainers() -> Result<(), Box<dyn Error>> {
    if std::env::var("STORAGE_LAB").as_deref() != Ok("1") {
        return Err("STORAGE_LAB=1 is required to execute the VM capability spike".into());
    }

    let artifact_dir = unique_artifact_dir()?;
    let image = GenericImage::new(IMAGE_NAME, IMAGE_TAG)
        .with_wait_for(WaitFor::message_on_stdout("STORAGE_LAB_READY"))
        .with_network("none")
        .with_privileged(true)
        .with_label("com.cosmic.storage.lab", "testing-v2-vm-spike");
    let container = image.start()?;
    write_artifact(&artifact_dir, "container-id.txt", container.id())?;

    let mut test = container.exec(
        ExecCommand::new(["/usr/local/bin/storage-lab-run-tests"])
            .with_env_vars([("STORAGE_LAB_TEST_FILTER", LUKS_FILTER)]),
    )?;
    let stdout = String::from_utf8(test.stdout_to_vec()?)?;
    let stderr = String::from_utf8(test.stderr_to_vec()?)?;
    let exit_code = test.exit_code()?;
    write_artifact(&artifact_dir, "inner-test.stdout.log", &stdout)?;
    write_artifact(&artifact_dir, "inner-test.stderr.log", &stderr)?;

    let mut services = container.exec(ExecCommand::new([
        "sh",
        "-ec",
        "cat /tmp/storage-lab/polkitd.log /tmp/storage-lab/udisksd.log /tmp/storage-lab/sshd.log 2>/dev/null || true",
    ]))?;
    let services_stdout = String::from_utf8(services.stdout_to_vec()?)?;
    let services_stderr = String::from_utf8(services.stderr_to_vec()?)?;
    write_artifact(&artifact_dir, "services.stdout.log", &services_stdout)?;
    write_artifact(&artifact_dir, "services.stderr.log", &services_stderr)?;

    let mut device_mapper = container.exec(ExecCommand::new([
        "sh",
        "-ec",
        "dmsetup version; ls -l /dev/mapper /dev/dm-* 2>&1 || true",
    ]))?;
    let dm_stdout = String::from_utf8(device_mapper.stdout_to_vec()?)?;
    let dm_stderr = String::from_utf8(device_mapper.stderr_to_vec()?)?;
    write_artifact(&artifact_dir, "device-mapper.stdout.log", &dm_stdout)?;
    write_artifact(&artifact_dir, "device-mapper.stderr.log", &dm_stderr)?;

    assert_eq!(
        exit_code,
        Some(0),
        "the guest VM still could not run the LUKS path; inspect {}",
        artifact_dir.display()
    );
    Ok(())
}

fn unique_artifact_dir() -> Result<PathBuf, Box<dyn Error>> {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root = std::env::var_os("STORAGE_LAB_ARTIFACT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    let directory = root
        .join("storage-lab-artifacts")
        .join(format!("vm-{nonce}"));
    fs::create_dir_all(&directory)?;
    Ok(directory)
}

fn write_artifact(directory: &Path, name: &str, contents: &str) -> Result<(), Box<dyn Error>> {
    fs::write(directory.join(name), contents)?;
    Ok(())
}

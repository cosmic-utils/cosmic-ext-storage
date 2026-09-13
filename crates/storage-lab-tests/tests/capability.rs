use std::{os::unix::fs::FileTypeExt, process::Command};

use std::sync::Arc;
use storage_contracts::{
    DiskDiscovery, EncryptionOperations, FilesystemOperations, PartitionOperations,
};
use storage_lab_tests::{LabFixture, Result};
use storage_udisks::UdisksBackend;
use storage_udisks::storage_types::FormatOptions;
use zbus::Connection;

/// This is compiled into the lab image and executed there by the outer bridge.
#[tokio::test]
#[ignore = "runs only inside the private Testcontainers storage lab"]
async fn capability_starts_private_dbus_udisks_and_sftp() -> Result<()> {
    require_success(Command::new("dbus-send").args([
        "--system",
        "--dest=org.freedesktop.UDisks2",
        "--print-reply",
        "/org/freedesktop/UDisks2",
        "org.freedesktop.DBus.Peer.Ping",
    ]))?;
    let sockets = require_success(Command::new("ss").args(["--tcp", "--listening", "--numeric"]))?;
    assert!(
        String::from_utf8_lossy(&sockets.stdout).contains(":2222"),
        "the container-local SFTP service is not listening on port 2222"
    );

    let mut fixture = LabFixture::create("capability")?;
    let loop_device = fixture.attach_sparse_loop("disk.img", 64 * 1024 * 1024)?;
    let loop_path = loop_device.path().to_owned();
    assert!(std::fs::metadata(&loop_path)?.file_type().is_block_device());
    assert!(fixture.ledger_path().is_file());

    let backend = private_backend().await?;
    wait_for_discovery(&backend, &loop_path).await?;
    fixture.cleanup()?;
    Ok(())
}

/// This executes the real adapter mutation and discovery path against the
/// same private connection selected for the test, not the process-global
/// system-bus helper.
#[tokio::test]
#[ignore = "runs only inside the private Testcontainers storage lab"]
async fn partition_table_round_trip_uses_the_private_adapter_transport() -> Result<()> {
    let mut fixture = LabFixture::create("partition-table")?;
    let loop_device = fixture.attach_sparse_loop("disk.img", 128 * 1024 * 1024)?;
    let loop_path = loop_device.path().to_owned();
    let backend = private_backend().await?;
    wait_for_discovery(&backend, &loop_path).await?;

    backend
        .create_partition_table(&loop_path.to_string_lossy(), "gpt")
        .await
        .map_err(|error| error.to_string())?;
    let partition_type = require_success(Command::new("lsblk").args([
        "--noheadings",
        "--output",
        "PTTYPE",
        loop_path.to_string_lossy().as_ref(),
    ]))?;
    assert_eq!(
        String::from_utf8_lossy(&partition_type.stdout).trim(),
        "gpt"
    );
    let partitions = backend
        .list_partitions(&loop_path.to_string_lossy())
        .await
        .map_err(|error| error.to_string())?;
    assert!(
        partitions.is_empty(),
        "new GPT table must start without partitions"
    );

    fixture.cleanup()?;
    Ok(())
}

#[test]
#[ignore = "runs only inside the private Testcontainers storage lab"]
fn dropping_a_fixture_detaches_its_ledgered_loop() -> Result<()> {
    let mut fixture = LabFixture::create("drop-cleanup")?;
    let loop_device = fixture.attach_sparse_loop("disk.img", 64 * 1024 * 1024)?;
    let loop_path = loop_device.path().to_owned();
    let root = fixture.root()?.path().to_owned();

    drop(fixture);

    assert!(!root.exists(), "fixture root must be removed on drop");
    let mappings = require_success(Command::new("losetup").args([
        "--list",
        "--noheadings",
        "--output",
        "BACK-FILE",
        loop_path.to_string_lossy().as_ref(),
    ]))?;
    assert!(
        String::from_utf8_lossy(&mappings.stdout).trim().is_empty(),
        "a dropped fixture left a loop backing mapping"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "runs only inside the private Testcontainers storage lab"]
async fn filesystem_format_and_label_use_the_production_adapter() -> Result<()> {
    let mut fixture = LabFixture::create("filesystem")?;
    let loop_device = fixture.attach_sparse_loop("disk.img", 128 * 1024 * 1024)?;
    let loop_path = loop_device.path().to_owned();
    let backend = private_backend().await?;
    wait_for_discovery(&backend, &loop_path).await?;

    let invalid = backend
        .format_filesystem(
            &loop_path.to_string_lossy(),
            "not-a-filesystem",
            "invalid",
            FormatOptions::default(),
        )
        .await;
    assert!(
        invalid.is_err(),
        "UDisks must reject an unknown filesystem type"
    );

    backend
        .format_filesystem(
            &loop_path.to_string_lossy(),
            "ext4",
            "storage-lab",
            FormatOptions::default(),
        )
        .await
        .map_err(|error| error.to_string())?;
    assert_eq!(
        backend
            .filesystem_label(&loop_path.to_string_lossy())
            .await
            .map_err(|error| error.to_string())?,
        "storage-lab"
    );
    let filesystem_type = require_success(Command::new("blkid").args([
        "--probe",
        "--output",
        "value",
        "--match-tag",
        "TYPE",
        loop_path.to_string_lossy().as_ref(),
    ]))?;
    assert_eq!(
        String::from_utf8_lossy(&filesystem_type.stdout).trim(),
        "ext4"
    );

    fixture.cleanup()?;
    Ok(())
}

#[tokio::test]
#[ignore = "runs only inside the private Testcontainers storage lab"]
async fn luks_unlock_rejects_bad_secret_and_locks_cleanly() -> Result<()> {
    let mut fixture = LabFixture::create("luks")?;
    let loop_device = fixture.attach_sparse_loop("disk.img", 128 * 1024 * 1024)?;
    let loop_path = loop_device.path().to_owned();
    let backend = private_backend().await?;
    wait_for_discovery(&backend, &loop_path).await?;

    let passphrase = "storage-lab-secret";
    backend
        .format_luks(&loop_path.to_string_lossy(), passphrase, "luks2")
        .await
        .map_err(|error| error.to_string())?;
    // UDisks automatically unlocks after formatting. Test bad credentials only
    // after closing that mapping, otherwise "already unlocked" proves nothing.
    backend
        .lock_luks(&loop_path.to_string_lossy())
        .await
        .map_err(|error| error.to_string())?;
    assert!(
        backend
            .unlock_luks(&loop_path.to_string_lossy(), "wrong-secret")
            .await
            .is_err(),
        "a wrong LUKS passphrase must be rejected"
    );
    let cleartext = backend
        .unlock_luks(&loop_path.to_string_lossy(), passphrase)
        .await
        .map_err(|error| error.to_string())?;
    fixture.track_mapper(std::path::Path::new(&cleartext))?;
    assert!(
        std::fs::metadata(&cleartext)?.file_type().is_block_device(),
        "UDisks must return a cleartext block device"
    );
    backend
        .lock_luks(&loop_path.to_string_lossy())
        .await
        .map_err(|error| error.to_string())?;

    fixture.cleanup()?;
    Ok(())
}

#[test]
#[ignore = "runs only inside the private Testcontainers storage lab"]
fn failing_case_still_removes_all_ledgered_resources() -> Result<()> {
    let mut fixture = LabFixture::create("intentional-unwind")?;
    let root = fixture.root()?.path().to_owned();
    let ledger = fixture.ledger_path().to_owned();
    let first = fixture
        .attach_sparse_loop("first.img", 64 * 1024 * 1024)?
        .path()
        .to_owned();
    let second = fixture
        .attach_sparse_loop("second.img", 64 * 1024 * 1024)?
        .path()
        .to_owned();
    assert_ne!(first, second);
    // The test succeeds only when an actual panic unwinds the fixture and
    // independent kernel observations prove both resources were removed.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _owned_until_unwind = fixture;
        panic!("deliberate inner case failure");
    }));
    assert!(outcome.is_err());
    assert!(!root.exists());
    let output = require_success(Command::new("losetup").args([
        "--list",
        "--noheadings",
        "--output",
        "BACK-FILE",
    ]))?;
    assert!(!String::from_utf8_lossy(&output.stdout).contains(root.to_string_lossy().as_ref()));
    let evidence = std::fs::read_to_string(ledger)?;
    assert_eq!(
        evidence
            .lines()
            .filter(|line| line.starts_with("loop\t"))
            .count(),
        2
    );
    assert_eq!(
        evidence
            .lines()
            .filter(|line| line.starts_with("detached\t"))
            .count(),
        2
    );
    assert!(!evidence.contains("cleanup-failed"));
    Ok(())
}

async fn private_backend() -> Result<UdisksBackend> {
    // This is deliberately the production adapter, attached to the lab's
    // private D-Bus socket. No host D-Bus bridge or test-only backend is used.
    let connection = Connection::system()
        .await
        .map_err(|error| error.to_string())?;
    Ok(UdisksBackend::from_connection(Arc::new(connection)))
}

async fn wait_for_discovery(backend: &UdisksBackend, loop_path: &std::path::Path) -> Result<()> {
    for _ in 0..100 {
        let discovered = backend
            .list_disks()
            .await
            .map_err(|error| error.to_string())?;
        if discovered
            .iter()
            .any(|disk| disk.device == loop_path.to_string_lossy())
        {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    Err(
        "the production UDisks adapter did not discover the lab-owned loop device"
            .to_owned()
            .into(),
    )
}

fn require_success(command: &mut Command) -> Result<std::process::Output> {
    let output = command.output()?;
    if output.status.success() {
        return Ok(output);
    }
    Err(format!(
        "command failed with {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr).trim()
    )
    .into())
}
/// The outer bridge must observe this failure, not turn it into a successful
/// catalog result. It selects this exact native test only for that regression.
#[test]
#[ignore = "deliberately fails; executed only by the bridge failure-propagation regression"]
fn deliberate_failure_after_fixture_cleanup() {
    let mut fixture = storage_lab_tests::LabFixture::create("deliberate-failure").unwrap();
    fixture
        .attach_sparse_loop("disk.img", 8 * 1024 * 1024)
        .unwrap();
    fixture.cleanup().unwrap();
    panic!("deliberate inner assertion failure");
}

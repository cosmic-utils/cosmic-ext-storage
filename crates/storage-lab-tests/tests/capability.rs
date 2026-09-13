use std::{os::unix::fs::FileTypeExt, process::Command};

use std::sync::Arc;
use storage_contracts::{DiskDiscovery, PartitionOperations};
use storage_lab_tests::{LabFixture, Result};
use storage_udisks::UdisksBackend;
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
    assert!(fixture.root()?.path().join("ledger.txt").is_file());

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

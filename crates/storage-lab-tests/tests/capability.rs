use std::{os::unix::fs::FileTypeExt, process::Command};

use std::sync::Arc;
use storage_contracts::DiskDiscovery;
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

    // This is deliberately the production adapter, attached to the lab's
    // private D-Bus socket. No host D-Bus bridge or test-only backend is used.
    let backend = UdisksBackend::from_connection(Arc::new(
        Connection::system()
            .await
            .map_err(|error| error.to_string())?,
    ));
    let mut discovered = Vec::new();
    for _ in 0..100 {
        discovered = backend
            .list_disks()
            .await
            .map_err(|error| error.to_string())?;
        if discovered
            .iter()
            .any(|disk| disk.device == loop_path.to_string_lossy())
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(
        discovered
            .iter()
            .any(|disk| disk.device == loop_path.to_string_lossy()),
        "the production UDisks adapter did not discover the lab-owned loop device"
    );
    fixture.cleanup()?;
    Ok(())
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

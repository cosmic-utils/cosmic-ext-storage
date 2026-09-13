use std::{os::unix::fs::FileTypeExt, process::Command};

use storage_lab_tests::{LabFixture, Result};

/// This is compiled into the lab image and executed there by the outer bridge.
#[test]
#[ignore = "runs only inside the private Testcontainers storage lab"]
fn capability_starts_private_dbus_udisks_and_sftp() -> Result<()> {
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
    assert!(
        std::fs::metadata(loop_device.path())?
            .file_type()
            .is_block_device()
    );
    assert!(fixture.root()?.path().join("ledger.txt").is_file());
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

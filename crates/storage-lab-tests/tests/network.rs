use std::{collections::BTreeMap, process::Command};
use storage_contracts::NetworkDriveBackend;
use storage_lab_tests::{LabFixture, Result};
use storage_udisks::storage_types::{NetworkBackendId, NetworkDriveConfig, NetworkDriveStatus};

#[tokio::test]
#[ignore = "runs only inside the private Testcontainers storage lab"]
async fn local_sftp_config_test_mount_unmount_and_failures() -> Result<()> {
    let mut fixture = LabFixture::create("local-sftp")?;
    let mount = fixture.prepare_network_mount("lab-sftp")?;
    let backend = storage_sys::RcloneNetworkBackend::with_home(fixture.root()?.path().to_owned())
        .map_err(|e| e.to_string())?;
    let obscured = Command::new("rclone")
        .args(["obscure", "storage-lab-fake-password"])
        .output()?;
    assert!(obscured.status.success());
    let mut config = NetworkDriveConfig {
        backend_id: NetworkBackendId::rclone(),
        id: "lab-sftp".into(),
        name: "lab-sftp".into(),
        provider_id: "sftp".into(),
        has_secrets: true,
        options: BTreeMap::from([
            ("host".into(), "127.0.0.1".into()),
            ("port".into(), "2222".into()),
            ("user".into(), "storage-lab-sftp".into()),
            (
                "pass".into(),
                String::from_utf8_lossy(&obscured.stdout).trim().into(),
            ),
            ("shell_type".into(), "unix".into()),
        ]),
    };
    backend.create_config(&config).await?;
    assert!(
        backend.create_config(&config).await.is_err(),
        "duplicate remote must fail"
    );
    assert_eq!(backend.list_configs().await?.configs.len(), 1);
    assert!(backend.test_config(&config.id).await?.success);
    backend.mount(&config.id).await?;
    assert_eq!(
        backend.mount_status(&config.id).await?.status,
        NetworkDriveStatus::Mounted
    );
    assert_eq!(backend.mount_status(&config.id).await?.mount_point, mount);
    std::fs::write(mount.join("payload"), b"container local SFTP")?;
    assert_eq!(
        std::fs::read(mount.join("payload"))?,
        b"container local SFTP"
    );
    backend.unmount(&config.id).await?;
    assert_eq!(
        backend.mount_status(&config.id).await?.status,
        NetworkDriveStatus::Unmounted
    );
    config.options.insert("port".into(), "1".into());
    backend.update_config(&config).await?;
    assert!(
        !backend.test_config(&config.id).await?.success,
        "closed local port must fail"
    );
    backend.delete_config(&config.id).await?;
    assert!(backend.list_configs().await?.configs.is_empty());
    fixture.cleanup()?;
    Ok(())
}

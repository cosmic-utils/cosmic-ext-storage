mod common;
use common::{lab, owned};
use std::{fs, path::Path, process::Command};
use storage_contracts::FilesystemOperations;
use storage_lab_tests::Result;
use storage_udisks::storage_types::{FormatOptions, MountOptions};

#[tokio::test]
#[ignore = "runs only inside the private Testcontainers storage lab"]
async fn filesystem_mount_options_busy_retry_and_cleanup() -> Result<()> {
    let (mut fixture, backend, disk) = lab("filesystem-mount", 128).await?;
    backend
        .format_filesystem(
            owned(&fixture, &disk)?,
            "ext4",
            "mount-test",
            FormatOptions::default(),
        )
        .await?;
    assert!(
        backend
            .check_filesystem(owned(&fixture, &disk)?, false)
            .await?
    );
    let device = fixture.owned_loop(Path::new(&disk))?.clone();
    let mount = fixture.prepare_mount(&device, "mount")?;
    // UDisks uses fstab for an explicit target; its Mount method otherwise
    // auto-selects a /media path and ignores the application's mount-point hint.
    backend
        .set_mount_options(
            owned(&fixture, &disk)?,
            false,
            false,
            false,
            None,
            None,
            None,
            "defaults,noexec,nosuid".into(),
            mount.to_string_lossy().into_owned(),
            disk.clone(),
            "ext4".into(),
        )
        .await?;
    let mounted = backend
        .mount_filesystem(
            owned(&fixture, &disk)?,
            &mount.to_string_lossy(),
            MountOptions::default(),
        )
        .await?;
    assert_eq!(Path::new(&mounted), mount);
    assert_eq!(backend.get_mount_point(&disk).await?, mounted);
    fs::write(mount.join("payload"), b"storage-lab round trip")?;
    let scan = storage_sys::usage::scan_local_mounts(
        &mount,
        &storage_sys::usage::ScanConfig {
            show_all_files: true,
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    assert_eq!(scan.mounts_scanned, 1);
    assert!(scan.files_scanned >= 1);
    assert!(scan.total_bytes >= b"storage-lab round trip".len() as u64);
    assert_eq!(scan.skipped_errors, 0);
    let options = Command::new("findmnt")
        .args(["-n", "-o", "OPTIONS", "--mountpoint"])
        .arg(&mount)
        .output()?;
    assert!(options.status.success());
    assert!(String::from_utf8_lossy(&options.stdout).contains("noexec"));
    let mut holder = Command::new("sleep")
        .arg("30")
        .current_dir(&mount)
        .spawn()?;
    let busy = backend
        .unmount_filesystem(owned(&fixture, &disk)?, false)
        .await;
    let failed_cleanup = fixture.cleanup();
    let root_retained = fixture.root().is_ok_and(|root| root.path().exists());
    // Always reap the process before assertions, including when UDisks errs.
    holder.kill()?;
    holder.wait()?;
    assert!(
        busy.is_err(),
        "a process working directory must keep the mount busy"
    );
    assert!(
        failed_cleanup.is_err(),
        "busy cleanup must report its failure"
    );
    assert!(
        root_retained,
        "failed cleanup must retain the backing file root"
    );
    backend
        .unmount_filesystem(owned(&fixture, &disk)?, false)
        .await?;
    assert!(backend.get_mount_point(&disk).await.is_err());
    backend
        .set_mount_options(
            owned(&fixture, &disk)?,
            false,
            false,
            false,
            None,
            None,
            None,
            "ro,noexec,nosuid".into(),
            mount.to_string_lossy().into_owned(),
            disk.clone(),
            "ext4".into(),
        )
        .await?;
    backend
        .mount_filesystem(
            owned(&fixture, &disk)?,
            &mount.to_string_lossy(),
            MountOptions {
                read_only: true,
                ..Default::default()
            },
        )
        .await?;
    assert_eq!(fs::read(mount.join("payload"))?, b"storage-lab round trip");
    assert!(fs::write(mount.join("forbidden"), b"must fail").is_err());
    backend
        .unmount_filesystem(owned(&fixture, &disk)?, false)
        .await?;
    backend.reset_mount_options(owned(&fixture, &disk)?).await?;
    fixture.cleanup()?;
    Ok(())
}

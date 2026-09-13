mod common;
use common::{lab, owned};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    sync::atomic::{AtomicBool, Ordering},
};
use storage_contracts::ImageDeviceOperations;
use storage_lab_tests::Result;

#[tokio::test]
#[ignore = "runs only inside the private Testcontainers storage lab"]
async fn image_backup_restore_round_trip_and_readonly_enforcement() -> Result<()> {
    let (mut fixture, backend, disk) = lab("image-roundtrip", 64).await?;
    let payload = b"storage lab production image descriptor round trip";
    let offset = 1024 * 1024;
    {
        let mut restore = File::from(backend.open_for_restore(owned(&fixture, &disk)?).await?);
        restore.seek(SeekFrom::Start(offset))?;
        restore.write_all(payload)?;
        restore.sync_all()?;
    }
    {
        let mut backup = File::from(backend.open_for_backup(owned(&fixture, &disk)?).await?);
        backup.seek(SeekFrom::Start(offset))?;
        let mut actual = vec![0; payload.len()];
        backup.read_exact(&mut actual)?;
        assert_eq!(actual, payload);
        assert!(
            backup.write_all(b"must fail").is_err(),
            "backup descriptors must not grant mutation authority"
        );
    }
    let backup_path = fixture.root()?.path().join("backup.img");
    let mut progress = Vec::new();
    let count = storage_sys::copy_image_to_file(
        backend.open_for_backup(owned(&fixture, &disk)?).await?,
        &backup_path,
        Some(|count| progress.push(count)),
    )
    .map_err(|e| e.to_string())?;
    assert_eq!(count, 64 * 1024 * 1024);
    assert_eq!(progress.last(), Some(&count));
    assert!(progress.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(
        storage_sys::copy_file_to_image(
            &backup_path,
            backend.open_for_restore(owned(&fixture, &disk)?).await?,
            None::<fn(u64)>
        )
        .map_err(|e| e.to_string())?,
        count
    );
    let cancelled_path = fixture.root()?.path().join("cancelled.img");
    let cancelled = AtomicBool::new(false);
    let result = storage_sys::image::copy_image_to_file_cancellable(
        backend.open_for_backup(owned(&fixture, &disk)?).await?,
        &cancelled_path,
        Some(|_| cancelled.store(true, Ordering::Release)),
        &cancelled,
    );
    assert!(result.unwrap_err().to_string().contains("cancelled"));
    assert_eq!(std::fs::metadata(cancelled_path)?.len(), 1024 * 1024);
    let attached = fixture
        .attach_image_loop(&backend, "attached.img", 16 * 1024 * 1024)
        .await?;
    let read_only = std::fs::read_to_string(format!(
        "/sys/class/block/{}/ro",
        std::path::Path::new(&attached)
            .file_name()
            .unwrap()
            .to_string_lossy()
    ))?;
    assert_eq!(
        read_only.trim(),
        "1",
        "the attached loop must be kernel read-only"
    );
    // Some kernels permit opening a read-only block device writable and reject
    // the write itself. Assert the protection, not one particular failure point.
    if let Ok(descriptor) = backend.open_for_restore(owned(&fixture, &attached)?).await {
        assert!(File::from(descriptor).write_all(&[7; 512]).is_err());
    }
    let mut attached_file = File::from(backend.open_for_backup(owned(&fixture, &attached)?).await?);
    let mut zeroes = [1; 512];
    attached_file.read_exact(&mut zeroes)?;
    assert_eq!(zeroes, [0; 512]);
    drop(attached_file);
    assert!(
        backend
            .loop_setup(fixture.root()?.path().to_str().unwrap())
            .await
            .is_err(),
        "a directory is not an attachable image"
    );
    fixture.cleanup()?;
    Ok(())
}

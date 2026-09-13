use super::*;

#[test]
fn descriptors_transfer_ownership_and_copy_round_trip() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = root.path().join("backup");
    let restored = root.path().join("restored");
    let bytes = vec![0x5a; 2 * 1024 * 1024 + 7];
    std::fs::write(&source, &bytes).unwrap();
    std::fs::write(&restored, []).unwrap();
    let mut progress = Vec::new();
    let count = copy_image_to_file(
        open_for_backup(source.to_str().unwrap()).unwrap(),
        &backup,
        Some(|count| progress.push(count)),
    )
    .unwrap();
    assert_eq!(count, bytes.len() as u64);
    assert_eq!(progress, [1024 * 1024, 2 * 1024 * 1024, count]);
    assert_eq!(
        copy_file_to_image(
            &backup,
            open_for_restore(restored.to_str().unwrap()).unwrap(),
            None::<fn(u64)>
        )
        .unwrap(),
        count
    );
    assert_eq!(std::fs::read(restored).unwrap(), bytes);
    let mut readonly = File::from(open_for_backup(source.to_str().unwrap()).unwrap());
    assert!(readonly.write_all(b"forbidden").is_err());
    let mut writeonly = File::from(open_for_restore(source.to_str().unwrap()).unwrap());
    assert!(writeonly.read(&mut [0; 1]).is_err());
}

#[test]
fn cancelled_copy_stops_before_the_second_chunk() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let dest = root.path().join("dest");
    std::fs::write(&source, vec![7; 3 * 1024 * 1024]).unwrap();
    let cancelled = AtomicBool::new(false);
    let error = copy_image_to_file_cancellable(
        File::open(&source).unwrap().into(),
        &dest,
        Some(|count| {
            assert_eq!(count, 1024 * 1024);
            cancelled.store(true, Ordering::Release);
        }),
        &cancelled,
    )
    .unwrap_err();
    assert!(error.to_string().contains("cancelled"));
    assert_eq!(std::fs::metadata(&dest).unwrap().len(), 1024 * 1024);
    // A pre-cancelled copy must not truncate an existing destination.
    assert!(
        copy_image_to_file_cancellable(
            File::open(&source).unwrap().into(),
            &dest,
            None::<fn(u64)>,
            &cancelled
        )
        .is_err()
    );
    assert_eq!(std::fs::metadata(&dest).unwrap().len(), 1024 * 1024);
    assert!(
        copy_file_to_image_cancellable(
            &source,
            File::create(root.path().join("restore")).unwrap().into(),
            None::<fn(u64)>,
            &cancelled
        )
        .is_err()
    );
}

#[test]
fn missing_paths_and_invalid_descriptors_fail() {
    let root = tempfile::tempdir().unwrap();
    let missing = root.path().join("missing");
    assert!(matches!(
        open_for_backup(missing.to_str().unwrap()),
        Err(SysError::DeviceNotFound(_))
    ));
    assert!(matches!(
        open_for_restore(missing.to_str().unwrap()),
        Err(SysError::DeviceNotFound(_))
    ));
    let source = root.path().join("source");
    std::fs::write(&source, b"payload").unwrap();
    assert!(
        copy_file_to_image(
            &source,
            File::open(&source).unwrap().into(),
            None::<fn(u64)>
        )
        .is_err()
    );
    assert!(
        copy_image_to_file(
            File::create(root.path().join("write-only")).unwrap().into(),
            &root.path().join("backup"),
            None::<fn(u64)>
        )
        .is_err()
    );
    assert!(
        copy_image_to_file(
            File::open(&source).unwrap().into(),
            &missing.join("parent"),
            None::<fn(u64)>
        )
        .is_err()
    );
    assert!(
        copy_file_to_image(
            &missing,
            File::create(root.path().join("dest")).unwrap().into(),
            None::<fn(u64)>
        )
        .is_err()
    );
}

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::mpsc;

use super::*;

#[test]
fn aggregates_category_bytes_over_tree() {
    let temp = tempfile::tempdir().expect("private test directory");
    fs::write(temp.path().join("main.rs"), vec![b'a'; 10]).expect("write rs file");
    fs::write(temp.path().join("pic.png"), vec![b'a'; 20]).expect("write image file");
    fs::write(temp.path().join("note.txt"), vec![b'a'; 30]).expect("write document file");

    let result = scan_paths(&[temp.path().to_path_buf()], &ScanConfig::default())
        .expect("scan should succeed");

    let code = result
        .categories
        .iter()
        .find(|entry| entry.category == Category::Code)
        .map(|entry| entry.bytes)
        .unwrap_or(0);
    let images = result
        .categories
        .iter()
        .find(|entry| entry.category == Category::Images)
        .map(|entry| entry.bytes)
        .unwrap_or(0);
    let documents = result
        .categories
        .iter()
        .find(|entry| entry.category == Category::Documents)
        .map(|entry| entry.bytes)
        .unwrap_or(0);

    assert_eq!(code, 10);
    assert_eq!(images, 20);
    assert_eq!(documents, 30);
    assert_eq!(result.total_bytes, 60);

    let code_top = result
        .top_files_by_category
        .iter()
        .find(|entry| entry.category == Category::Code)
        .expect("code top files should exist");
    assert_eq!(code_top.files.len(), 1);
    assert_eq!(code_top.files[0].bytes, 10);
}

#[test]
fn keeps_only_top_twenty_per_category() {
    let temp = tempfile::tempdir().expect("private test directory");

    for size in 1_u8..=25 {
        let path = temp.path().join(format!("f{size:02}.rs"));
        fs::write(path, vec![b'a'; size as usize]).expect("write file");
    }

    let result = scan_paths(
        &[temp.path().to_path_buf()],
        &ScanConfig {
            threads: None,
            top_files_per_category: 20,
            show_all_files: false,
            caller_uid: None,
            caller_gids: None,
        },
    )
    .expect("scan should succeed");

    let code_top = result
        .top_files_by_category
        .iter()
        .find(|entry| entry.category == Category::Code)
        .expect("code top files should exist");

    assert_eq!(code_top.files.len(), 20);
    assert_eq!(code_top.files.first().expect("has first").bytes, 25);
    assert_eq!(code_top.files.last().expect("has last").bytes, 6);
}

#[test]
fn top_files_limit_is_configurable() {
    let temp = tempfile::tempdir().expect("private test directory");

    for size in 1_u8..=8 {
        let path = temp.path().join(format!("f{size:02}.rs"));
        fs::write(path, vec![b'a'; size as usize]).expect("write file");
    }

    let result = scan_paths(
        &[temp.path().to_path_buf()],
        &ScanConfig {
            threads: None,
            top_files_per_category: 3,
            show_all_files: false,
            caller_uid: None,
            caller_gids: None,
        },
    )
    .expect("scan should succeed");

    let code_top = result
        .top_files_by_category
        .iter()
        .find(|entry| entry.category == Category::Code)
        .expect("code top files should exist");

    assert_eq!(code_top.files.len(), 3);
    assert_eq!(code_top.files[0].bytes, 8);
    assert_eq!(code_top.files[1].bytes, 7);
    assert_eq!(code_top.files[2].bytes, 6);
}

#[test]
fn top_files_sorted_desc_then_path_for_ties() {
    let temp = tempfile::tempdir().expect("private test directory");

    fs::write(temp.path().join("a.rs"), vec![b'a'; 10]).expect("write file a");
    fs::write(temp.path().join("b.rs"), vec![b'a'; 10]).expect("write file b");
    fs::write(temp.path().join("z.rs"), vec![b'a'; 12]).expect("write file z");

    let result = scan_paths(&[temp.path().to_path_buf()], &ScanConfig::default())
        .expect("scan should succeed");

    let code_top = result
        .top_files_by_category
        .iter()
        .find(|entry| entry.category == Category::Code)
        .expect("code top files should exist");

    assert_eq!(code_top.files[0].bytes, 12);
    assert!(code_top.files[1].path.ends_with("a.rs"));
    assert!(code_top.files[2].path.ends_with("b.rs"));
}

#[test]
fn scanner_emits_progress_deltas_for_processed_bytes() {
    let temp = tempfile::tempdir().expect("private test directory");
    fs::write(temp.path().join("a.rs"), vec![b'a'; 10]).expect("write file");
    fs::write(temp.path().join("b.rs"), vec![b'a'; 20]).expect("write file");
    fs::write(temp.path().join("c.rs"), vec![b'a'; 30]).expect("write file");

    let (tx, rx) = mpsc::channel();
    let result = scan_paths_with_progress(
        &[temp.path().to_path_buf()],
        &ScanConfig::default(),
        Some(tx),
    )
    .expect("scan should succeed");

    let emitted: u64 = rx.try_iter().sum();
    assert_eq!(emitted, result.total_bytes);
}

#[test]
fn default_scan_includes_caller_owned_files_even_without_owner_read_bit() {
    let temp = tempfile::tempdir().expect("private test directory");
    let readable = temp.path().join("readable.rs");
    let unreadable = temp.path().join("unreadable.rs");

    fs::write(&readable, vec![b'a'; 10]).expect("write readable file");
    fs::write(&unreadable, vec![b'a'; 20]).expect("write unreadable file");

    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000))
        .expect("make file unreadable");

    let result = scan_paths(&[temp.path().to_path_buf()], &ScanConfig::default())
        .expect("scan should succeed");

    assert_eq!(result.total_bytes, 30);
    assert_eq!(result.files_scanned, 2);

    let include_all = scan_paths(
        &[temp.path().to_path_buf()],
        &ScanConfig {
            threads: None,
            top_files_per_category: 20,
            show_all_files: true,
            caller_uid: None,
            caller_gids: None,
        },
    )
    .expect("scan should succeed");

    assert_eq!(include_all.total_bytes, 30);
    assert_eq!(include_all.files_scanned, 2);

    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o600))
        .expect("restore permissions for cleanup");
}

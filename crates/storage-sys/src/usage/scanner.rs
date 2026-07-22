// SPDX-License-Identifier: GPL-3.0-only

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::BinaryHeap;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::Instant;

use rayon::ThreadPoolBuilder;
use rayon::prelude::*;

use super::classifier::classify_path;
use super::error::UsageScanError;
use super::types::{
    Category, CategoryTopFiles, CategoryTotal, ScanConfig, ScanResult, TopFileEntry,
};

const PROGRESS_EMIT_BYTES_STEP: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, Eq, PartialEq)]
struct HeapEntry {
    path: PathBuf,
    bytes: u64,
}

impl HeapEntry {
    fn better_than(&self, other: &Self) -> bool {
        self.bytes > other.bytes || (self.bytes == other.bytes && self.path < other.path)
    }
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .bytes
            .cmp(&self.bytes)
            .then_with(|| self.path.cmp(&other.path))
    }
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Default)]
struct LocalStats {
    bytes_by_category: BTreeMap<Category, u64>,
    top_files_by_category: BTreeMap<Category, BinaryHeap<HeapEntry>>,
    total_bytes: u64,
    files_scanned: u64,
    dirs_scanned: u64,
    skipped_errors: u64,
}

#[derive(Debug, Clone)]
struct CallerAccess {
    uid: u32,
    gids: Vec<u32>,
}

impl CallerAccess {
    fn from_config(caller_uid: Option<u32>, caller_gids: Option<&[u32]>) -> Self {
        let process_uid = unsafe { libc::geteuid() };
        let uid = caller_uid.unwrap_or(process_uid);

        if caller_uid.is_some() && uid != process_uid {
            let mut gids = caller_gids.map_or_else(Vec::new, |gids| gids.to_vec());
            gids.sort_unstable();
            gids.dedup();
            return Self { uid, gids };
        }

        let mut gids = vec![unsafe { libc::getegid() }];

        let group_count = unsafe { libc::getgroups(0, std::ptr::null_mut()) };
        if group_count > 0 {
            let mut groups = vec![0 as libc::gid_t; group_count as usize];
            let read_count = unsafe { libc::getgroups(group_count, groups.as_mut_ptr()) };
            if read_count > 0 {
                groups.truncate(read_count as usize);
                gids.extend(groups);
            }
        }

        gids.sort_unstable();
        gids.dedup();

        Self { uid, gids }
    }

    fn can_read(&self, metadata: &fs::Metadata) -> bool {
        let mode = metadata.permissions().mode();
        if metadata.uid() == self.uid {
            return true;
        }

        if self.gids.binary_search(&metadata.gid()).is_ok() {
            return mode & 0o040 != 0;
        }

        mode & 0o004 != 0
    }
}

fn should_include_file(
    metadata: &fs::Metadata,
    caller_access: &CallerAccess,
    show_all_files: bool,
) -> bool {
    show_all_files || caller_access.can_read(metadata)
}

impl LocalStats {
    fn add_file(&mut self, path: &Path, bytes: u64, top_files_per_category: usize) {
        self.total_bytes += bytes;
        self.files_scanned += 1;

        let category = classify_path(path);
        *self.bytes_by_category.entry(category).or_insert(0) += bytes;
        self.consider_top_file(
            category,
            HeapEntry {
                path: path.to_path_buf(),
                bytes,
            },
            top_files_per_category,
        );
    }

    fn consider_top_file(
        &mut self,
        category: Category,
        candidate: HeapEntry,
        top_files_per_category: usize,
    ) {
        if top_files_per_category == 0 {
            return;
        }

        let heap = self.top_files_by_category.entry(category).or_default();

        if heap.len() < top_files_per_category {
            heap.push(candidate);
            return;
        }

        if let Some(worst) = heap.peek()
            && candidate.better_than(worst)
        {
            heap.pop();
            heap.push(candidate);
        }
    }

    fn merge(&mut self, other: LocalStats, top_files_per_category: usize) {
        self.total_bytes += other.total_bytes;
        self.files_scanned += other.files_scanned;
        self.dirs_scanned += other.dirs_scanned;
        self.skipped_errors += other.skipped_errors;

        for (category, bytes) in other.bytes_by_category {
            *self.bytes_by_category.entry(category).or_insert(0) += bytes;
        }

        for (category, files) in other.top_files_by_category {
            for file in files {
                self.consider_top_file(category, file, top_files_per_category);
            }
        }
    }
}

pub fn scan_paths(roots: &[PathBuf], config: &ScanConfig) -> Result<ScanResult, UsageScanError> {
    scan_paths_with_progress(roots, config, None)
}

pub fn scan_paths_with_progress(
    roots: &[PathBuf],
    config: &ScanConfig,
    progress_tx: Option<Sender<u64>>,
) -> Result<ScanResult, UsageScanError> {
    let started = Instant::now();
    let mounts_scanned = roots.len();

    if roots.is_empty() {
        return Ok(ScanResult {
            categories: Vec::new(),
            top_files_by_category: Vec::new(),
            total_bytes: 0,
            total_free_bytes: 0,
            files_scanned: 0,
            dirs_scanned: 0,
            skipped_errors: 0,
            mounts_scanned: 0,
            elapsed_ms: 0,
        });
    }

    let threads = config
        .threads
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(4, usize::from));

    let pool = ThreadPoolBuilder::new()
        .num_threads(threads.max(1))
        .build()
        .map_err(|error| UsageScanError::ThreadPoolBuild(error.to_string()))?;

    let caller_access = CallerAccess::from_config(config.caller_uid, config.caller_gids.as_deref());

    let root_stats: Vec<LocalStats> = pool.install(|| {
        roots
            .par_iter()
            .map(|root| {
                scan_single_root(
                    root.as_path(),
                    config.top_files_per_category,
                    config.show_all_files,
                    caller_access.clone(),
                    progress_tx.clone(),
                )
            })
            .collect()
    });

    let mut combined = LocalStats::default();
    for stats in root_stats {
        combined.merge(stats, config.top_files_per_category);
    }

    let mut categories: Vec<CategoryTotal> = Category::ALL
        .iter()
        .map(|category| CategoryTotal {
            category: *category,
            bytes: *combined.bytes_by_category.get(category).unwrap_or(&0),
        })
        .collect();

    categories.sort_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then_with(|| left.category.cmp(&right.category))
    });

    let top_files_by_category: Vec<CategoryTopFiles> = Category::ALL
        .iter()
        .map(|category| {
            let mut files: Vec<TopFileEntry> = combined
                .top_files_by_category
                .remove(category)
                .unwrap_or_default()
                .into_vec()
                .into_iter()
                .map(|entry| TopFileEntry {
                    path: entry.path,
                    bytes: entry.bytes,
                })
                .collect();

            files.sort_by(|left, right| {
                right
                    .bytes
                    .cmp(&left.bytes)
                    .then_with(|| left.path.cmp(&right.path))
            });

            CategoryTopFiles {
                category: *category,
                files,
            }
        })
        .collect();

    Ok(ScanResult {
        categories,
        top_files_by_category,
        total_bytes: combined.total_bytes,
        total_free_bytes: 0,
        files_scanned: combined.files_scanned,
        dirs_scanned: combined.dirs_scanned,
        skipped_errors: combined.skipped_errors,
        mounts_scanned,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

fn scan_single_root(
    root: &Path,
    top_files_per_category: usize,
    show_all_files: bool,
    caller_access: CallerAccess,
    progress_tx: Option<Sender<u64>>,
) -> LocalStats {
    let mut stats = LocalStats::default();
    let mut pending_progress_bytes = 0_u64;

    let root_metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(_) => {
            stats.skipped_errors += 1;
            return stats;
        }
    };

    if !root_metadata.is_dir() {
        stats.skipped_errors += 1;
        return stats;
    }

    let root_dev = root_metadata.dev();
    let mut stack = vec![root.to_path_buf()];

    while let Some(directory) = stack.pop() {
        stats.dirs_scanned += 1;

        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) => {
                stats.skipped_errors += 1;
                continue;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    stats.skipped_errors += 1;
                    continue;
                }
            };

            let path = entry.path();
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(_) => {
                    stats.skipped_errors += 1;
                    continue;
                }
            };

            if metadata.is_file() {
                if !should_include_file(&metadata, &caller_access, show_all_files) {
                    continue;
                }

                let file_bytes = metadata.len();
                stats.add_file(&path, file_bytes, top_files_per_category);

                pending_progress_bytes = pending_progress_bytes.saturating_add(file_bytes);
                if pending_progress_bytes >= PROGRESS_EMIT_BYTES_STEP {
                    if let Some(tx) = &progress_tx {
                        let _ = tx.send(pending_progress_bytes);
                    }
                    pending_progress_bytes = 0;
                }

                continue;
            }

            if metadata.is_dir() && metadata.dev() == root_dev {
                stack.push(path);
            }
        }
    }

    if pending_progress_bytes > 0
        && let Some(tx) = &progress_tx
    {
        let _ = tx.send(pending_progress_bytes);
    }

    stats
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::mpsc;

    use super::*;

    static COUNTER: AtomicU64 = AtomicU64::new(1);

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("storage-sys-usage-scan-{unique}"));
            fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn aggregates_category_bytes_over_tree() {
        let temp = TempDir::new();
        fs::write(temp.path.join("main.rs"), vec![b'a'; 10]).expect("write rs file");
        fs::write(temp.path.join("pic.png"), vec![b'a'; 20]).expect("write image file");
        fs::write(temp.path.join("note.txt"), vec![b'a'; 30]).expect("write document file");

        let result = scan_paths(std::slice::from_ref(&temp.path), &ScanConfig::default())
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
        let temp = TempDir::new();

        for size in 1_u8..=25 {
            let path = temp.path.join(format!("f{size:02}.rs"));
            fs::write(path, vec![b'a'; size as usize]).expect("write file");
        }

        let result = scan_paths(
            std::slice::from_ref(&temp.path),
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
        let temp = TempDir::new();

        for size in 1_u8..=8 {
            let path = temp.path.join(format!("f{size:02}.rs"));
            fs::write(path, vec![b'a'; size as usize]).expect("write file");
        }

        let result = scan_paths(
            std::slice::from_ref(&temp.path),
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
        let temp = TempDir::new();

        fs::write(temp.path.join("a.rs"), vec![b'a'; 10]).expect("write file a");
        fs::write(temp.path.join("b.rs"), vec![b'a'; 10]).expect("write file b");
        fs::write(temp.path.join("z.rs"), vec![b'a'; 12]).expect("write file z");

        let result = scan_paths(std::slice::from_ref(&temp.path), &ScanConfig::default())
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
        let temp = TempDir::new();
        fs::write(temp.path.join("a.rs"), vec![b'a'; 10]).expect("write file");
        fs::write(temp.path.join("b.rs"), vec![b'a'; 20]).expect("write file");
        fs::write(temp.path.join("c.rs"), vec![b'a'; 30]).expect("write file");

        let (tx, rx) = mpsc::channel();
        let result = scan_paths_with_progress(
            std::slice::from_ref(&temp.path),
            &ScanConfig::default(),
            Some(tx),
        )
        .expect("scan should succeed");

        let emitted: u64 = rx.try_iter().sum();
        assert_eq!(emitted, result.total_bytes);
    }

    #[test]
    fn default_scan_includes_caller_owned_files_even_without_owner_read_bit() {
        let temp = TempDir::new();
        let readable = temp.path.join("readable.rs");
        let unreadable = temp.path.join("unreadable.rs");

        fs::write(&readable, vec![b'a'; 10]).expect("write readable file");
        fs::write(&unreadable, vec![b'a'; 20]).expect("write unreadable file");

        fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000))
            .expect("make file unreadable");

        let result = scan_paths(std::slice::from_ref(&temp.path), &ScanConfig::default())
            .expect("scan should succeed");

        assert_eq!(result.total_bytes, 30);
        assert_eq!(result.files_scanned, 2);

        let include_all = scan_paths(
            std::slice::from_ref(&temp.path),
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
}

// SPDX-License-Identifier: GPL-3.0-only

pub mod classifier;
pub mod error;
pub mod mounts;
pub mod progress;
pub mod scanner;
pub mod types;

pub use classifier::classify_path;
pub use error::UsageScanError;
pub use mounts::{discover_local_mounts_under, estimate_used_bytes_for_mounts};
pub use progress::{compute_progress_percent, format_bytes};
pub use scanner::{scan_paths, scan_paths_with_progress};
pub use types::{Category, CategoryTopFiles, CategoryTotal, ScanConfig, ScanResult, TopFileEntry};

use std::path::Path;
use std::sync::mpsc::Sender;

pub fn scan_local_mounts(root: &Path, config: &ScanConfig) -> Result<ScanResult, UsageScanError> {
    let roots = discover_local_mounts_under(root)?;
    scan_paths(&roots, config)
}

/// Same as `scan_local_mounts`, emitting byte-progress deltas while scanning.
///
/// Honors `ScanConfig::show_all_files` for scanner-side caller visibility filtering.
pub fn scan_local_mounts_with_progress(
    root: &Path,
    config: &ScanConfig,
    progress_tx: Option<Sender<u64>>,
) -> Result<ScanResult, UsageScanError> {
    let roots = discover_local_mounts_under(root)?;
    scan_paths_with_progress(&roots, config, progress_tx)
}

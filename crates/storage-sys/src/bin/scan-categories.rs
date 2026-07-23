// SPDX-License-Identifier: GPL-3.0-only

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;
use storage_sys::usage::{
    ScanConfig, compute_progress_percent, discover_local_mounts_under,
    estimate_used_bytes_for_mounts, format_bytes, scan_paths, scan_paths_with_progress,
};

#[derive(Debug, Parser)]
#[command(name = "scan-categories")]
#[command(about = "Scan file categories and byte totals quickly on Linux")]
struct Args {
    #[arg(long, default_value = "/")]
    root: PathBuf,

    #[arg(long)]
    json: bool,

    #[arg(long)]
    threads: Option<usize>,

    #[arg(long, default_value_t = 20)]
    top_files_per_category: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let config = ScanConfig {
        threads: args.threads,
        top_files_per_category: args.top_files_per_category,
        show_all_files: false,
        caller_uid: None,
        caller_gids: None,
    };

    let roots = if args.root == std::path::Path::new("/") {
        discover_local_mounts_under(&args.root)?
    } else {
        vec![args.root.clone()]
    };

    let progress_enabled = !args.json;
    let denominator_estimate = estimate_used_bytes_for_mounts(&roots);

    let (progress_tx, progress_rx) = mpsc::channel::<u64>();

    let progress_handle = if progress_enabled {
        let total_used_bytes = denominator_estimate.used_bytes;
        Some(thread::spawn(move || {
            let mut bytes_processed = 0_u64;
            let render_interval = Duration::from_millis(250);
            let mut last_render = Instant::now() - render_interval;

            loop {
                match progress_rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(delta) => {
                        bytes_processed = bytes_processed.saturating_add(delta);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }

                if last_render.elapsed() >= render_interval {
                    print_progress_line(bytes_processed, total_used_bytes, false);
                    last_render = Instant::now();
                }
            }

            print_progress_line(bytes_processed, total_used_bytes, true);
            println!();
        }))
    } else {
        None
    };

    let result = if progress_enabled {
        scan_paths_with_progress(&roots, &config, Some(progress_tx))?
    } else {
        scan_paths(&roots, &config)?
    };

    if let Some(handle) = progress_handle {
        let _ = handle.join();
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    println!("CATEGORY       BYTES            PERCENT");
    println!("----------------------------------------");

    for entry in &result.categories {
        let percent = if result.total_bytes == 0 {
            0.0
        } else {
            (entry.bytes as f64 * 100.0) / result.total_bytes as f64
        };

        println!(
            "{:<13} {:>14} {:>9.2}%",
            entry.category.as_str(),
            entry.bytes,
            percent
        );
    }

    println!();
    println!(
        "total_bytes={} files_scanned={} dirs_scanned={} skipped_errors={} mounts_scanned={} elapsed_ms={}",
        result.total_bytes,
        result.files_scanned,
        result.dirs_scanned,
        result.skipped_errors,
        result.mounts_scanned,
        result.elapsed_ms
    );

    println!();
    for category_top in &result.top_files_by_category {
        println!(
            "Top {} largest files - {}",
            config.top_files_per_category,
            category_top.category.as_str()
        );

        if category_top.files.is_empty() {
            println!("  (no files)");
            println!();
            continue;
        }

        for (index, file) in category_top.files.iter().enumerate() {
            println!(
                "  {:>2}. {:>14} {}",
                index + 1,
                file.bytes,
                file.path.display()
            );
        }

        println!();
    }

    Ok(())
}

fn print_progress_line(
    bytes_processed: u64,
    estimated_total_used_bytes: u64,
    force_complete: bool,
) {
    let percent = if force_complete {
        100.0
    } else {
        compute_progress_percent(bytes_processed, estimated_total_used_bytes)
    };

    print!(
        "\rEstimated progress: {:>5.1}% | {} processed",
        percent,
        format_bytes(bytes_processed)
    );

    let _ = std::io::Write::flush(&mut std::io::stdout());
}

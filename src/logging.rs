// SPDX-License-Identifier: GPL-3.0-only

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::{Duration, SystemTime};

use crate::config::{Config, LoggingLevel};
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();
static LOG_LEVEL: AtomicU8 = AtomicU8::new(3);
static LOG_TO_DISK: AtomicBool = AtomicBool::new(true);

const DEFAULT_LOG_PREFIX: &str = "cosmic-ext-storage.log";
const KEEP_DAYS: u64 = 7;

pub(crate) fn init(config: &Config) {
    set_log_level(config.log_level);
    set_log_to_disk(config.log_to_disk);

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        // Default verbosity: keep our crates at INFO, but quiet very chatty GPU logging.
        EnvFilter::new(config.log_level.as_directive())
            .add_directive(
                format!("cosmic_ext_storage={}", config.log_level.as_directive())
                    .parse()
                    .expect("Invalid log directive: cosmic_ext_storage level"),
            )
            .add_directive(
                "wgpu=warn"
                    .parse()
                    .expect("Invalid log directive: wgpu=warn"),
            )
            .add_directive(
                "wgpu_core=warn"
                    .parse()
                    .expect("Invalid log directive: wgpu_core=warn"),
            )
            .add_directive(
                "wgpu_hal=warn"
                    .parse()
                    .expect("Invalid log directive: wgpu_hal=warn"),
            )
            .add_directive(
                "naga=warn"
                    .parse()
                    .expect("Invalid log directive: naga=warn"),
            )
            .add_directive(
                "iced_winit=warn"
                    .parse()
                    .expect("Invalid log directive: iced_winit=warn"),
            )
            .add_directive(
                "iced_wgpu=warn"
                    .parse()
                    .expect("Invalid log directive: iced_wgpu=warn"),
            )
            .add_directive(
                "i18n_embed=warn"
                    .parse()
                    .expect("Invalid log directive: i18n_embed=warn"),
            )
    });

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(true)
        .with_timer(tracing_subscriber::fmt::time::SystemTime)
        .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
            log_level_allows(*metadata.level())
        }));

    match file_writer() {
        Ok((writer, guard)) => {
            let file_layer = tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .with_target(true)
                .with_ansi(false)
                .with_timer(tracing_subscriber::fmt::time::SystemTime)
                .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
                    LOG_TO_DISK.load(Ordering::Relaxed) && log_level_allows(*metadata.level())
                }));

            tracing_subscriber::registry()
                .with(env_filter)
                .with(stdout_layer)
                .with(file_layer)
                .init();

            // Keep the background logging worker alive for the duration of the process.
            let _ = LOG_GUARD.set(guard);
        }
        Err(e) => {
            eprintln!("cosmic-ext-storage: failed to initialize file logging: {e:#}");
            tracing_subscriber::registry()
                .with(env_filter)
                .with(stdout_layer)
                .init();
        }
    }
}

pub(crate) fn set_log_level(level: LoggingLevel) {
    LOG_LEVEL.store(level_to_int(level), Ordering::Relaxed);
}

pub(crate) fn set_log_to_disk(enabled: bool) {
    LOG_TO_DISK.store(enabled, Ordering::Relaxed);
}

fn level_to_int(level: LoggingLevel) -> u8 {
    match level {
        LoggingLevel::Error => 1,
        LoggingLevel::Warn => 2,
        LoggingLevel::Info => 3,
        LoggingLevel::Debug => 4,
        LoggingLevel::Trace => 5,
    }
}

fn severity(level: Level) -> u8 {
    match level {
        Level::ERROR => 1,
        Level::WARN => 2,
        Level::INFO => 3,
        Level::DEBUG => 4,
        Level::TRACE => 5,
    }
}

fn log_level_allows(level: Level) -> bool {
    severity(level) <= LOG_LEVEL.load(Ordering::Relaxed)
}

fn file_writer() -> anyhow::Result<(tracing_appender::non_blocking::NonBlocking, WorkerGuard)> {
    let (dir, prefix) = resolve_log_location();

    if let Err(e) = fs::create_dir_all(&dir) {
        return Err(anyhow::anyhow!(
            "create log directory failed: {} ({})",
            dir.display(),
            e
        ));
    }

    cleanup_old_logs(&dir, &prefix);

    let appender = tracing_appender::rolling::daily(&dir, &prefix);
    let (writer, guard) = tracing_appender::non_blocking(appender);

    Ok((writer, guard))
}

fn resolve_log_location() -> (PathBuf, OsString) {
    if let Some(file) = std::env::var_os("COSMIC_EXT_STORAGE_LOG_FILE") {
        let path = PathBuf::from(file);
        let dir = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(default_log_dir);
        let prefix = path
            .file_name()
            .map(OsString::from)
            .unwrap_or_else(|| OsString::from(DEFAULT_LOG_PREFIX));
        return (dir, prefix);
    }

    if let Some(dir) = std::env::var_os("COSMIC_EXT_STORAGE_LOG_DIR") {
        return (PathBuf::from(dir), OsString::from(DEFAULT_LOG_PREFIX));
    }

    (default_log_dir(), OsString::from(DEFAULT_LOG_PREFIX))
}

fn default_log_dir() -> PathBuf {
    if let Some(xdg_state) = std::env::var_os("XDG_STATE_HOME") {
        return PathBuf::from(xdg_state)
            .join("cosmic-ext-storage")
            .join("logs");
    }

    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("cosmic-ext-storage")
            .join("logs");
    }

    PathBuf::from("/tmp")
        .join("cosmic-ext-storage")
        .join("logs")
}

fn cleanup_old_logs(dir: &Path, prefix: &OsString) {
    let cutoff = SystemTime::now().checked_sub(Duration::from_secs(KEEP_DAYS * 24 * 60 * 60));
    let Some(cutoff) = cutoff else { return };

    let prefix = prefix.to_string_lossy();

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }

        let file_name = entry.file_name();
        // Only touch files created by our rolling appender.
        if !file_name.to_string_lossy().starts_with(prefix.as_ref()) {
            continue;
        }

        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if modified >= cutoff {
            continue;
        }

        let _ = fs::remove_file(entry.path());
    }
}

// SPDX-License-Identifier: GPL-3.0-only

use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};
use serde::{Deserialize, Serialize};
use storage_types::UsageScanParallelismPreset;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Eq, PartialEq)]
pub enum LoggingLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl LoggingLevel {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Error,
            1 => Self::Warn,
            2 => Self::Info,
            3 => Self::Debug,
            4 => Self::Trace,
            _ => Self::Info,
        }
    }

    pub fn to_index(self) -> usize {
        match self {
            Self::Error => 0,
            Self::Warn => 1,
            Self::Info => 2,
            Self::Debug => 3,
            Self::Trace => 4,
        }
    }

    pub fn as_directive(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }
}

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 3]
pub struct Config {
    demo: String,
    pub show_reserved: bool,
    pub usage_scan_parallelism: UsageScanParallelismPreset,
    pub log_to_disk: bool,
    pub log_level: LoggingLevel,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            demo: String::new(),
            show_reserved: false,
            usage_scan_parallelism: UsageScanParallelismPreset::default(),
            log_to_disk: true,
            log_level: LoggingLevel::Info,
        }
    }
}

impl Config {
    pub fn load(app_id: &str) -> Self {
        cosmic_config::Config::new(app_id, Config::VERSION)
            .map(|context| match Self::get_entry(&context) {
                Ok(config) => config,
                Err((_errors, config)) => config,
            })
            .unwrap_or_default()
    }
}

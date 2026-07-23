// SPDX-License-Identifier: GPL-3.0-only

use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum UsageScanError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("invalid mountinfo line: {0}")]
    InvalidMountInfoLine(String),

    #[error("thread pool initialization failed: {0}")]
    ThreadPoolBuild(String),
}

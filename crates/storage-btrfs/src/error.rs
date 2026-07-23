// SPDX-License-Identifier: GPL-3.0-only

use thiserror::Error;

/// Error types for BTRFS operations
#[derive(Error, Debug)]
pub enum BtrfsError {
    #[error("Subvolume not found: {0}")]
    SubvolumeNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Filesystem not mounted: {0}")]
    NotMounted(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("BTRFS operation failed: {0}")]
    OperationFailed(String),

    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type alias for BTRFS operations
pub type Result<T> = std::result::Result<T, BtrfsError>;

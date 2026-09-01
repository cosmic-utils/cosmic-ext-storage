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

    #[error("{0}")]
    OperationFailed(String),

    #[error("{0}")]
    CommandFailed(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type alias for BTRFS operations
pub type Result<T> = std::result::Result<T, BtrfsError>;

#[cfg(test)]
mod tests {
    use super::BtrfsError;

    #[test]
    fn command_error_preserves_the_native_message() {
        assert_eq!(
            BtrfsError::CommandFailed("Operation not permitted".into()).to_string(),
            "Operation not permitted"
        );
    }
}

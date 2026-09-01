// SPDX-License-Identifier: GPL-3.0-only

use storage_contracts::{StorageError, StorageErrorKind};
use thiserror::Error;

/// Sole application-operation error type.  Transport implementation details
/// are intentionally converted at the adapter boundary.
#[derive(Debug, Clone, Error)]
pub enum OperationError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Storage backend unavailable: {0}")]
    Unavailable(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Unsupported operation: {0}")]
    Unsupported(String),
    #[error("Operation not found: {0}")]
    MissingOperation(String),
    #[error("Storage device is busy: {0}")]
    Busy(String),
    #[error("Storage state changed: {0}")]
    Conflict(String),
    #[error("{0}")]
    Other(String),
    #[error("{0}")]
    Failed(String),
}

impl From<StorageError> for OperationError {
    fn from(error: StorageError) -> Self {
        match error.kind {
            StorageErrorKind::InvalidInput => Self::InvalidInput(error.message),
            StorageErrorKind::NotFound => Self::MissingOperation(error.message),
            StorageErrorKind::PermissionDenied => Self::PermissionDenied(error.message),
            StorageErrorKind::Unsupported => Self::Unsupported(error.message),
            StorageErrorKind::Unavailable => Self::Unavailable(error.message),
            StorageErrorKind::Busy => Self::Busy(error.message),
            StorageErrorKind::Conflict => Self::Conflict(error.message),
            StorageErrorKind::Other => Self::Other(error.message),
            _ => Self::Failed(error.message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OperationError;

    #[test]
    fn generic_failures_do_not_add_another_prefix() {
        assert_eq!(
            OperationError::Failed("native denial".into()).to_string(),
            "native denial"
        );
        assert_eq!(
            OperationError::Other("native failure".into()).to_string(),
            "native failure"
        );
    }
}

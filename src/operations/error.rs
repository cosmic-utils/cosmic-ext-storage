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
    #[error("Storage operation failed: {0}")]
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
            _ => Self::Failed(error.message),
        }
    }
}

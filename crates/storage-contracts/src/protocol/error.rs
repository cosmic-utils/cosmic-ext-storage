// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageErrorKind {
    InvalidInput,
    NotFound,
    PermissionDenied,
    Conflict,
    Unsupported,
    Busy,
    Timeout,
    Unavailable,
    Internal,
}

impl StorageErrorKind {
    pub fn code(self) -> u16 {
        match self {
            Self::InvalidInput => 400,
            Self::NotFound => 404,
            Self::PermissionDenied => 403,
            Self::Conflict => 409,
            Self::Unsupported => 501,
            Self::Busy => 423,
            Self::Timeout => 504,
            Self::Unavailable => 503,
            Self::Internal => 500,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[error("{kind:?}: {message}")]
pub struct StorageError {
    pub kind: StorageErrorKind,
    pub message: String,
}

impl StorageError {
    pub fn new(kind: StorageErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_error_roundtrips() {
        let error = StorageError::new(StorageErrorKind::Conflict, "already exists");
        let json = serde_json::to_string(&error).expect("serialize error");
        let parsed: StorageError = serde_json::from_str(&json).expect("deserialize error");
        assert_eq!(parsed, error);
    }
}

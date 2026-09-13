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
    Other,
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
            Self::Other => 520,
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

#[path = "../../tests/unit/protocol/error_tests.rs"]
#[cfg(test)]
mod tests;

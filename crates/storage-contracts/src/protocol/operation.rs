// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

use super::{OperationId, StorageError};

#[cfg(test)]
use super::StorageErrorKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    DiskDiscovery,
    Partitioning,
    Filesystem,
    Encryption,
    Image,
    Lvm,
    Btrfs,
    Rclone,
    UsageScan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationProgress {
    pub operation_id: OperationId,
    pub operation: OperationKind,
    pub phase: String,
    pub bytes_processed: u64,
    pub bytes_total: Option<u64>,
    pub percent: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "payload")]
pub enum OperationEvent {
    Progress(OperationProgress),
    Completed {
        operation_id: OperationId,
        operation: OperationKind,
    },
    Failed {
        operation_id: OperationId,
        operation: OperationKind,
        error: StorageError,
    },
}

#[path = "../../tests/unit/protocol/operation_tests.rs"]
#[cfg(test)]
mod tests;

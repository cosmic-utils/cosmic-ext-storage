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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_id_roundtrips_as_uuid_string() {
        let id = OperationId::new();
        let json = serde_json::to_string(&id).expect("serialize operation id");
        let parsed: OperationId = serde_json::from_str(&json).expect("deserialize operation id");
        assert_eq!(parsed, id);
    }

    #[test]
    fn operation_event_progress_roundtrips() {
        let event = OperationEvent::Progress(OperationProgress {
            operation_id: OperationId::new(),
            operation: OperationKind::UsageScan,
            phase: "enumerating".to_string(),
            bytes_processed: 1024,
            bytes_total: Some(4096),
            percent: Some(25),
        });

        let json = serde_json::to_string(&event).expect("serialize event");
        let parsed: OperationEvent = serde_json::from_str(&json).expect("deserialize event");
        assert_eq!(parsed, event);
    }

    #[test]
    fn storage_error_kind_http_family_codes_are_stable() {
        assert_eq!(StorageErrorKind::InvalidInput.code(), 400);
        assert_eq!(StorageErrorKind::NotFound.code(), 404);
        assert_eq!(StorageErrorKind::PermissionDenied.code(), 403);
        assert_eq!(StorageErrorKind::Conflict.code(), 409);
        assert_eq!(StorageErrorKind::Unsupported.code(), 501);
        assert_eq!(StorageErrorKind::Internal.code(), 500);
    }
}

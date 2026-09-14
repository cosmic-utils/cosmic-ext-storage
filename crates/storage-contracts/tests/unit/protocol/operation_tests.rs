use super::*;

#[test]
fn operation_id_roundtrips_as_validated_string() {
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

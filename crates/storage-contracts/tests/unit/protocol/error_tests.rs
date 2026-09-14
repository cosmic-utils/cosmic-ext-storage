use super::*;

#[test]
fn storage_error_roundtrips() {
    let error = StorageError::new(StorageErrorKind::Conflict, "already exists");
    let json = serde_json::to_string(&error).expect("serialize error");
    let parsed: StorageError = serde_json::from_str(&json).expect("deserialize error");
    assert_eq!(parsed, error);
}

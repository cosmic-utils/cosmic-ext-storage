use storage_contracts::{StorageError, StorageErrorKind};

pub(crate) fn native_error(error: impl std::fmt::Display) -> StorageError {
    let message = error.to_string();
    let lowercase = message.to_ascii_lowercase();
    let kind = if lowercase.contains("notauthorized")
        || lowercase.contains("accessdenied")
        || lowercase.contains("not authorized")
    {
        StorageErrorKind::PermissionDenied
    } else if lowercase.contains("busy") || lowercase.contains("in use") {
        StorageErrorKind::Busy
    } else if lowercase.contains("not found") || lowercase.contains("unknown object") {
        StorageErrorKind::NotFound
    } else if lowercase.contains("not supported") || lowercase.contains("unknown method") {
        StorageErrorKind::Unsupported
    } else if lowercase.contains("invalid") {
        StorageErrorKind::InvalidInput
    } else if lowercase.contains("serviceunknown") || lowercase.contains("no such service") {
        StorageErrorKind::Unavailable
    } else {
        StorageErrorKind::Other
    };
    StorageError::new(kind, message)
}

pub(crate) fn conflict(message: impl Into<String>) -> StorageError {
    StorageError::new(StorageErrorKind::Conflict, message)
}

pub(crate) fn unsupported(message: impl Into<String>) -> StorageError {
    StorageError::new(StorageErrorKind::Unsupported, message)
}

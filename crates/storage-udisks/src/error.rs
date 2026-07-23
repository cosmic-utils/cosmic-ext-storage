//! Error types for storage-udisks operations

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DiskError {
    #[error("The model {0} is not connected")]
    NotConnected(String),

    #[error("Device is busy: {device} at {mount_point}")]
    ResourceBusy { device: String, mount_point: String },

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("D-Bus error: {0}")]
    DBusError(String),

    #[error("Operation failed: {0}")]
    OperationFailed(String),

    #[error("Zbus Error")]
    ZbusError(#[from] zbus::Error),
}

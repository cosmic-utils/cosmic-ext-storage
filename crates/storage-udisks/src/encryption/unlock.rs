// SPDX-License-Identifier: GPL-3.0-only

//! LUKS unlocking operations

use crate::disk::resolve::block_object_path_for_device;
use crate::error::DiskError;
use std::collections::HashMap;
use udisks2::{block::BlockProxy, encrypted::EncryptedProxy};
use zbus::zvariant::Value;

/// Get the cleartext device path for an unlocked LUKS container
///
/// Returns the device path (e.g., "/dev/dm-0") or empty string if not unlocked
pub async fn get_cleartext_device(device_path: &str) -> Result<String, DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let encrypted_path = block_object_path_for_device(device_path).await?;

    let encrypted_proxy = EncryptedProxy::builder(&connection)
        .path(&encrypted_path)?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let cleartext_object = encrypted_proxy
        .cleartext_device()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    // If no cleartext device, return empty string
    if cleartext_object.as_str() == "/" {
        return Ok(String::new());
    }

    // Get the device path from the cleartext block device
    let block_proxy = BlockProxy::builder(&connection)
        .path(&cleartext_object)?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let device_bytes = block_proxy
        .preferred_device()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let device = String::from_utf8(device_bytes.into_iter().filter(|&b| b != 0).collect())
        .unwrap_or_default();

    Ok(device)
}

/// Unlock a LUKS container
///
/// Returns the cleartext device path (e.g., "/dev/mapper/luks-...")
pub async fn unlock_luks(device_path: &str, passphrase: &str) -> Result<String, DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let encrypted_path = block_object_path_for_device(device_path).await?;

    let encrypted_proxy = EncryptedProxy::builder(&connection)
        .path(&encrypted_path)?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let opts: HashMap<&str, Value<'_>> = HashMap::new();
    let cleartext_path = encrypted_proxy
        .unlock(passphrase, opts)
        .await
        .map_err(|e| DiskError::OperationFailed(format!("Unlock failed: {}", e)))?;

    // Get the device path from the cleartext object
    let block_proxy = BlockProxy::builder(&connection)
        .path(&cleartext_path)?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let device_bytes = block_proxy
        .preferred_device()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let device = String::from_utf8(device_bytes.into_iter().filter(|&b| b != 0).collect())
        .unwrap_or_else(|_| cleartext_path.to_string());

    Ok(device)
}

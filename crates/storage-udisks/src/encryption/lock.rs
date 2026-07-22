// SPDX-License-Identifier: GPL-3.0-only

//! LUKS locking operations

use crate::error::DiskError;
use std::collections::HashMap;
use udisks2::encrypted::EncryptedProxy;
use zbus::zvariant::Value;

/// Lock a LUKS container
pub async fn lock_luks(device_path: &str) -> Result<(), DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let encrypted_path = crate::disk::resolve::block_object_path_for_device(device_path).await?;

    let encrypted_proxy = EncryptedProxy::builder(&connection)
        .path(&encrypted_path)?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let opts: HashMap<&str, Value<'_>> = HashMap::new();
    encrypted_proxy
        .lock(opts)
        .await
        .map_err(|e| DiskError::OperationFailed(format!("Lock failed: {}", e)))?;

    Ok(())
}

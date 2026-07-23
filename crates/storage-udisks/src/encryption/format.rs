// SPDX-License-Identifier: GPL-3.0-only

//! LUKS encryption / formatting operations

use crate::error::DiskError;
use std::collections::HashMap;
use udisks2::block::BlockProxy;
use zbus::zvariant::Value;

/// Format a device as LUKS encrypted container
pub async fn format_luks(
    device_path: &str,
    passphrase: &str,
    version: &str,
) -> Result<(), DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let block_path = crate::disk::resolve::block_object_path_for_device(device_path).await?;

    let block_proxy = BlockProxy::builder(&connection)
        .path(&block_path)?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    // UDisks2 requires "empty" as the format type with encrypt.passphrase option
    // The LUKS version is specified via encrypt.type option
    let luks_version = if version == "luks1" {
        "luks1"
    } else {
        "luks2" // Default to luks2
    };

    let mut options: HashMap<&str, Value<'_>> = HashMap::new();
    options.insert("encrypt.passphrase", Value::from(passphrase));
    options.insert("encrypt.type", Value::from(luks_version));

    block_proxy
        .format("empty", options)
        .await
        .map_err(|e| DiskError::OperationFailed(format!("Format failed: {}", e)))?;

    Ok(())
}

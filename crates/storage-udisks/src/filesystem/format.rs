// SPDX-License-Identifier: GPL-3.0-only

//! Filesystem formatting operations

use crate::error::DiskError;
use std::collections::HashMap;
use storage_types::FormatOptions;
use udisks2::block::BlockProxy;
use zbus::zvariant::Value;

/// Format a filesystem
pub async fn format_filesystem(
    device_path: &str,
    fs_type: &str,
    label: &str,
    options: FormatOptions,
) -> Result<(), DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let block_path = crate::disk::resolve::block_object_path_for_device(device_path).await?;

    // Format using Block.Format
    let block_proxy = BlockProxy::builder(&connection)
        .path(&block_path)
        .map_err(|e| DiskError::DBusError(e.to_string()))?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let mut format_opts: HashMap<&str, Value<'_>> = HashMap::new();

    // Use label from options if provided, otherwise use the label parameter
    let final_label = if options.label.is_empty() {
        label
    } else {
        &options.label
    };
    if !final_label.is_empty() {
        format_opts.insert("label", Value::from(final_label));
    }

    // Enable secure erase if requested
    if options.erase {
        format_opts.insert("erase", Value::from(true));
    }

    // Force formatting if requested
    if options.force {
        format_opts.insert("force", Value::from(true));
    }

    // Enable discard/TRIM if requested
    if options.discard {
        format_opts.insert("discard", Value::from(true));
    }

    block_proxy
        .format(fs_type, format_opts)
        .await
        .map_err(|e| DiskError::OperationFailed(format!("Format failed: {}", e)))?;

    Ok(())
}

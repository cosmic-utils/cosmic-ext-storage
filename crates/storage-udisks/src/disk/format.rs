// SPDX-License-Identifier: GPL-3.0-only

//! Disk formatting operations

use anyhow::Result;
use std::collections::HashMap;
use udisks2::block::BlockProxy;
use zbus::zvariant::Value;

/// Format the entire disk (drive block device) via UDisks2.
///
/// `format_type` is passed directly to `org.freedesktop.UDisks2.Block.Format`, and may be
/// values like `"gpt"`, `"dos"`, or `"empty"` (depending on UDisks support).
///
/// If `erase` is true, request a zero-fill erase (slow) via the `erase=zero` option.
///
/// Note: Caller should ensure no mounted filesystems exist before calling this.
pub async fn format_disk(block_path: String, format_type: &str, erase: bool) -> Result<()> {
    let connection = crate::manager::shared_connection().await?;
    format_disk_with_connection(connection.as_ref(), block_path, format_type, erase).await
}

/// Format a disk through the caller-selected UDisks transport.
pub(crate) async fn format_disk_with_connection(
    connection: &zbus::Connection,
    block_path: String,
    format_type: &str,
    erase: bool,
) -> Result<()> {
    let block_proxy = BlockProxy::builder(connection)
        .path(block_path)?
        .build()
        .await?;

    let mut format_options: HashMap<&str, Value<'_>> = HashMap::new();

    if erase {
        format_options.insert("erase", Value::from("zero"));
    }

    block_proxy.format(format_type, format_options).await?;
    Ok(())
}

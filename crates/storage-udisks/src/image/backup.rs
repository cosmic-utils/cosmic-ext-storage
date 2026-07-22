// SPDX-License-Identifier: GPL-3.0-only

//! Backup and restore operations

use anyhow::Result;
use std::collections::HashMap;
use std::os::fd::OwnedFd;
use zbus::zvariant::{OwnedFd as ZOwnedFd, OwnedObjectPath, Value};

use crate::image::udisks_call::call_udisks_raw;

/// Open a block device for backup (read-only access)
pub async fn open_for_backup(block_object_path: OwnedObjectPath) -> Result<OwnedFd> {
    let connection = crate::manager::shared_connection().await?;
    let options_empty: HashMap<&str, Value<'_>> = HashMap::new();

    let fd: ZOwnedFd = call_udisks_raw(
        &connection,
        &block_object_path,
        "org.freedesktop.UDisks2.Block",
        "OpenForBackup",
        &(options_empty),
    )
    .await?;

    Ok(fd.into())
}

/// Open a block device for restore (read-write access)
pub async fn open_for_restore(block_object_path: OwnedObjectPath) -> Result<OwnedFd> {
    let connection = crate::manager::shared_connection().await?;
    let options_empty: HashMap<&str, Value<'_>> = HashMap::new();

    let fd: ZOwnedFd = call_udisks_raw(
        &connection,
        &block_object_path,
        "org.freedesktop.UDisks2.Block",
        "OpenForRestore",
        &(options_empty),
    )
    .await?;

    Ok(fd.into())
}

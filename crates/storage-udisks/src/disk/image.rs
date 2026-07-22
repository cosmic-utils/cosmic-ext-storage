// SPDX-License-Identifier: GPL-3.0-only

//! Disk image operations (backup/restore file descriptor access)

use anyhow::Result;
use zbus::zvariant::OwnedObjectPath;

/// Open a drive for backup (read-only access to block device)
pub async fn open_for_backup(block_path: OwnedObjectPath) -> Result<std::os::fd::OwnedFd> {
    crate::image::open_for_backup(block_path).await
}

/// Open a drive for restore (read-write access to block device)
pub async fn open_for_restore(block_path: OwnedObjectPath) -> Result<std::os::fd::OwnedFd> {
    crate::image::open_for_restore(block_path).await
}

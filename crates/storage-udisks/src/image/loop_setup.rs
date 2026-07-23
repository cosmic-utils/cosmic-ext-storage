// SPDX-License-Identifier: GPL-3.0-only

//! Loop device setup operations

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::os::fd::OwnedFd;
use std::os::unix::fs::FileTypeExt;
use std::path::PathBuf;
use zbus::zvariant::{OwnedFd as ZOwnedFd, OwnedObjectPath, Value};

use crate::image::udisks_call::call_udisks_raw;

fn file_type_for_display(file_type: &std::fs::FileType) -> &'static str {
    if file_type.is_file() {
        "regular file"
    } else if file_type.is_dir() {
        "directory"
    } else if file_type.is_symlink() {
        "symlink"
    } else if file_type.is_block_device() {
        "block device"
    } else if file_type.is_char_device() {
        "character device"
    } else if file_type.is_fifo() {
        "fifo"
    } else if file_type.is_socket() {
        "socket"
    } else {
        "unknown file type"
    }
}

async fn open_image_readonly_fd(image_path: &str) -> Result<OwnedFd> {
    let path: PathBuf = image_path.into();

    tokio::task::spawn_blocking(move || -> Result<OwnedFd> {
        let metadata = std::fs::metadata(&path)
            .with_context(|| format!("Failed to stat image path {}", path.display()))?;

        let file_type = metadata.file_type();
        if !file_type.is_file() {
            anyhow::bail!(
                "Image path {} is a {}; expected a regular file",
                path.display(),
                file_type_for_display(&file_type)
            );
        }

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(false)
            .open(&path)
            .with_context(|| format!("Failed to open image file {}", path.display()))?;

        Ok(file.into())
    })
    .await
    .context("Image file open task panicked or was cancelled")?
}

/// Set up a loop device for an image file
pub async fn loop_setup(image_path: &str) -> Result<OwnedObjectPath> {
    let connection = crate::manager::shared_connection().await?;

    let manager_path: OwnedObjectPath = "/org/freedesktop/UDisks2/Manager".try_into()?;

    // UDisks2 expects a Unix FD handle for LoopSetup: (h a{sv}).
    // Passing a path string will fail with InvalidArgs.
    // Opening the file can block on slow/remote filesystems, so offload it.
    let fd: OwnedFd = open_image_readonly_fd(image_path).await?;
    let fd: ZOwnedFd = fd.into();

    // Attach is used for mounting images (e.g. ISO); default to read-only.
    let mut options: HashMap<&str, Value<'_>> = HashMap::new();
    options.insert("read-only", Value::from(true));

    call_udisks_raw(
        &connection,
        &manager_path,
        "org.freedesktop.UDisks2.Manager",
        "LoopSetup",
        &(fd, options),
    )
    .await
}

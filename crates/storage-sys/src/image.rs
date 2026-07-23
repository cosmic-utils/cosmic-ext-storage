// SPDX-License-Identifier: GPL-3.0-only

//! Disk image operations using direct file I/O
//!
//! These functions handle byte-level copying between block devices and image files.

use crate::error::{Result, SysError};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::path::Path;

/// Open a block device for reading (backup)
///
/// Returns an owned file descriptor that can be used for reading raw bytes.
pub fn open_for_backup(device: &str) -> Result<OwnedFd> {
    let file = OpenOptions::new().read(true).open(device).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            SysError::PermissionDenied(format!("Cannot open {} for reading", device))
        } else if e.kind() == std::io::ErrorKind::NotFound {
            SysError::DeviceNotFound(device.to_string())
        } else {
            SysError::Io(e)
        }
    })?;

    Ok(unsafe { OwnedFd::from_raw_fd(file.as_raw_fd()) })
}

/// Open a block device for writing (restore)
///
/// Returns an owned file descriptor that can be used for writing raw bytes.
pub fn open_for_restore(device: &str) -> Result<OwnedFd> {
    let file = OpenOptions::new().write(true).open(device).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            SysError::PermissionDenied(format!("Cannot open {} for writing", device))
        } else if e.kind() == std::io::ErrorKind::NotFound {
            SysError::DeviceNotFound(device.to_string())
        } else {
            SysError::Io(e)
        }
    })?;

    Ok(unsafe { OwnedFd::from_raw_fd(file.as_raw_fd()) })
}

/// Copy from a file descriptor to a file path
///
/// # Arguments
/// * `source_fd` - File descriptor to read from (typically a block device)
/// * `dest_path` - Destination file path
/// * `progress_callback` - Optional callback for progress updates (bytes copied)
pub fn copy_image_to_file<F>(
    source_fd: OwnedFd,
    dest_path: &Path,
    mut progress_callback: Option<F>,
) -> Result<u64>
where
    F: FnMut(u64),
{
    let mut source = File::from(source_fd);
    let mut dest = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dest_path)?;

    let mut buffer = vec![0u8; 1024 * 1024]; // 1MB buffer
    let mut total_copied: u64 = 0;

    loop {
        let bytes_read = source.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        dest.write_all(&buffer[..bytes_read])?;
        total_copied += bytes_read as u64;

        if let Some(ref mut callback) = progress_callback {
            callback(total_copied);
        }
    }

    dest.sync_all()?;
    Ok(total_copied)
}

/// Copy from a file path to a file descriptor
///
/// # Arguments
/// * `source_path` - Source file path
/// * `dest_fd` - File descriptor to write to (typically a block device)
/// * `progress_callback` - Optional callback for progress updates (bytes copied)
pub fn copy_file_to_image<F>(
    source_path: &Path,
    dest_fd: OwnedFd,
    mut progress_callback: Option<F>,
) -> Result<u64>
where
    F: FnMut(u64),
{
    let mut source = File::open(source_path)?;
    let mut dest = File::from(dest_fd);

    let mut buffer = vec![0u8; 1024 * 1024]; // 1MB buffer
    let mut total_copied: u64 = 0;

    loop {
        let bytes_read = source.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        dest.write_all(&buffer[..bytes_read])?;
        total_copied += bytes_read as u64;

        if let Some(ref mut callback) = progress_callback {
            callback(total_copied);
        }
    }

    dest.sync_all()?;
    Ok(total_copied)
}

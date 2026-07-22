// SPDX-License-Identifier: GPL-3.0-only

use crate::error::{BtrfsError, Result};
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::path::Path;
use storage_types::btrfs::FilesystemUsage;

/// Get filesystem usage information using statvfs
pub fn get_filesystem_usage(mount_point: &Path) -> Result<FilesystemUsage> {
    // Convert path to CString
    let c_path = CString::new(mount_point.to_string_lossy().as_bytes())
        .map_err(|e| BtrfsError::InvalidPath(format!("Invalid mount point path: {}", e)))?;

    // Call statvfs
    let mut stat: MaybeUninit<libc::statvfs> = MaybeUninit::uninit();
    let result = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };

    if result != 0 {
        let err = std::io::Error::last_os_error();
        return Err(BtrfsError::Io(err));
    }

    let stat = unsafe { stat.assume_init() };

    // Calculate used space
    // f_blocks = total blocks, f_bfree = free blocks
    // f_frsize = fragment size (preferred for calculations)
    let total_bytes = stat.f_blocks * stat.f_frsize;
    let free_bytes = stat.f_bfree * stat.f_frsize;
    let used_bytes = total_bytes - free_bytes;

    Ok(FilesystemUsage { used_bytes })
}

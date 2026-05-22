use anyhow::{Context, Result};
use std::{ffi::CString, mem::MaybeUninit};

// Re-export Usage from storage-types
pub use storage_types::Usage;

pub fn usage_for_mount_point(mount_point: &str, filesystem: Option<&str>) -> Result<Usage> {
    let mount_point_c = CString::new(mount_point)
        .with_context(|| format!("mount point contains NUL byte: {mount_point:?}"))?;

    let mut stat = MaybeUninit::<libc::statvfs>::uninit();
    let rc = unsafe { libc::statvfs(mount_point_c.as_ptr(), stat.as_mut_ptr()) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("statvfs failed for mount point {mount_point:?}"));
    }

    let stat = unsafe { stat.assume_init() };
    let frsize = if stat.f_frsize > 0 {
        stat.f_frsize
    } else {
        stat.f_bsize
    };

    let total = stat.f_blocks.saturating_mul(frsize);
    let free = stat.f_bfree.saturating_mul(frsize);
    let available = stat.f_bavail.saturating_mul(frsize);
    let used = total.saturating_sub(free);
    let percent = used
        .saturating_mul(100)
        .checked_div(total)
        .unwrap_or(0)
        .min(100) as u32;

    Ok(Usage {
        filesystem: filesystem.unwrap_or_default().to_string(),
        blocks: total,
        used,
        available,
        percent,
        mount_point: mount_point.to_string(),
    })
}

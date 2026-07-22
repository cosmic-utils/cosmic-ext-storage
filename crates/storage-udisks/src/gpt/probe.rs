use std::{collections::HashMap, fs, io, path::Path};

use anyhow::{Context, Result};
use storage_types::ByteRange;
use tracing::{debug, warn};
use udisks2::block::BlockProxy;
use zbus::zvariant::Value;

pub const GPT_ALIGNMENT_BYTES: u64 = 1024 * 1024;

fn parse_c_string_bytes(bytes: &[u8]) -> String {
    let nul_pos = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..nul_pos]).to_string()
}

fn sysfs_logical_block_size(devnode: &str) -> io::Result<u64> {
    let dev_name = Path::new(devnode)
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing dev basename"))?;

    let path = Path::new("/sys/class/block")
        .join(dev_name)
        .join("queue/logical_block_size");

    let raw = fs::read_to_string(path)?;
    let value = raw
        .trim()
        .parse::<u64>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(value)
}

fn ioctl_logical_block_size(file: &std::fs::File) -> io::Result<u64> {
    // linux/fs.h: BLKSSZGET = _IO(0x12, 104)
    const BLKSSZGET: libc::c_ulong = 0x1268;
    let mut size: libc::c_int = 0;

    use std::os::fd::AsRawFd;
    let ret = unsafe { libc::ioctl(file.as_raw_fd(), BLKSSZGET, &mut size) };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }
    u64::try_from(size).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("negative logical block size returned by ioctl: {}", size),
        )
    })
}

fn gpt_parse_first_last_usable_lba(sector: &[u8]) -> Option<(u64, u64)> {
    // GPT header is at LBA 1 and starts with signature "EFI PART".
    if sector.len() < 92 {
        return None;
    }

    if &sector[0..8] != b"EFI PART" {
        return None;
    }

    // Header size is LE u32 at offset 12. We only sanity-check it.
    let header_size = u32::from_le_bytes(sector[12..16].try_into().ok()?);
    if header_size < 92 || header_size as usize > sector.len() {
        return None;
    }

    let first_usable = u64::from_le_bytes(sector[40..48].try_into().ok()?);
    let last_usable = u64::from_le_bytes(sector[48..56].try_into().ok()?);

    if first_usable == 0 || last_usable == 0 || first_usable >= last_usable {
        return None;
    }

    Some((first_usable, last_usable))
}

/// Probe a GPT disk for its usable byte range.
///
/// Uses UDisks `OpenDevice("r")` for the actual read (authorization-friendly).
/// Sector size is sourced from ioctl `BLKSSZGET` (fallback sysfs `queue/logical_block_size`).
///
/// Returns `Ok(None)` if GPT cannot be parsed.
pub async fn probe_gpt_usable_range_bytes(
    block: &BlockProxy<'_>,
    disk_size: u64,
) -> Result<Option<ByteRange>> {
    if disk_size == 0 {
        return Ok(None);
    }

    // Determine devnode for sysfs fallback.
    let devnode = parse_c_string_bytes(&block.preferred_device().await?);

    // Open a read-only fd via UDisks.
    let mut options: HashMap<&str, Value<'_>> = HashMap::new();
    // Don't trigger a polkit prompt just to compute UI segmentation.
    // If authorization is required, the call will fail and we'll fall back.
    options.insert("auth.no_user_interaction", Value::from(true));
    let owned_fd = match block.open_device("r", options).await {
        Ok(fd) => fd,
        Err(e) => {
            debug!(
                "failed to open device for GPT probe (no-user-interaction): {e}; devnode={devnode}"
            );
            return Ok(None);
        }
    };

    // Convert to a File we can pread from.
    let fd: std::os::fd::OwnedFd = owned_fd.into();
    let file: std::fs::File = fd.into();

    let sector_size = match ioctl_logical_block_size(&file) {
        Ok(v) if v >= 512 => v,
        Ok(v) => {
            warn!("suspicious logical block size from ioctl: {v}; devnode={devnode}");
            return Ok(None);
        }
        Err(ioctl_err) => match sysfs_logical_block_size(&devnode) {
            Ok(v) if v >= 512 => v,
            Ok(v) => {
                warn!("suspicious logical_block_size from sysfs: {v}; devnode={devnode}");
                return Ok(None);
            }
            Err(sysfs_err) => {
                warn!(
                    "failed to determine logical block size; ioctl={ioctl_err}; sysfs={sysfs_err}; devnode={devnode}"
                );
                return Ok(None);
            }
        },
    };

    // Explicitly check that sector_size is valid before proceeding.
    if sector_size == 0 {
        warn!("logical block size is zero; devnode={devnode}");
        return Ok(None);
    }

    // Read LBA 1.
    let sector_size_usize = usize::try_from(sector_size).context("sector size too large")?;
    if sector_size_usize > 64 * 1024 {
        warn!("logical sector size unusually large: {sector_size} bytes; refusing to read");
        return Ok(None);
    }

    let mut buf = vec![0u8; sector_size_usize];
    use std::os::unix::fs::FileExt;
    file.read_exact_at(&mut buf, sector_size)
        .context("read GPT header sector")?;

    let Some((first_lba, last_lba)) = gpt_parse_first_last_usable_lba(&buf) else {
        return Ok(None);
    };

    // Convert to a half-open byte range.
    let start = first_lba.saturating_mul(sector_size);
    let end = last_lba
        .saturating_add(1)
        .saturating_mul(sector_size)
        .min(disk_size);

    let range = ByteRange { start, end }.clamp_to_disk(disk_size);
    if !range.is_valid_for_disk(disk_size) {
        return Ok(None);
    }

    Ok(Some(range))
}

/// Conservative fallback usable range when GPT parsing fails.
///
/// Reserves 1 MiB at the start and end of disk (clamped), matching typical tooling behavior.
pub fn fallback_gpt_usable_range_bytes(disk_size: u64) -> Option<ByteRange> {
    if disk_size <= 2 * GPT_ALIGNMENT_BYTES {
        return None;
    }

    Some(ByteRange {
        start: GPT_ALIGNMENT_BYTES,
        end: disk_size.saturating_sub(GPT_ALIGNMENT_BYTES),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_first_last_usable_lba() {
        let mut sector = vec![0u8; 512];
        sector[0..8].copy_from_slice(b"EFI PART");
        sector[12..16].copy_from_slice(&(92u32.to_le_bytes()));
        sector[40..48].copy_from_slice(&(34u64.to_le_bytes()));
        sector[48..56].copy_from_slice(&(1000u64.to_le_bytes()));

        let res = gpt_parse_first_last_usable_lba(&sector).unwrap();
        assert_eq!(res, (34, 1000));
    }

    #[test]
    fn rejects_non_gpt_signature() {
        let sector = vec![0u8; 512];
        assert!(gpt_parse_first_last_usable_lba(&sector).is_none());
    }

    #[test]
    fn fallback_returns_none_for_small_disks() {
        // Disk exactly at threshold (2 MiB)
        assert!(fallback_gpt_usable_range_bytes(2 * GPT_ALIGNMENT_BYTES).is_none());
        // Disk smaller than threshold
        assert!(fallback_gpt_usable_range_bytes(1024 * 1024).is_none());
        assert!(fallback_gpt_usable_range_bytes(0).is_none());
    }

    #[test]
    fn fallback_returns_range_for_larger_disks() {
        // 10 MiB disk should return range with 1 MiB reserved at each end
        let disk_size = 10 * 1024 * 1024;
        let range = fallback_gpt_usable_range_bytes(disk_size).unwrap();
        assert_eq!(range.start, GPT_ALIGNMENT_BYTES);
        assert_eq!(range.end, disk_size - GPT_ALIGNMENT_BYTES);

        // 100 GiB disk
        let disk_size = 100 * 1024 * 1024 * 1024;
        let range = fallback_gpt_usable_range_bytes(disk_size).unwrap();
        assert_eq!(range.start, GPT_ALIGNMENT_BYTES);
        assert_eq!(range.end, disk_size - GPT_ALIGNMENT_BYTES);
    }

    #[test]
    fn rejects_gpt_header_size_larger_than_sector() {
        let mut sector = vec![0u8; 512];
        sector[0..8].copy_from_slice(b"EFI PART");
        // Set header_size to 1024 (larger than our 512-byte sector)
        sector[12..16].copy_from_slice(&(1024u32.to_le_bytes()));
        sector[40..48].copy_from_slice(&(34u64.to_le_bytes()));
        sector[48..56].copy_from_slice(&(1000u64.to_le_bytes()));

        assert!(gpt_parse_first_last_usable_lba(&sector).is_none());
    }

    #[test]
    fn rejects_first_usable_equal_to_last_usable() {
        let mut sector = vec![0u8; 512];
        sector[0..8].copy_from_slice(b"EFI PART");
        sector[12..16].copy_from_slice(&(92u32.to_le_bytes()));
        sector[40..48].copy_from_slice(&(100u64.to_le_bytes()));
        sector[48..56].copy_from_slice(&(100u64.to_le_bytes()));

        assert!(gpt_parse_first_last_usable_lba(&sector).is_none());
    }

    #[test]
    fn rejects_first_usable_greater_than_last_usable() {
        let mut sector = vec![0u8; 512];
        sector[0..8].copy_from_slice(b"EFI PART");
        sector[12..16].copy_from_slice(&(92u32.to_le_bytes()));
        sector[40..48].copy_from_slice(&(1000u64.to_le_bytes()));
        sector[48..56].copy_from_slice(&(34u64.to_le_bytes()));

        assert!(gpt_parse_first_last_usable_lba(&sector).is_none());
    }
}

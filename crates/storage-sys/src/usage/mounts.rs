// SPDX-License-Identifier: GPL-3.0-only

use std::collections::BTreeSet;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs;
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use super::error::UsageScanError;

const EXCLUDED_FS_TYPES: &[&str] = &[
    "autofs",
    "binfmt_misc",
    "bpf",
    "cgroup",
    "cgroup2",
    "configfs",
    "debugfs",
    "devpts",
    "devtmpfs",
    "efivarfs",
    "fusectl",
    "hugetlbfs",
    "mqueue",
    "nfs",
    "nfs4",
    "nsfs",
    "overlay",
    "proc",
    "pstore",
    "ramfs",
    "rpc_pipefs",
    "securityfs",
    "selinuxfs",
    "smb3",
    "smbfs",
    "squashfs",
    "sysfs",
    "tmpfs",
    "tracefs",
    "fuse.sshfs",
    "cifs",
    "9p",
    "ceph",
    "glusterfs",
    "lustre",
    "sshfs",
];

#[derive(Debug, Clone, Copy)]
pub struct MountUsageEstimate {
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub mounts_counted: usize,
    pub mounts_failed: usize,
}

#[derive(Debug, Clone)]
struct MountInfoEntry {
    path: PathBuf,
    device_id: String,
}

pub fn discover_local_mounts_under(root: &Path) -> Result<Vec<PathBuf>, UsageScanError> {
    let mount_info = fs::read_to_string("/proc/self/mountinfo")?;
    let mut mount_points = parse_local_mount_entries(&mount_info)?
        .into_iter()
        .map(|entry| entry.path)
        .collect::<Vec<_>>();

    if root == Path::new("/") {
        return Ok(mount_points);
    }

    let root = root.to_path_buf();
    mount_points.retain(|mount| mount.starts_with(&root));
    if mount_points.is_empty() {
        mount_points.push(root);
    }

    Ok(mount_points)
}

pub fn parse_local_mounts(input: &str) -> Result<Vec<PathBuf>, UsageScanError> {
    Ok(parse_local_mount_entries(input)?
        .into_iter()
        .map(|entry| entry.path)
        .collect())
}

fn parse_local_mount_entries(input: &str) -> Result<Vec<MountInfoEntry>, UsageScanError> {
    let mut roots = BTreeSet::new();
    let mut entries = Vec::new();

    for line in input.lines().filter(|line| !line.trim().is_empty()) {
        let (left, right) = line
            .split_once(" - ")
            .ok_or_else(|| UsageScanError::InvalidMountInfoLine(line.to_string()))?;

        let mut left_fields = left.split_whitespace();
        let _mount_id = left_fields.next();
        let _parent_id = left_fields.next();
        let device_id = left_fields
            .next()
            .ok_or_else(|| UsageScanError::InvalidMountInfoLine(line.to_string()))?;
        let mount_point = left_fields
            .nth(1)
            .ok_or_else(|| UsageScanError::InvalidMountInfoLine(line.to_string()))?;

        let fs_type = right
            .split_whitespace()
            .next()
            .ok_or_else(|| UsageScanError::InvalidMountInfoLine(line.to_string()))?;

        if is_non_local_fs_type(fs_type) {
            continue;
        }

        let path = PathBuf::from(unescape_mount_field(mount_point));
        if roots.insert(path.clone()) {
            entries.push(MountInfoEntry {
                path,
                device_id: device_id.to_string(),
            });
        }
    }

    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

pub fn estimate_used_bytes_for_mounts(mounts: &[PathBuf]) -> MountUsageEstimate {
    let mount_info_entries = fs::read_to_string("/proc/self/mountinfo")
        .ok()
        .and_then(|contents| parse_local_mount_entries(&contents).ok())
        .unwrap_or_default();

    let mount_to_device: HashMap<PathBuf, String> = mount_info_entries
        .into_iter()
        .map(|entry| (entry.path, entry.device_id))
        .collect();

    let mut seen_devices = BTreeSet::new();
    let mut estimate = MountUsageEstimate {
        used_bytes: 0,
        free_bytes: 0,
        mounts_counted: 0,
        mounts_failed: 0,
    };

    for mount in mounts {
        let dedupe_key = if let Some(device_id) = mount_to_device.get(mount) {
            format!("mountinfo:{device_id}")
        } else {
            format!("path:{}", mount.display())
        };

        if !seen_devices.insert(dedupe_key) {
            continue;
        }

        match used_and_free_bytes_for_mount(mount) {
            Ok((used, free)) => {
                estimate.used_bytes = estimate.used_bytes.saturating_add(used);
                estimate.free_bytes = estimate.free_bytes.saturating_add(free);
                estimate.mounts_counted += 1;
            }
            Err(_) => {
                estimate.mounts_failed += 1;
            }
        }
    }

    estimate
}

fn used_and_free_bytes_for_mount(mount: &Path) -> Result<(u64, u64), UsageScanError> {
    let mount_bytes = mount.as_os_str().as_bytes();
    let mount_cstr = CString::new(mount_bytes)
        .map_err(|_| UsageScanError::InvalidMountInfoLine("mount path contains NUL byte".into()))?;

    let mut stat = MaybeUninit::<libc::statvfs>::uninit();
    let result = unsafe { libc::statvfs(mount_cstr.as_ptr(), stat.as_mut_ptr()) };
    if result != 0 {
        return Err(UsageScanError::Io(std::io::Error::last_os_error()));
    }

    let stat = unsafe { stat.assume_init() };
    Ok((
        used_bytes_from_fields(stat.f_blocks, stat.f_bfree, stat.f_frsize),
        free_bytes_from_fields(stat.f_bavail, stat.f_frsize),
    ))
}

fn used_bytes_from_fields(blocks: u64, bfree: u64, frsize: u64) -> u64 {
    blocks.saturating_sub(bfree).saturating_mul(frsize)
}

fn free_bytes_from_fields(bavail: u64, frsize: u64) -> u64 {
    bavail.saturating_mul(frsize)
}

fn is_non_local_fs_type(fs_type: &str) -> bool {
    if EXCLUDED_FS_TYPES.contains(&fs_type) {
        return true;
    }

    fs_type.starts_with("fuse.") || fs_type.starts_with("nfs")
}

fn unescape_mount_field(value: &str) -> String {
    let mut output = Vec::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'\\'
            && index + 3 < bytes.len()
            && bytes[index + 1].is_ascii_digit()
            && bytes[index + 2].is_ascii_digit()
            && bytes[index + 3].is_ascii_digit()
        {
            let octal = &value[index + 1..index + 4];
            if let Ok(num) = u8::from_str_radix(octal, 8) {
                output.push(num);
                index += 4;
                continue;
            }
        }

        output.push(bytes[index]);
        index += 1;
    }

    String::from_utf8_lossy(&output).into_owned()
}

#[cfg(test)]
mod tests {
    use super::{parse_local_mounts, used_bytes_from_fields};

    #[test]
    fn parses_mountinfo_and_filters_non_local_types() {
        let sample = "36 25 8:2 / / rw,relatime - ext4 /dev/nvme0n1p2 rw\n37 25 0:5 / /proc rw,nosuid,nodev,noexec,relatime - proc proc rw\n38 25 0:57 / /mnt/nfs rw,relatime - nfs server:/x rw\n";

        let mounts = parse_local_mounts(sample).expect("parse should succeed");
        assert_eq!(mounts, vec![std::path::PathBuf::from("/")]);
    }

    #[test]
    fn sums_used_bytes_for_included_mounts() {
        assert_eq!(used_bytes_from_fields(1_000, 250, 4096), 3_072_000);
    }
}

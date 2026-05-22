use crate::error::{Result, SysError};
use std::path::PathBuf;
use std::process::Command;
use tracing::debug;

/// Unescape octal sequences in /proc/mounts paths (e.g. `\040` -> ` `)
fn unescape_mount_path(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            let mut octal = String::with_capacity(3);
            for _ in 0..3 {
                if let Some(&next) = chars.as_str().as_bytes().first()
                    && (b'0'..=b'7').contains(&next)
                {
                    octal.push(next as char);
                    chars.next();
                } else {
                    break;
                }
            }
            if octal.len() == 3 {
                if let Ok(byte) = u8::from_str_radix(&octal, 8) {
                    result.push(byte as char);
                } else {
                    result.push('\\');
                    result.push_str(&octal);
                }
            } else {
                result.push('\\');
                result.push_str(&octal);
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub(crate) fn is_mounted(mount_point: &PathBuf) -> Result<bool> {
    debug!("Checking if {:?} is mounted", mount_point);

    let canonical = mount_point
        .canonicalize()
        .unwrap_or_else(|_| mount_point.clone());

    if canonical.exists() {
        let output = Command::new("mountpoint")
            .arg("-q")
            .arg(&canonical)
            .output()
            .map_err(|e| SysError::OperationFailed(format!("Failed to run mountpoint: {}", e)))?;
        if output.status.success() {
            return Ok(true);
        }
    }

    let mount_path_str = canonical.to_string_lossy();
    if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
        for line in mounts.lines() {
            let mut fields = line.split_whitespace();
            if let Some(_device) = fields.next()
                && let Some(mp) = fields.next()
            {
                let unescaped = unescape_mount_path(mp);
                if unescaped == mount_path_str.as_ref() {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

// SPDX-License-Identifier: GPL-3.0-only

use std::path::Path;

pub const PROTECTED_SYSTEM_PATHS: &[&str] = &[
    "/",
    "/boot",
    "/boot/efi",
    "/efi",
    "/home",
    "/usr",
    "/var",
    "/etc",
    "/opt",
    "/srv",
    "/tmp",
    "/root",
];

pub fn is_protected_path(mount_point: &Path) -> bool {
    let Ok(canonical_mount) = mount_point.canonicalize() else {
        return false;
    };
    PROTECTED_SYSTEM_PATHS.iter().any(|protected| {
        let protected = Path::new(protected);
        let canonical = protected
            .canonicalize()
            .unwrap_or_else(|_| protected.to_path_buf());
        canonical_mount == canonical || canonical_mount.starts_with(canonical)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_is_protected() {
        assert!(is_protected_path(Path::new("/")));
    }
}

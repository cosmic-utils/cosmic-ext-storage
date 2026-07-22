// SPDX-License-Identifier: GPL-3.0-only

use std::path::Path;

use super::types::Category;

pub fn classify_path(path: &Path) -> Category {
    let lower_path = path.to_string_lossy().to_ascii_lowercase();
    if is_package_path(&lower_path) {
        return Category::Packages;
    }

    if is_system_path(&lower_path) {
        return Category::System;
    }

    let lower_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if lower_name.ends_with(".deb")
        || lower_name.ends_with(".rpm")
        || lower_name.ends_with(".pkg.tar")
        || lower_name.ends_with(".pkg.tar.zst")
    {
        return Category::Packages;
    }

    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return Category::Other;
    };

    let ext = ext.to_ascii_lowercase();

    match ext.as_str() {
        "txt" | "pdf" | "doc" | "docx" | "odt" | "rtf" | "md" | "markdown" | "epub" | "xls"
        | "xlsx" | "ods" | "ppt" | "pptx" | "odp" => Category::Documents,
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "svg" | "tif" | "tiff" | "heic"
        | "avif" => Category::Images,
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "opus" => Category::Audio,
        "mp4" | "mkv" | "webm" | "mov" | "avi" | "flv" | "m4v" => Category::Video,
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "zst" | "iso" | "img" | "dmg"
        | "qcow" | "qcow2" | "vdi" | "vmdk" => Category::Archives,
        "rs" | "c" | "h" | "cpp" | "hpp" | "cc" | "py" | "js" | "ts" | "tsx" | "jsx" | "java"
        | "kt" | "go" | "rb" | "php" | "swift" | "cs" | "toml" | "yaml" | "yml" | "json"
        | "xml" | "sh" | "bash" | "zsh" | "fish" | "sql" => Category::Code,
        "bin" | "so" | "dll" | "exe" | "appimage" | "deb" | "rpm" | "apk" | "msi" => {
            Category::Binaries
        }
        _ => Category::Other,
    }
}

fn is_package_path(lower_path: &str) -> bool {
    lower_path.starts_with("/var/lib/dpkg")
        || lower_path.starts_with("/var/lib/rpm")
        || lower_path.starts_with("/var/lib/pacman")
        || lower_path.starts_with("/var/cache/pacman/pkg")
        || lower_path.starts_with("/var/cache/apt/archives")
}

fn is_system_path(lower_path: &str) -> bool {
    lower_path.starts_with("/usr/")
        || lower_path == "/usr"
        || lower_path.starts_with("/lib/")
        || lower_path == "/lib"
        || lower_path.starts_with("/lib64/")
        || lower_path == "/lib64"
        || lower_path.starts_with("/etc/")
        || lower_path == "/etc"
        || lower_path.starts_with("/boot/")
        || lower_path == "/boot"
        || lower_path.starts_with("/opt/")
        || lower_path == "/opt"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_by_extension_and_falls_back_to_other() {
        assert_eq!(classify_path(Path::new("/tmp/file.rs")), Category::Code);
        assert_eq!(classify_path(Path::new("/tmp/pic.JPG")), Category::Images);
        assert_eq!(
            classify_path(Path::new("/tmp/live.iso")),
            Category::Archives
        );
        assert_eq!(
            classify_path(Path::new("/tmp/disk.img")),
            Category::Archives
        );
        assert_eq!(classify_path(Path::new("/tmp/noext")), Category::Other);
        assert_eq!(classify_path(Path::new("/usr/bin/bash")), Category::System);
        assert_eq!(
            classify_path(Path::new("/var/cache/apt/archives/demo.deb")),
            Category::Packages
        );
    }
}

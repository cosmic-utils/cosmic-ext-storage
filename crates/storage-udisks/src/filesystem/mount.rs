// SPDX-License-Identifier: GPL-3.0-only

//! Filesystem mount/unmount operations

use crate::error::DiskError;
use std::collections::HashMap;
use storage_types::MountOptions;
use udisks2::filesystem::FilesystemProxy;
use zbus::zvariant::Value;

/// Get username from UID
fn get_username_from_uid(uid: u32) -> Option<String> {
    // SAFETY: getpwuid is thread-safe in POSIX
    let pw = unsafe { libc::getpwuid(uid) };
    if pw.is_null() {
        return None;
    }
    // SAFETY: pw is valid and name is a null-terminated C string
    unsafe {
        std::ffi::CStr::from_ptr((*pw).pw_name)
            .to_str()
            .ok()
            .map(|s| s.to_string())
    }
}

/// Mount a filesystem
///
/// # Arguments
/// * `device_path` - Device path (e.g., "/dev/sda1")
/// * `_mount_point` - Mount point (unused when auto-mounting)
/// * `options` - Mount options
/// * `caller_uid` - Optional UID to mount as (important for proper file ownership and mount path)
pub async fn mount_filesystem(
    device_path: &str,
    _mount_point: &str,
    options: MountOptions,
    caller_uid: Option<u32>,
) -> Result<String, DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let fs_path = crate::disk::resolve::block_object_path_for_device(device_path).await?;

    let fs_proxy = FilesystemProxy::builder(&connection)
        .path(&fs_path)
        .map_err(|e| DiskError::DBusError(e.to_string()))?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    // Build mount options
    let mut opts: HashMap<&str, Value<'_>> = HashMap::new();
    let mut options_vec = Vec::new();

    // Use UDisks2's `as-user` option to mount on behalf of the caller
    // This ensures the mount point is created under /run/media/<username>/
    // and the user can unmount it later
    if let Some(uid) = caller_uid
        && let Some(username) = get_username_from_uid(uid)
    {
        opts.insert("as-user", Value::from(username.clone()));
        // Also set uid for filesystems that support it (vfat, ntfs, etc.)
        opts.insert("uid", Value::from(uid));
    }

    if options.read_only {
        options_vec.push("ro");
    }
    if options.no_exec {
        options_vec.push("noexec");
    }
    if options.no_suid {
        options_vec.push("nosuid");
    }
    for opt in &options.other {
        options_vec.push(opt.as_str());
    }

    if !options_vec.is_empty() {
        opts.insert("options", Value::from(options_vec.join(",")));
    }

    // Mount
    let mount_point_bytes = fs_proxy
        .mount(opts)
        .await
        .map_err(|e| DiskError::OperationFailed(format!("Mount failed: {}", e)))?;

    // mount_point_bytes is a String returned by UDisks2
    Ok(mount_point_bytes)
}

/// Unmount a filesystem
pub async fn unmount_filesystem(device_or_mount: &str, force: bool) -> Result<(), DiskError> {
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let fs_path = match crate::disk::resolve::block_object_path_for_device(device_or_mount).await {
        Ok(p) => p,
        Err(_) => crate::disk::resolve::block_object_path_for_mount_point(device_or_mount).await?,
    };

    let fs_proxy = FilesystemProxy::builder(&connection)
        .path(&fs_path)
        .map_err(|e| DiskError::DBusError(e.to_string()))?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let mut opts: HashMap<&str, Value<'_>> = HashMap::new();
    if force {
        opts.insert("force", Value::from(true));
    }

    fs_proxy
        .unmount(opts)
        .await
        .map_err(|e| DiskError::OperationFailed(format!("Unmount failed: {}", e)))?;

    Ok(())
}

/// Get the mount point for a mounted device
pub async fn get_mount_point(device: &str) -> Result<String, DiskError> {
    let connection = crate::manager::shared_connection().await.map_err(|e| {
        DiskError::ConnectionFailed(format!("Failed to connect to system bus: {}", e))
    })?;

    let fs_path = crate::disk::resolve::block_object_path_for_device(device).await?;

    let fs_proxy = FilesystemProxy::builder(&connection)
        .path(&fs_path)
        .map_err(|e| DiskError::InvalidPath(format!("Invalid filesystem path: {}", e)))?
        .build()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    let mount_points = fs_proxy
        .mount_points()
        .await
        .map_err(|e| DiskError::DBusError(e.to_string()))?;

    if mount_points.is_empty() {
        return Err(DiskError::OperationFailed(
            "Device is not mounted".to_string(),
        ));
    }

    // mount_points returns Vec<Vec<u8>> with each mount point as null-terminated string
    let mount_str = String::from_utf8(
        mount_points[0]
            .clone()
            .into_iter()
            .filter(|&b| b != 0)
            .collect(),
    )
    .map_err(|e| DiskError::OperationFailed(format!("Invalid mount point encoding: {}", e)))?;

    Ok(mount_str)
}

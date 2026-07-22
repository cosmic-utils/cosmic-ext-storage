// SPDX-License-Identifier: GPL-3.0-only

//! Low-level RClone CLI operations
//!
//! This module provides wrappers around the rclone command-line tool
//! for listing remotes, reading configuration, and managing mounts.

use crate::error::{Result, SysError};
use configparser::ini::Ini;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use tracing::{debug, info, warn};
use which::which;

mod mount_state;
mod systemd;

pub mod backend;

use mount_state::is_mounted;

pub use backend::RcloneNetworkBackend;

/// RClone CLI wrapper for low-level operations
pub struct RCloneCli {
    /// Path to the rclone binary
    binary_path: PathBuf,
}

impl RCloneCli {
    /// Create a new RClone CLI wrapper
    ///
    /// Returns an error if rclone is not installed
    pub fn new() -> Result<Self> {
        let binary_path = Self::find_rclone_binary()?;
        info!("Found rclone binary at {:?}", binary_path);
        Ok(Self { binary_path })
    }

    /// Find the rclone binary in PATH
    pub fn find_rclone_binary() -> Result<PathBuf> {
        which("rclone").map_err(|_| SysError::RCloneNotFound)
    }

    /// Get the current desktop user's rclone configuration path.
    pub fn get_config_path() -> Result<PathBuf> {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join(".config/rclone/rclone.conf"))
            .ok_or_else(|| {
                SysError::OperationFailed(
                    "Could not determine the current user's home directory".into(),
                )
            })
    }

    /// List all configured remotes using `rclone listremotes`
    pub fn list_remotes(&self, config_path: &PathBuf) -> Result<Vec<String>> {
        debug!("Listing remotes from {:?}", config_path);

        let output = Command::new(&self.binary_path)
            .arg("listremotes")
            .arg("--config")
            .arg(config_path)
            .output()
            .map_err(|e| SysError::OperationFailed(format!("Failed to execute rclone: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("rclone listremotes failed: {}", stderr);
            return Err(SysError::RCloneConfigParse(format!(
                "Failed to list remotes: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let remotes: Vec<String> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.trim().trim_end_matches(':').to_string())
            .collect();

        debug!("Found {} remotes", remotes.len());
        Ok(remotes)
    }

    /// Read and parse the rclone configuration file
    pub fn read_config(
        &self,
        config_path: &PathBuf,
    ) -> Result<HashMap<String, HashMap<String, Option<String>>>> {
        debug!("Reading config from {:?}", config_path);

        if !config_path.exists() {
            return Err(SysError::RCloneConfigNotFound);
        }

        // Read the file content first to provide better error messages
        let content = std::fs::read_to_string(config_path).map_err(|e| {
            SysError::RCloneConfigParse(format!("Failed to read configuration file: {}", e))
        })?;

        // Check if the file is empty
        if content.trim().is_empty() {
            warn!("Configuration file is empty: {:?}", config_path);
            return Ok(HashMap::new());
        }

        let mut conf = Ini::new();
        match conf.read(content) {
            Ok(_) => {
                let remotes = conf.get_map_ref().clone();
                debug!("Parsed {} remote sections", remotes.keys().count());
                Ok(remotes)
            }
            Err(e) => {
                warn!("Failed to parse rclone config: {}", e);
                Err(SysError::RCloneConfigParse(format!(
                    "Failed to parse configuration: {}",
                    e
                )))
            }
        }
    }

    /// Get the current desktop user's mount point for a remote.
    pub fn get_mount_point(remote_name: &str) -> Result<PathBuf> {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join("mnt").join(remote_name))
            .ok_or_else(|| {
                SysError::OperationFailed(
                    "Could not determine the current user's home directory".into(),
                )
            })
    }

    /// Check if a mount point is currently mounted
    ///
    /// Uses two approaches:
    /// 1. The `mountpoint -q` command (works for local mounts visible in current namespace)
    /// 2. Fallback: parsing `/proc/mounts` which shows all mounts system-wide, including
    ///    FUSE mounts created by user systemd units that may not be visible via `mountpoint`
    ///    when checked from a root service context.
    pub fn is_mounted(mount_point: &PathBuf) -> Result<bool> {
        is_mounted(mount_point)
    }

    /// Mount a remote using `rclone mount`
    ///
    /// This runs rclone in daemon mode (--daemon) which forks into the background
    pub fn mount(
        &self,
        remote_name: &str,
        mount_point: &PathBuf,
        config_path: &PathBuf,
    ) -> Result<()> {
        info!("Mounting remote {} at {:?}", remote_name, mount_point);

        // Check if already mounted
        if Self::is_mounted(mount_point)? {
            return Err(SysError::RCloneAlreadyMounted(remote_name.to_string()));
        }

        // Create mount point if it doesn't exist
        if !mount_point.exists() {
            std::fs::create_dir_all(mount_point).map_err(|e| {
                SysError::OperationFailed(format!("Failed to create mount point: {}", e))
            })?;
        }

        let remote_path = format!("{}:", remote_name);

        let output = Command::new(&self.binary_path)
            .arg("mount")
            .arg(&remote_path)
            .arg(mount_point)
            .arg("--config")
            .arg(config_path)
            .arg("--daemon")
            .arg("--vfs-cache-mode")
            .arg("writes")
            .output()
            .map_err(|e| {
                SysError::RCloneMountFailed(format!("Failed to execute rclone mount: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("rclone mount failed: {}", stderr);
            return Err(SysError::RCloneMountFailed(format!(
                "Mount failed for {}: {}",
                remote_name, stderr
            )));
        }

        info!("Successfully mounted {} at {:?}", remote_name, mount_point);
        Ok(())
    }

    /// Unmount a remote using fusermount
    pub fn unmount(&self, mount_point: &PathBuf) -> Result<()> {
        info!("Unmounting {:?}", mount_point);

        // Check if mounted
        if !Self::is_mounted(mount_point)? {
            return Err(SysError::RCloneNotMounted(
                mount_point.display().to_string(),
            ));
        }

        let output = Command::new("fusermount")
            .arg("-u")
            .arg(mount_point)
            .output()
            .map_err(|e| {
                SysError::RCloneUnmountFailed(format!("Failed to execute fusermount: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("fusermount failed: {}", stderr);
            return Err(SysError::RCloneUnmountFailed(format!(
                "Unmount failed for {:?}: {}",
                mount_point, stderr
            )));
        }

        info!("Successfully unmounted {:?}", mount_point);
        Ok(())
    }

    /// Test a remote configuration using `rclone ls`
    ///
    /// Returns (success, message, latency_ms)
    pub fn test_remote(
        &self,
        remote_name: &str,
        config_path: &PathBuf,
    ) -> Result<(bool, String, u64)> {
        info!(
            "Testing remote {} with config {:?}",
            remote_name, config_path
        );

        let remote_path = format!("{}:", remote_name);
        let start = Instant::now();

        let output = Command::new(&self.binary_path)
            .arg("ls")
            .arg(&remote_path)
            .arg("--config")
            .arg(config_path)
            .arg("--max-depth")
            .arg("1")
            .output()
            .map_err(|e| {
                SysError::RCloneTestFailed(format!("Failed to execute rclone ls: {}", e))
            })?;

        let latency_ms = start.elapsed().as_millis() as u64;

        if output.status.success() {
            info!("Remote {} test succeeded in {}ms", remote_name, latency_ms);
            Ok((true, "Connection successful".to_string(), latency_ms))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let message = if stderr.is_empty() {
                "Connection failed".to_string()
            } else {
                stderr.to_string()
            };
            warn!("Remote {} test failed: {}", remote_name, message);
            Ok((false, message, latency_ms))
        }
    }

    /// Write configuration back to file
    pub fn write_config(
        &self,
        config_path: &PathBuf,
        remotes: &HashMap<String, HashMap<String, Option<String>>>,
    ) -> Result<()> {
        info!("Writing config to {:?}", config_path);

        let mut conf = Ini::new();

        for (section, properties) in remotes.iter() {
            for (key, value) in properties.iter() {
                conf.set(section, key, value.clone());
            }
        }

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent).map_err(|e| {
                SysError::OperationFailed(format!("Failed to create config directory: {}", e))
            })?;
        }

        conf.write(config_path)
            .map_err(|e| SysError::RCloneConfigParse(format!("Failed to write config: {}", e)))?;

        info!("Successfully wrote config to {:?}", config_path);
        Ok(())
    }
}

impl std::fmt::Debug for RCloneCli {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RCloneCli")
            .field("binary_path", &self.binary_path)
            .finish()
    }
}

pub fn set_mount_on_login(remote_name: &str, enabled: bool, home: &std::path::Path) -> Result<()> {
    systemd::set_mount_on_login(remote_name, enabled, home)
}

pub fn is_mount_on_login_enabled(remote_name: &str, home: &std::path::Path) -> Result<bool> {
    systemd::is_mount_on_login_enabled(remote_name, home)
}

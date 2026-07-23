// SPDX-License-Identifier: GPL-3.0-only

use crate::error::{BtrfsError, Result};
use btrfsutil::subvolume::{DeleteFlags, SnapshotFlags, Subvolume};
use std::path::{Path, PathBuf};
use std::process::Command;
use storage_types::btrfs::BtrfsSubvolume;

/// Manager for BTRFS subvolume operations
pub struct SubvolumeManager {
    mount_point: PathBuf,
}

impl SubvolumeManager {
    /// Create a new SubvolumeManager for the given mount point
    pub fn new<P: Into<PathBuf>>(mount_point: P) -> Result<Self> {
        let mount_point = mount_point.into();

        // Verify the path is a BTRFS filesystem
        match Subvolume::try_from(mount_point.as_path()) {
            Ok(_) => Ok(Self { mount_point }),
            Err(e) => Err(BtrfsError::NotMounted(format!(
                "{}: {}",
                mount_point.display(),
                e
            ))),
        }
    }

    /// List all subvolumes in the filesystem
    pub fn list_all(&self) -> Result<Vec<BtrfsSubvolume>> {
        // Use btrfs command-line tool instead of btrfsutil iterator
        // The iterator fails with "Could not statfs" when running via pkexec
        let output = Command::new("btrfs")
            .args(["subvolume", "list", "-a", "-u", "-q", "-R"])
            .arg(&self.mount_point)
            .output()
            .map_err(|e| {
                BtrfsError::CommandFailed(format!("Failed to run btrfs command: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BtrfsError::CommandFailed(format!(
                "btrfs command failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut subvolumes = Vec::new();

        // Parse output: ID 256 gen 89534 parent 5 top level 5 parent_uuid - received_uuid - uuid ... path ...
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 11 || parts[0] != "ID" {
                continue;
            }

            let id = parts[1].parse::<u64>().ok();
            let generation = parts[3].parse::<u64>().ok();

            // Find "path" keyword and take everything after it
            let path_idx = parts.iter().position(|&p| p == "path");
            let mut path = if let Some(idx) = path_idx {
                parts[idx + 1..].join(" ")
            } else {
                continue;
            };

            // Strip "<FS_TREE>/" prefix from paths (comes from -a flag)
            if path.starts_with("<FS_TREE>/") {
                path = path
                    .strip_prefix("<FS_TREE>/")
                    .ok_or_else(|| {
                        BtrfsError::OperationFailed(
                            "Failed to strip <FS_TREE>/ prefix from subvolume path".to_string(),
                        )
                    })?
                    .to_string();
            }

            // Find UUID field (after "uuid" keyword)
            let uuid_idx = parts.iter().position(|&p| p == "uuid");
            let uuid = if let Some(idx) = uuid_idx {
                parts
                    .get(idx + 1)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| String::from("00000000-0000-0000-0000-000000000000"))
            } else {
                String::from("00000000-0000-0000-0000-000000000000")
            };

            // Find parent_uuid field (after "parent_uuid" keyword)
            let parent_uuid_idx = parts.iter().position(|&p| p == "parent_uuid");
            let parent_uuid = if let Some(idx) = parent_uuid_idx {
                parts.get(idx + 1).and_then(|s| {
                    if *s == "-" {
                        None // "-" means no parent (original subvolume)
                    } else {
                        Some(s.to_string())
                    }
                })
            } else {
                None
            };

            // Find received_uuid field (after "received_uuid" keyword)
            let received_uuid_idx = parts.iter().position(|&p| p == "received_uuid");
            let received_uuid = if let Some(idx) = received_uuid_idx {
                parts
                    .get(idx + 1)
                    .and_then(|s| if *s == "-" { None } else { Some(s.to_string()) })
            } else {
                None
            };

            if let (Some(id), Some(generation)) = (id, generation) {
                subvolumes.push(BtrfsSubvolume {
                    id,
                    path,
                    parent_id: None, // Not critical for UI
                    uuid,
                    parent_uuid,
                    received_uuid,
                    generation,
                    ctransid: generation, // Approximate
                    otransid: generation, // Approximate
                    stransid: None,
                    rtransid: None,
                    ctime: 0, // Not available from list command
                    otime: 0, // Not available from list command
                    stime: None,
                    rtime: None,
                    flags: 0,
                });
            }
        }

        if subvolumes.is_empty() {
            tracing::warn!("No subvolumes found - output may not have been parsed correctly");
        }

        Ok(subvolumes)
    }

    /// Create a new subvolume
    pub fn create(&self, name: &str) -> Result<()> {
        let subvol_path = self.mount_point.join(name);

        Subvolume::create(subvol_path.as_path(), None).map_err(|e| {
            BtrfsError::OperationFailed(format!("Failed to create subvolume '{}': {}", name, e))
        })?;

        Ok(())
    }

    /// Delete a subvolume
    pub fn delete(&self, path: &Path, recursive: bool) -> Result<()> {
        let subvol = Subvolume::try_from(path)
            .map_err(|e| BtrfsError::SubvolumeNotFound(format!("{}: {}", path.display(), e)))?;

        let flags = if recursive {
            DeleteFlags::RECURSIVE
        } else {
            DeleteFlags::empty()
        };

        subvol.delete(flags).map_err(|e| {
            BtrfsError::OperationFailed(format!(
                "Failed to delete subvolume at {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(())
    }

    /// Create a snapshot of a subvolume
    pub fn snapshot(
        &self,
        source: &Path,
        dest: &Path,
        readonly: bool,
        recursive: bool,
    ) -> Result<()> {
        let source_subvol = Subvolume::try_from(source).map_err(|e| {
            BtrfsError::SubvolumeNotFound(format!("Source {}: {}", source.display(), e))
        })?;

        let mut flags = SnapshotFlags::empty();
        if readonly {
            flags |= SnapshotFlags::READ_ONLY;
        }
        if recursive {
            flags |= SnapshotFlags::RECURSIVE;
        }

        source_subvol.snapshot(dest, flags, None).map_err(|e| {
            BtrfsError::OperationFailed(format!(
                "Failed to create snapshot from {} to {}: {}",
                source.display(),
                dest.display(),
                e
            ))
        })?;

        Ok(())
    }

    /// Set or unset the read-only flag on a subvolume
    pub fn set_readonly(&self, path: &Path, readonly: bool) -> Result<()> {
        let subvol = Subvolume::try_from(path)
            .map_err(|e| BtrfsError::SubvolumeNotFound(format!("{}: {}", path.display(), e)))?;

        subvol.set_ro(readonly).map_err(|e| {
            BtrfsError::OperationFailed(format!(
                "Failed to set readonly={} on {}: {}",
                readonly,
                path.display(),
                e
            ))
        })?;

        Ok(())
    }

    /// Set a subvolume as the default
    pub fn set_default(&self, path: &Path) -> Result<()> {
        let subvol = Subvolume::try_from(path)
            .map_err(|e| BtrfsError::SubvolumeNotFound(format!("{}: {}", path.display(), e)))?;

        Subvolume::set_default(&subvol).map_err(|e| {
            BtrfsError::OperationFailed(format!(
                "Failed to set default subvolume to {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(())
    }

    /// Get the default subvolume ID
    pub fn get_default(&self) -> Result<u64> {
        let root = Subvolume::try_from(self.mount_point.as_path()).map_err(|e| {
            BtrfsError::NotMounted(format!("{}: {}", self.mount_point.display(), e))
        })?;

        let default_subvol = Subvolume::get_default(&root).map_err(|e| {
            BtrfsError::OperationFailed(format!("Failed to get default subvolume: {}", e))
        })?;

        Ok(default_subvol.id())
    }

    /// List deleted subvolumes pending cleanup
    pub fn list_deleted(&self) -> Result<Vec<BtrfsSubvolume>> {
        let root = Subvolume::try_from(self.mount_point.as_path()).map_err(|e| {
            BtrfsError::NotMounted(format!("{}: {}", self.mount_point.display(), e))
        })?;

        let deleted_iter = Subvolume::deleted(&root).map_err(|e| {
            BtrfsError::OperationFailed(format!("Failed to list deleted subvolumes: {}", e))
        })?;

        let mut deleted = Vec::new();

        for subvol in deleted_iter {
            let info = subvol.info().map_err(|e| {
                BtrfsError::OperationFailed(format!("Failed to get deleted subvolume info: {}", e))
            })?;

            deleted.push(BtrfsSubvolume {
                id: info.id,
                path: subvol.path().to_string_lossy().to_string(),
                parent_id: info.parent_id,
                uuid: info.uuid.to_string(),
                parent_uuid: info.parent_uuid.map(|u| u.to_string()),
                received_uuid: info.received_uuid.map(|u| u.to_string()),
                generation: info.generation,
                ctransid: info.ctransid,
                otransid: info.otransid,
                stransid: info.stransid,
                rtransid: info.rtransid,
                ctime: info.ctime.timestamp(),
                otime: info.otime.timestamp(),
                stime: info.stime.map(|t| t.timestamp()),
                rtime: info.rtime.map(|t| t.timestamp()),
                flags: info.flags,
            });
        }

        Ok(deleted)
    }
}

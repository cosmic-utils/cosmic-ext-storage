// SPDX-License-Identifier: GPL-3.0-only

//! LUKS device listing

use crate::error::DiskError;
use crate::manager::DiskManager;
use storage_types::{LuksInfo, LuksVersion, VolumeInfo, VolumeKind};

fn collect_luks_devices(volume: &VolumeInfo, output: &mut Vec<LuksInfo>) {
    if volume.kind == VolumeKind::CryptoContainer {
        let cleartext_device = volume
            .children
            .iter()
            .find_map(|child| child.device_path.clone());

        let version = volume
            .id_type
            .split('_')
            .find_map(LuksVersion::parse)
            .unwrap_or(LuksVersion::Luks2);

        let device = volume
            .device_path
            .clone()
            .unwrap_or_else(|| volume.label.clone());

        output.push(LuksInfo {
            device,
            version,
            cipher: String::new(),
            key_size: 0,
            unlocked: !volume.locked,
            cleartext_device,
            keyslot_count: 0,
        });
    }

    for child in &volume.children {
        collect_luks_devices(child, output);
    }
}

/// List all LUKS encrypted devices
pub async fn list_luks_devices() -> Result<Vec<LuksInfo>, DiskError> {
    let manager = DiskManager::new()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;

    let disks_with_volumes = crate::disk::get_disks_with_volumes(&manager)
        .await
        .map_err(|e| DiskError::OperationFailed(e.to_string()))?;

    let mut luks_devices = Vec::new();
    for (_disk, volumes) in disks_with_volumes {
        for volume in &volumes {
            collect_luks_devices(volume, &mut luks_devices);
        }
    }

    Ok(luks_devices)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_volume(kind: VolumeKind, device: Option<&str>, locked: bool) -> VolumeInfo {
        VolumeInfo {
            kind,
            label: String::new(),
            size: 0,
            offset: 0,
            partition_number: 0,
            id_type: "crypto_LUKS_luks2".to_string(),
            device_path: device.map(ToString::to_string),
            parent_path: None,
            has_filesystem: false,
            mount_points: Vec::new(),
            usage: None,
            locked,
            children: Vec::new(),
        }
    }

    #[test]
    fn collects_luks_devices_from_nested_tree() {
        let mut root = make_volume(VolumeKind::Partition, Some("/dev/sda1"), false);
        let mut crypto = make_volume(VolumeKind::CryptoContainer, Some("/dev/sda2"), false);
        let cleartext = make_volume(VolumeKind::Filesystem, Some("/dev/mapper/luks-abc"), false);
        crypto.children.push(cleartext);
        root.children.push(crypto);

        let mut out = Vec::new();
        collect_luks_devices(&root, &mut out);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].device, "/dev/sda2");
        assert!(out[0].unlocked);
        assert_eq!(
            out[0].cleartext_device.as_deref(),
            Some("/dev/mapper/luks-abc")
        );
        assert_eq!(out[0].version, LuksVersion::Luks2);
    }
}

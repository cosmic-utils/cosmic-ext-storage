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
    let connection = crate::manager::shared_connection()
        .await
        .map_err(|e| DiskError::ConnectionFailed(e.to_string()))?;
    list_luks_devices_with_connection(connection.as_ref()).await
}

pub(crate) async fn list_luks_devices_with_connection(
    connection: &zbus::Connection,
) -> Result<Vec<LuksInfo>, DiskError> {
    let manager = DiskManager::from_connection(std::sync::Arc::new(connection.clone()));

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

#[path = "../../tests/unit/encryption/list_tests.rs"]
#[cfg(test)]
mod tests;

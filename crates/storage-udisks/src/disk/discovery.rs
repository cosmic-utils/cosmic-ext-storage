// SPDX-License-Identifier: GPL-3.0-only

//! Disk and volume discovery - builds storage_types::DiskInfo and VolumeInfo directly from UDisks2.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use storage_types::{DiskInfo, PartitionInfo, VolumeInfo, VolumeKind};
use udisks2::{
    block::BlockProxy,
    drive::{DriveProxy, RotationRate},
    encrypted::EncryptedProxy,
    partitiontable::PartitionTableProxy,
};
use zbus::Connection;
use zbus::zvariant::OwnedObjectPath;

use super::block_index::BlockIndex;
use super::volume_tree;
use crate::dbus::bytestring as bs;
use crate::gpt::{fallback_gpt_usable_range_bytes, probe_gpt_usable_range_bytes};
use crate::manager::DiskManager;
use crate::manager::UDisks2ManagerProxy;

#[derive(Debug, Clone)]
struct DriveBlockPair {
    block_path: OwnedObjectPath,
    drive_path: Option<OwnedObjectPath>,
    backing_file: Option<String>,
}

async fn loop_backing_file(
    connection: &Connection,
    block_path: &OwnedObjectPath,
) -> Option<String> {
    let proxy = match zbus::Proxy::new(
        connection,
        "org.freedesktop.UDisks2",
        block_path.as_str(),
        "org.freedesktop.UDisks2.Loop",
    )
    .await
    {
        Ok(p) => p,
        Err(_) => return None,
    };

    if let Ok(bytes) = proxy.get_property::<Vec<u8>>("BackingFile").await {
        let raw = bytes.split(|b| *b == 0).next().unwrap_or(&bytes);
        let s = String::from_utf8_lossy(raw).to_string();
        if !s.trim().is_empty() {
            return Some(s);
        }
    }

    if let Ok(s) = proxy.get_property::<String>("BackingFile").await
        && !s.trim().is_empty()
    {
        return Some(s);
    }

    None
}

async fn get_drive_paths(connection: &Connection) -> Result<Vec<DriveBlockPair>> {
    let manager_proxy = UDisks2ManagerProxy::new(connection).await?;
    let block_paths = manager_proxy.get_block_devices(HashMap::new()).await?;

    let mut drive_paths: Vec<DriveBlockPair> = vec![];

    for path in block_paths {
        let block_device = match BlockProxy::builder(connection).path(&path)?.build().await {
            Ok(d) => d,
            Err(e) => {
                tracing::info!("Could not get block device: {}", e);
                continue;
            }
        };

        if let Ok(partition_proxy) = udisks2::partition::PartitionProxy::builder(connection)
            .path(&path)?
            .build()
            .await
            && partition_proxy.table().await.is_ok()
        {
            continue;
        }

        match block_device.drive().await {
            Ok(dp) if dp.as_str() != "/" => drive_paths.push(DriveBlockPair {
                block_path: path,
                drive_path: Some(dp),
                backing_file: None,
            }),
            _ => {
                let backing = loop_backing_file(connection, &path).await;
                if backing.is_some() {
                    drive_paths.push(DriveBlockPair {
                        block_path: path,
                        drive_path: None,
                        backing_file: backing,
                    });
                }
            }
        }
    }

    Ok(drive_paths)
}

fn infer_connection_bus(
    block_path: &str,
    model: &str,
    vendor: &str,
    is_loop: bool,
    optical: bool,
) -> String {
    if is_loop {
        return "loop".to_string();
    }

    let path_lower = block_path.to_lowercase();
    let model_lower = model.to_lowercase();
    let vendor_lower = vendor.to_lowercase();

    if path_lower.contains("nvme") {
        return "nvme".to_string();
    }
    if path_lower.contains("mmc") || path_lower.contains("mmcblk") {
        return "mmc".to_string();
    }
    if path_lower.contains("sr") || optical {
        return "optical".to_string();
    }
    if model_lower.contains("usb") || vendor_lower.contains("usb") {
        return "usb".to_string();
    }

    "ata".to_string()
}

async fn block_device_path(
    connection: &Connection,
    block_path: &OwnedObjectPath,
) -> Result<String> {
    let block_proxy = BlockProxy::builder(connection)
        .path(block_path)?
        .build()
        .await?;
    let preferred = bs::decode_c_string_bytes(
        &block_proxy
            .preferred_device()
            .await
            .map_err(anyhow::Error::msg)?,
    );
    let device = if preferred.is_empty() {
        bs::decode_c_string_bytes(&block_proxy.device().await.map_err(anyhow::Error::msg)?)
    } else {
        preferred
    };
    Ok(if device.is_empty() {
        block_path.to_string()
    } else {
        device
    })
}

async fn build_disk_info(
    connection: &Connection,
    drive_path: Option<&OwnedObjectPath>,
    block_path: &OwnedObjectPath,
    backing_file: Option<String>,
) -> Result<DiskInfo> {
    let is_loop = backing_file.is_some();
    let device_path = block_device_path(connection, block_path).await?;

    let (
        id,
        model,
        serial,
        vendor,
        revision,
        size,
        can_power_off,
        ejectable,
        media_available,
        media_removable,
        optical,
        optical_blank,
        removable,
        rotation_rate,
    ) = if let Some(drive_path) = drive_path {
        let drive_proxy = DriveProxy::builder(connection)
            .path(drive_path)?
            .build()
            .await?;

        let mut size = drive_proxy.size().await?;
        if size == 0 {
            let block_proxy = BlockProxy::builder(connection)
                .path(block_path)?
                .build()
                .await?;
            size = block_proxy.size().await?;
        }

        let rot = match drive_proxy.rotation_rate().await {
            Ok(rate) => match rate {
                RotationRate::Rotating(rpm) => rpm,
                RotationRate::NonRotating => 0,
                RotationRate::Unknown => -1,
            },
            Err(_) => 0,
        };

        (
            drive_proxy.id().await?,
            drive_proxy.model().await?,
            drive_proxy.serial().await?,
            drive_proxy.vendor().await?,
            drive_proxy.revision().await?,
            size,
            drive_proxy.can_power_off().await?,
            drive_proxy.ejectable().await?,
            drive_proxy.media_available().await?,
            drive_proxy.media_removable().await?,
            drive_proxy.optical().await?,
            drive_proxy.optical_blank().await?,
            drive_proxy.removable().await?,
            rot,
        )
    } else {
        let block_proxy = BlockProxy::builder(connection)
            .path(block_path)?
            .build()
            .await?;
        let size = block_proxy.size().await?;
        (
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            size,
            false,
            false,
            true,
            false,
            false,
            false,
            false,
            0,
        )
    };

    let connection_bus = infer_connection_bus(&device_path, &model, &vendor, is_loop, optical);

    let rotation_rate = if rotation_rate > 0 {
        Some(rotation_rate as u16)
    } else {
        None
    };

    Ok(DiskInfo {
        device: device_path,
        id,
        model,
        serial,
        vendor,
        revision,
        size,
        connection_bus,
        rotation_rate,
        removable,
        ejectable,
        media_removable,
        media_available,
        optical,
        optical_blank,
        can_power_off,
        is_loop,
        backing_file,
        partition_table_type: None,
        gpt_usable_range: None,
    })
}

async fn build_volumes_for_block(
    connection: &Connection,
    block_path: &OwnedObjectPath,
    block_index: &BlockIndex,
    disk_device_path: &str,
) -> Result<Vec<VolumeInfo>> {
    let mut volumes = Vec::new();

    let partition_table_proxy = match PartitionTableProxy::builder(connection)
        .path(block_path)?
        .build()
        .await
    {
        Ok(p) => p,
        Err(_) => {
            let label = block_path
                .as_str()
                .split('/')
                .next_back()
                .unwrap_or("Block")
                .replace('_', " ");
            let info = volume_tree::from_block_object(
                connection,
                block_path.clone(),
                label,
                VolumeKind::Block,
                None,
                Some(block_index),
            )
            .await?;
            if info.has_filesystem {
                let mut fs_info = info;
                fs_info.kind = VolumeKind::Filesystem;
                if fs_info.label.trim().is_empty() {
                    fs_info.label = "Filesystem".to_string();
                }
                volumes.push(fs_info);
            } else {
                volumes.push(info);
            }
            return Ok(volumes);
        }
    };

    let partition_paths = match partition_table_proxy.partitions().await {
        Ok(p) => p,
        Err(_) => return Ok(volumes),
    };

    for (idx, part_path) in partition_paths.into_iter().enumerate() {
        let label = format!("Partition {}", idx + 1);

        let is_luks = match EncryptedProxy::builder(connection)
            .path(&part_path)?
            .build()
            .await
        {
            Ok(proxy) => proxy.cleartext_device().await.is_ok(),
            Err(_) => false,
        };

        let info = if is_luks {
            volume_tree::crypto_container_for_partition(
                connection,
                part_path,
                label,
                Some(disk_device_path.to_string()),
                block_index,
            )
            .await?
        } else {
            volume_tree::from_block_object(
                connection,
                part_path,
                label,
                VolumeKind::Partition,
                Some(disk_device_path.to_string()),
                Some(block_index),
            )
            .await?
        };

        volumes.push(info);
    }

    Ok(volumes)
}

fn flatten_volumes_to_partitions(
    volumes: &[VolumeInfo],
    parent_device: &str,
) -> Vec<PartitionInfo> {
    let mut out = Vec::new();
    for vol in volumes {
        // Only include actual disk partitions (have a partition number > 0)
        // This excludes nested volumes like LUKS cleartext devices
        if let Some(ref dev) = vol.device_path
            && vol.partition_number > 0
        {
            out.push(PartitionInfo {
                device: dev.clone(),
                number: vol.partition_number,
                parent_path: parent_device.to_string(),
                size: vol.size,
                offset: vol.offset,
                type_id: vol.id_type.clone(),
                type_name: String::new(),
                flags: 0,
                name: vol.label.clone(),
                uuid: String::new(),
                table_type: String::new(),
                has_filesystem: vol.has_filesystem,
                filesystem_type: if vol.has_filesystem {
                    Some(vol.id_type.clone())
                } else {
                    None
                },
                mount_points: vol.mount_points.clone(),
                usage: vol.usage.clone(),
            });
        }
        // Note: We intentionally do NOT recurse into children here.
        // Children like LUKS cleartext devices are not disk partitions.
    }
    // Sort by offset to ensure partitions appear in disk order
    out.sort_by_key(|p| p.offset);
    out
}

async fn get_disks_with_volumes_inner(
    connection: &Arc<Connection>,
) -> Result<Vec<(DiskInfo, Vec<VolumeInfo>)>> {
    let drive_paths = get_drive_paths(connection).await?;

    let manager_proxy = UDisks2ManagerProxy::new(connection).await?;
    let all_block_objects = manager_proxy.get_block_devices(HashMap::new()).await?;
    let block_index = BlockIndex::build(connection, &all_block_objects).await?;

    let mut result: Vec<(DiskInfo, Vec<VolumeInfo>)> = Vec::new();

    for pair in drive_paths {
        let disk_info = match build_disk_info(
            connection,
            pair.drive_path.as_ref(),
            &pair.block_path,
            pair.backing_file.clone(),
        )
        .await
        {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Could not get disk info: {}", e);
                continue;
            }
        };

        let block_path_str = pair.block_path.to_string();

        let partition_table_type = match PartitionTableProxy::builder(connection)
            .path(&pair.block_path)?
            .build()
            .await
        {
            Ok(pt) => pt.type_().await.ok(),
            Err(_) => None,
        };

        let mut disk_info = disk_info;
        disk_info.partition_table_type = partition_table_type.clone();

        if partition_table_type.as_deref() == Some("gpt")
            && let Ok(block_proxy) = BlockProxy::builder(connection)
                .path(&pair.block_path)?
                .build()
                .await
        {
            match probe_gpt_usable_range_bytes(&block_proxy, disk_info.size).await {
                Ok(Some(range)) => disk_info.gpt_usable_range = Some(range),
                Ok(None) => {
                    disk_info.gpt_usable_range = fallback_gpt_usable_range_bytes(disk_info.size);
                }
                Err(e) => {
                    tracing::warn!("GPT probe failed: {}; using fallback", e);
                    disk_info.gpt_usable_range = fallback_gpt_usable_range_bytes(disk_info.size);
                }
            }
        }

        let volumes = build_volumes_for_block(
            connection,
            &pair.block_path,
            &block_index,
            &disk_info.device,
        )
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("Could not build volumes for {}: {}", block_path_str, e);
            Vec::new()
        });

        result.push((disk_info, volumes));
    }

    result.sort_by(|a, b| {
        a.0.removable
            .cmp(&b.0.removable)
            .then_with(|| b.0.device.cmp(&a.0.device))
    });

    Ok(result)
}

/// Resolve a device path (e.g. "/dev/sda1") to the UDisks2 block object path.
pub async fn block_object_path_for_device(device: &str) -> Result<String, crate::error::DiskError> {
    super::resolve::block_object_path_for_device(device)
        .await
        .map(|path| path.to_string())
}

/// Get disk information as canonical storage-types models (public API).
/// Uses the cached connection from DiskManager for improved performance.
pub async fn get_disks(manager: &DiskManager) -> Result<Vec<DiskInfo>> {
    let pairs = get_disks_with_volumes_inner(manager.connection()).await?;
    Ok(pairs.into_iter().map(|(d, _)| d).collect())
}

/// Get disks with their volume hierarchies as canonical storage-types models.
/// Uses the cached connection from DiskManager for improved performance.
pub async fn get_disks_with_volumes(
    manager: &DiskManager,
) -> Result<Vec<(DiskInfo, Vec<VolumeInfo>)>> {
    get_disks_with_volumes_inner(manager.connection()).await
}

/// Get disks with flat partition lists as canonical storage-types models.
/// Uses the cached connection from DiskManager for improved performance.
pub async fn get_disks_with_partitions(
    manager: &DiskManager,
) -> Result<Vec<(DiskInfo, Vec<PartitionInfo>)>> {
    let pairs = get_disks_with_volumes_inner(manager.connection()).await?;
    Ok(pairs
        .into_iter()
        .map(|(d, vols)| {
            let device = d.device.clone();
            (d, flatten_volumes_to_partitions(&vols, &device))
        })
        .collect())
}

/// Get DiskInfo for a drive given its UDisks2 drive object path (e.g. from InterfacesAdded).
/// Uses the cached connection from DiskManager for improved performance.
pub async fn get_disk_info_for_drive_path(
    manager: &DiskManager,
    drive_path: &str,
) -> Result<DiskInfo> {
    let connection = manager.connection();
    let manager_proxy = UDisks2ManagerProxy::new(connection.as_ref()).await?;
    let block_paths = manager_proxy.get_block_devices(HashMap::new()).await?;
    for block_path in block_paths {
        let is_partition = match udisks2::partition::PartitionProxy::builder(connection.as_ref())
            .path(&block_path)?
            .build()
            .await
        {
            Ok(p) => p.table().await.is_ok(),
            Err(_) => false,
        };
        if is_partition {
            continue;
        }
        let block_proxy = BlockProxy::builder(connection.as_ref())
            .path(&block_path)?
            .build()
            .await?;
        if let Ok(d) = block_proxy.drive().await
            && d.as_str() == drive_path
        {
            return build_disk_info(connection.as_ref(), Some(&d), &block_path, None).await;
        }
    }
    Err(anyhow::anyhow!(
        "No block device found for drive: {}",
        drive_path
    ))
}

#[cfg(test)]
mod tests {
    use super::flatten_volumes_to_partitions;
    use storage_types::{VolumeInfo, VolumeKind};

    fn volume(
        kind: VolumeKind,
        partition_number: u32,
        offset: u64,
        device_path: Option<&str>,
        children: Vec<VolumeInfo>,
    ) -> VolumeInfo {
        VolumeInfo {
            kind,
            label: String::new(),
            size: 1024,
            offset,
            partition_number,
            id_type: "ext4".to_string(),
            device_path: device_path.map(ToOwned::to_owned),
            parent_path: None,
            has_filesystem: true,
            mount_points: vec![],
            usage: None,
            locked: false,
            children,
        }
    }

    #[test]
    fn flatten_partitions_sorts_by_offset() {
        let volumes = vec![
            volume(VolumeKind::Partition, 2, 4096, Some("/dev/sda2"), vec![]),
            volume(VolumeKind::Partition, 1, 2048, Some("/dev/sda1"), vec![]),
        ];

        let partitions = flatten_volumes_to_partitions(&volumes, "/dev/sda");

        let devices: Vec<&str> = partitions.iter().map(|p| p.device.as_str()).collect();
        assert_eq!(devices, vec!["/dev/sda1", "/dev/sda2"]);
    }

    #[test]
    fn flatten_partitions_ignores_non_partition_and_nested_children() {
        let nested_child = volume(
            VolumeKind::Partition,
            99,
            8192,
            Some("/dev/mapper/inner"),
            vec![],
        );
        let volumes = vec![
            volume(VolumeKind::Filesystem, 0, 1024, Some("/dev/sda"), vec![]),
            volume(
                VolumeKind::Partition,
                1,
                2048,
                Some("/dev/sda1"),
                vec![nested_child],
            ),
        ];

        let partitions = flatten_volumes_to_partitions(&volumes, "/dev/sda");

        assert_eq!(partitions.len(), 1);
        assert_eq!(partitions[0].device, "/dev/sda1");
    }
}

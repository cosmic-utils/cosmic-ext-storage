// SPDX-License-Identifier: GPL-3.0-only

//! Build storage_types::VolumeInfo tree from UDisks2.
//! Used by discovery to populate volumes per disk.

use std::path::Path;

use anyhow::Result;
use storage_types::VolumeKind;
use udisks2::{
    block::BlockProxy, encrypted::EncryptedProxy, filesystem::FilesystemProxy,
    partition::PartitionProxy, partitiontable::PartitionTableProxy,
};
use zbus::Connection;
use zbus::zvariant::OwnedObjectPath;

use super::block_index::BlockIndex;
use crate::dbus::bytestring as bs;
use crate::lvm;

/// Probe a block device and return a VolumeInfo (no children).
pub(crate) async fn probe_basic_block(
    connection: &Connection,
    object_path: OwnedObjectPath,
    label: String,
    kind: VolumeKind,
) -> Result<storage_types::VolumeInfo> {
    let block_proxy = BlockProxy::builder(connection)
        .path(&object_path)?
        .build()
        .await?;

    let preferred_device = bs::decode_c_string_bytes(
        &block_proxy
            .preferred_device()
            .await
            .map_err(anyhow::Error::msg)?,
    );
    let device = if preferred_device.is_empty() {
        bs::decode_c_string_bytes(&block_proxy.device().await.map_err(anyhow::Error::msg)?)
    } else {
        preferred_device
    };

    let mut device_path = if device.is_empty() {
        None
    } else {
        Some(device)
    };
    if device_path.is_none() {
        let proposed = format!(
            "/dev/{}",
            object_path.as_str().split('/').next_back().unwrap_or("")
        );
        if Path::new(&proposed).exists() {
            device_path = Some(proposed);
        }
    }

    let (has_filesystem, mount_points) = match FilesystemProxy::builder(connection)
        .path(&object_path)?
        .build()
        .await
    {
        Ok(proxy) => match proxy.mount_points().await {
            Ok(mps) => (true, bs::decode_mount_points(mps)),
            Err(_) => (false, Vec::new()),
        },
        Err(_) => (false, Vec::new()),
    };

    let usage = match mount_points.first() {
        Some(mount_point) => crate::usage_for_mount_point(mount_point, device_path.as_deref()).ok(),
        None => None,
    };

    let id_type = block_proxy.id_type().await.map_err(anyhow::Error::msg)?;
    let size = block_proxy.size().await.map_err(anyhow::Error::msg)?;

    // Get partition offset and number if this is a partition
    let (offset, partition_number) = match PartitionProxy::builder(connection)
        .path(&object_path)?
        .build()
        .await
    {
        Ok(partition_proxy) => (
            partition_proxy.offset().await.unwrap_or(0),
            partition_proxy.number().await.unwrap_or(0),
        ),
        Err(_) => (0, 0),
    };

    // Use partition number for label if this is a partition, otherwise use provided label
    let final_label = if partition_number > 0 {
        format!("Partition {}", partition_number)
    } else {
        label
    };

    Ok(storage_types::VolumeInfo {
        kind,
        label: final_label,
        size,
        offset,
        partition_number,
        id_type,
        device_path,
        parent_path: None,
        has_filesystem,
        mount_points,
        usage,
        locked: false,
        children: Vec::new(),
    })
}

/// Build VolumeInfo for a block object, including LVM children if applicable.
pub(crate) async fn from_block_object(
    connection: &Connection,
    object_path: OwnedObjectPath,
    label: String,
    kind: VolumeKind,
    parent_path: Option<String>,
    block_index: Option<&BlockIndex>,
) -> Result<storage_types::VolumeInfo> {
    let mut info = probe_basic_block(connection, object_path, label, kind).await?;
    info.parent_path = parent_path;

    if info.kind == VolumeKind::LvmPhysicalVolume || info.id_type == "LVM2_member" {
        info.kind = VolumeKind::LvmPhysicalVolume;
        if let (Some(pv_device), Some(index)) = (info.device_path.as_deref(), block_index)
            && let Ok(lvs) = lvm::list_lvs_for_pv(pv_device)
        {
            let mut children = Vec::new();
            for lv in lvs {
                let lv_obj_path = match index.object_path_for_device(&lv.device_path) {
                    Some(p) => p,
                    None => continue,
                };
                let mut child = probe_basic_block(
                    connection,
                    lv_obj_path,
                    lv.display_name(),
                    VolumeKind::LvmLogicalVolume,
                )
                .await?;
                child.parent_path = info.device_path.clone();
                children.push(child);
            }
            info.children = children;
        }
    }

    Ok(info)
}

/// Build VolumeInfo for a LUKS partition (crypto container), with cleartext child if unlocked.
pub(crate) async fn crypto_container_for_partition(
    connection: &Connection,
    partition_object_path: OwnedObjectPath,
    label: String,
    parent_path: Option<String>,
    block_index: &BlockIndex,
) -> Result<storage_types::VolumeInfo> {
    let encrypted_proxy = EncryptedProxy::builder(connection)
        .path(&partition_object_path)?
        .build()
        .await?;

    let mut info = from_block_object(
        connection,
        partition_object_path.clone(),
        label,
        VolumeKind::CryptoContainer,
        parent_path,
        Some(block_index),
    )
    .await?;

    let cleartext = encrypted_proxy.cleartext_device().await?;
    let cleartext_str = cleartext.as_str();
    let unlocked = cleartext_str != "/";

    info.locked = !unlocked;

    if unlocked {
        let mut cleartext_info = from_block_object(
            connection,
            cleartext.clone(),
            String::new(),
            VolumeKind::Block,
            info.device_path.clone(),
            Some(block_index),
        )
        .await?;

        if cleartext_info.has_filesystem {
            cleartext_info.kind = VolumeKind::Filesystem;
            if cleartext_info.label.trim().is_empty() {
                cleartext_info.label = "Filesystem".to_string();
            }
        } else {
            if cleartext_info.label.trim().is_empty() {
                cleartext_info.label = "Cleartext".to_string();
            }
            if let Ok(pt) = PartitionTableProxy::builder(connection)
                .path(&cleartext)?
                .build()
                .await
                && let Ok(parts) = pt.partitions().await
            {
                for part_path in parts {
                    let part_label = part_path
                        .as_str()
                        .split('/')
                        .next_back()
                        .unwrap_or("Partition")
                        .replace('_', " ");
                    if let Ok(child) = from_block_object(
                        connection,
                        part_path.clone(),
                        part_label,
                        VolumeKind::Partition,
                        cleartext_info.device_path.clone(),
                        Some(block_index),
                    )
                    .await
                    {
                        cleartext_info.children.push(child);
                    }
                }
            }
        }

        info.children.push(cleartext_info);
    }

    Ok(info)
}

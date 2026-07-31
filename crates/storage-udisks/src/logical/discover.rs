//! Native logical-topology discovery.

use std::{
    collections::{BTreeMap, HashMap},
    num::{NonZeroU32, NonZeroU64},
};

use storage_contracts::StorageError;
use storage_types::{
    BlockDeviceRef, BtrfsFilesystemDetails, BtrfsMember, BtrfsMemberState, BtrfsPrimaryMember,
    BtrfsRelativePath, BtrfsSubvolumeDetails, BtrfsSubvolumeEntityDetails, BtrfsSubvolumeHierarchy,
    BtrfsSubvolumeRowKey, LogicalBlockedReason, LogicalCapabilities, LogicalDisplay, LogicalEntity,
    LogicalEntityDetails, LogicalEntityId, LogicalEntityKind, LogicalMember, LogicalMemberId,
    LogicalOperation, LvmActivationState, LvmLogicalVolumeDetails, LvmLogicalVolumeSummary,
    LvmPhysicalVolumeDetails, LvmPhysicalVolumeState, LvmPhysicalVolumeSummary,
    LvmVolumeGroupDetails, MdRaidArrayDetails, MdRaidHealth, MdRaidLevelName, MdRaidMemberDetails,
    MdRaidMemberRole, MdRaidMemberState, MountPointUsage, ProgressRatio,
};
use udisks2::{filesystem::FilesystemProxy, mdraid::MDRaidProxy};
use uuid::Uuid;

use crate::DiskManager;

use super::{
    error::native_error,
    proxy::{BtrfsProxy, LogicalVolumeProxy, VolumeGroupProxy},
    resolve::{BTRFS_INTERFACE, MDRAID_INTERFACE, VOLUME_GROUP_INTERFACE, blocks, managed_paths},
};

pub(crate) async fn entities(
    manager: &DiskManager,
    observed_generation: u64,
) -> Result<Vec<LogicalEntity>, StorageError> {
    let paths = managed_paths(manager).await?;
    let blocks = blocks(manager).await?;
    let block_by_path: HashMap<_, _> = blocks
        .iter()
        .map(|block| (block.path.clone(), block.clone()))
        .collect();

    let mut output = Vec::new();
    output.extend(volume_groups(manager, &paths, &block_by_path, observed_generation).await?);
    output.extend(mdraid_arrays(manager, &paths, &block_by_path, observed_generation).await?);
    output.extend(btrfs_filesystems(manager, &paths, &block_by_path, observed_generation).await?);
    output.sort_by(|left, right| {
        left.display_name()
            .cmp(right.display_name())
            .then(left.id.cmp(&right.id))
    });
    output.dedup_by(|left, right| left.id == right.id);
    Ok(output)
}

async fn volume_groups(
    manager: &DiskManager,
    paths: &HashMap<zbus::zvariant::OwnedObjectPath, Vec<String>>,
    block_by_path: &HashMap<zbus::zvariant::OwnedObjectPath, super::resolve::ResolvedBlock>,
    generation: u64,
) -> Result<Vec<LogicalEntity>, StorageError> {
    let mut output = Vec::new();
    for (path, interfaces) in paths {
        if !interfaces
            .iter()
            .any(|interface| interface == VOLUME_GROUP_INTERFACE)
        {
            continue;
        }
        let proxy = VolumeGroupProxy::builder(manager.connection())
            .path(path)
            .map_err(native_error)?
            .build()
            .await
            .map_err(native_error)?;
        let uuid = proxy.uuid().await.map_err(native_error)?;
        if uuid.trim().is_empty() {
            continue;
        }
        let id = LogicalEntityId(format!("lvm-vg:{uuid}"));
        let name = proxy.name().await.unwrap_or_else(|_| uuid.clone());
        let size = proxy.size().await.unwrap_or(0);
        let free = proxy.free_size().await.unwrap_or(0);
        let logical_volumes = proxy.logical_volumes().await.unwrap_or_default();
        let physical_volumes = proxy.physical_volumes().await.unwrap_or_default();
        let mut members = Vec::new();
        let mut children = Vec::new();
        let mut logical_volume_summaries = Vec::new();
        let mut physical_volume_summaries = Vec::new();

        for lv_path in logical_volumes {
            let lv = LogicalVolumeProxy::builder(manager.connection())
                .path(&lv_path)
                .map_err(native_error)?
                .build()
                .await
                .map_err(native_error)?;
            let lv_name = lv
                .name()
                .await
                .unwrap_or_else(|_| "Logical volume".to_string());
            let lv_id = LogicalEntityId(format!("lvm-lv:{uuid}:{lv_name}"));
            let active = lv.active().await.unwrap_or(false);
            let lv_size = lv.size().await.unwrap_or(0);
            members.push(LogicalMember {
                id: LogicalMemberId(format!("member:{}", lv_id.0)),
                kind: LogicalEntityKind::LvmLogicalVolume,
                name: lv_name.clone(),
                device_ref: None,
                device_path: None,
                role: Some("lv".into()),
                state: Some(if active { "active" } else { "inactive" }.into()),
                size_bytes: Some(lv_size),
            });
            children.push(LogicalEntity {
                id: lv_id,
                kind: LogicalEntityKind::LvmLogicalVolume,
                details: LogicalEntityDetails::LvmLogicalVolume(LvmLogicalVolumeDetails {
                    name: lv_name.clone(),
                    volume_group: id.clone(),
                    device_path: LogicalDisplay::unknown(
                        "UDisks did not report the logical-volume device path",
                    ),
                    size: LogicalDisplay::known(lv_size),
                    activation: LogicalDisplay::known(if active {
                        LvmActivationState::Active
                    } else {
                        LvmActivationState::Inactive
                    }),
                }),
                parent_id: Some(id.clone()),
                capabilities: logical_volume_capabilities(active),
                metadata: BTreeMap::new(),
                name: lv_name,
                uuid: None,
                device_path: None,
                size_bytes: lv_size,
                used_bytes: None,
                free_bytes: None,
                health_status: Some(if active { "active" } else { "inactive" }.into()),
                progress_fraction: None,
                members: Vec::new(),
            });
            let child = children.last().expect("logical volume was just pushed");
            logical_volume_summaries.push(LvmLogicalVolumeSummary {
                entity_id: child.id.clone(),
                name: child.display_name().to_string(),
                size: LogicalDisplay::known(lv_size),
                activation: LogicalDisplay::known(if active {
                    LvmActivationState::Active
                } else {
                    LvmActivationState::Inactive
                }),
            });
        }

        for pv_path in physical_volumes {
            let Some(block) = block_by_path.get(&pv_path) else {
                continue;
            };
            let device_ref = block.fingerprint.as_ref().map(|fingerprint| {
                BlockDeviceRef::new(block.id.clone(), fingerprint.clone(), generation)
            });
            let pv_id = LogicalEntityId(format!(
                "lvm-pv:{uuid}:{}",
                device_ref
                    .as_ref()
                    .map(|reference| reference.fingerprint.as_str())
                    .unwrap_or(block.id.as_str())
            ));
            members.push(LogicalMember {
                id: LogicalMemberId(format!("member:{}", pv_id.0)),
                kind: LogicalEntityKind::LvmPhysicalVolume,
                name: block.device_path.clone(),
                device_ref: device_ref.clone(),
                device_path: Some(block.device_path.clone()),
                role: Some("pv".into()),
                state: None,
                size_bytes: None,
            });
            children.push(LogicalEntity {
                id: pv_id,
                kind: LogicalEntityKind::LvmPhysicalVolume,
                details: LogicalEntityDetails::LvmPhysicalVolume(LvmPhysicalVolumeDetails {
                    block: device_ref.clone(),
                    display_path: LogicalDisplay::known(block.device_path.clone()),
                    size: LogicalDisplay::unknown("UDisks did not report the physical-volume size"),
                    state: LogicalDisplay::known(LvmPhysicalVolumeState::Available),
                }),
                parent_id: Some(id.clone()),
                capabilities: LogicalCapabilities::normalized(
                    vec![LogicalOperation::RemoveMember],
                    Vec::new(),
                ),
                metadata: BTreeMap::new(),
                name: block.device_path.clone(),
                uuid: None,
                device_path: Some(block.device_path.clone()),
                size_bytes: 0,
                used_bytes: None,
                free_bytes: None,
                health_status: None,
                progress_fraction: None,
                members: Vec::new(),
            });
            let child = children.last().expect("physical volume was just pushed");
            physical_volume_summaries.push(LvmPhysicalVolumeSummary {
                entity_id: child.id.clone(),
                member: LvmPhysicalVolumeDetails {
                    block: device_ref,
                    display_path: LogicalDisplay::known(block.device_path.clone()),
                    size: LogicalDisplay::unknown("UDisks did not report the physical-volume size"),
                    state: LogicalDisplay::known(LvmPhysicalVolumeState::Available),
                },
            });
        }

        output.push(LogicalEntity {
            id,
            kind: LogicalEntityKind::LvmVolumeGroup,
            details: LogicalEntityDetails::LvmVolumeGroup(LvmVolumeGroupDetails {
                name: name.clone(),
                uuid: LogicalDisplay::known(uuid.clone()),
                size: LogicalDisplay::known(size),
                used: LogicalDisplay::known(size.saturating_sub(free)),
                free: LogicalDisplay::known(free),
                logical_volumes: logical_volume_summaries,
                physical_volumes: physical_volume_summaries,
            }),
            parent_id: None,
            capabilities: LogicalCapabilities::normalized(
                vec![
                    LogicalOperation::Delete,
                    LogicalOperation::AddMember,
                    LogicalOperation::RemoveMember,
                    LogicalOperation::Create,
                ],
                Vec::new(),
            ),
            metadata: BTreeMap::new(),
            name,
            uuid: Some(uuid),
            device_path: None,
            size_bytes: size,
            used_bytes: Some(size.saturating_sub(free)),
            free_bytes: Some(free),
            health_status: None,
            progress_fraction: None,
            members,
        });
        output.extend(children);
    }
    Ok(output)
}

async fn mdraid_arrays(
    manager: &DiskManager,
    paths: &HashMap<zbus::zvariant::OwnedObjectPath, Vec<String>>,
    block_by_path: &HashMap<zbus::zvariant::OwnedObjectPath, super::resolve::ResolvedBlock>,
    generation: u64,
) -> Result<Vec<LogicalEntity>, StorageError> {
    let mut output = Vec::new();
    for (path, interfaces) in paths {
        if !interfaces
            .iter()
            .any(|interface| interface == MDRAID_INTERFACE)
        {
            continue;
        }
        let proxy = MDRaidProxy::builder(manager.connection())
            .path(path)
            .map_err(native_error)?
            .build()
            .await
            .map_err(native_error)?;
        let uuid = proxy.uuid().await.unwrap_or_default();
        if uuid.trim().is_empty() {
            continue;
        }
        let id = LogicalEntityId(format!("mdraid:{uuid}"));
        let running = proxy.running().await.unwrap_or(false);
        let degraded = proxy.degraded().await.unwrap_or(0) > 0;
        let mut members = Vec::new();
        let mut children = Vec::new();
        let mut typed_members = Vec::new();
        for (member_path, _slot, states, member_size, _) in
            proxy.active_devices().await.unwrap_or_default()
        {
            let Some(block) = block_by_path.get(&member_path) else {
                continue;
            };
            let device_ref = block.fingerprint.as_ref().map(|fingerprint| {
                BlockDeviceRef::new(block.id.clone(), fingerprint.clone(), generation)
            });
            let member_id = LogicalEntityId(format!(
                "mdraid-member:{}",
                device_ref
                    .as_ref()
                    .map(|reference| reference.fingerprint.as_str())
                    .unwrap_or(block.id.as_str())
            ));
            let mut labels = states;
            labels.sort();
            labels.dedup();
            let state = labels.join(",");
            let typed_member = MdRaidMemberDetails {
                member_id: LogicalMemberId(format!("member:{}", member_id.0)),
                block: device_ref.clone(),
                display_path: LogicalDisplay::known(block.device_path.clone()),
                size: LogicalDisplay::known(member_size),
                role: LogicalDisplay::known(MdRaidMemberRole::Active),
                state: LogicalDisplay::known(MdRaidMemberState::Native { labels }),
            };
            members.push(LogicalMember {
                id: typed_member.member_id.clone(),
                kind: LogicalEntityKind::MdRaidMember,
                name: block.device_path.clone(),
                device_ref: device_ref.clone(),
                device_path: Some(block.device_path.clone()),
                role: Some("member".into()),
                state: Some(state.clone()),
                size_bytes: Some(member_size),
            });
            children.push(LogicalEntity {
                id: member_id,
                kind: LogicalEntityKind::MdRaidMember,
                details: LogicalEntityDetails::MdRaidMember(typed_member.clone()),
                parent_id: Some(id.clone()),
                capabilities: LogicalCapabilities::default(),
                metadata: BTreeMap::new(),
                name: block.device_path.clone(),
                uuid: None,
                device_path: Some(block.device_path.clone()),
                size_bytes: member_size,
                used_bytes: None,
                free_bytes: None,
                health_status: Some(state),
                progress_fraction: None,
                members: Vec::new(),
            });
            typed_members.push(typed_member);
        }
        let array_name = proxy.name().await.unwrap_or_else(|_| uuid.clone());
        let level = proxy
            .level()
            .await
            .ok()
            .and_then(|value| MdRaidLevelName::new(value).ok())
            .map(LogicalDisplay::known)
            .unwrap_or_else(|| {
                LogicalDisplay::unknown("UDisks did not report a valid MD RAID level")
            });
        let size = proxy.size().await.ok();
        let sync_progress = proxy
            .sync_completed()
            .await
            .ok()
            .map(ProgressRatio::from_fraction)
            .map(LogicalDisplay::known)
            .unwrap_or_else(|| {
                LogicalDisplay::unknown("UDisks did not report MD RAID sync progress")
            });
        output.push(LogicalEntity {
            id,
            kind: LogicalEntityKind::MdRaidArray,
            details: LogicalEntityDetails::MdRaidArray(MdRaidArrayDetails {
                name: array_name.clone(),
                uuid: LogicalDisplay::known(uuid.clone()),
                level,
                size: size.map(LogicalDisplay::known).unwrap_or_else(|| {
                    LogicalDisplay::unknown("UDisks did not report the MD RAID size")
                }),
                running: LogicalDisplay::known(running),
                health: LogicalDisplay::known(if degraded {
                    MdRaidHealth::Degraded
                } else {
                    MdRaidHealth::Healthy
                }),
                sync_progress,
                members: typed_members,
            }),
            parent_id: None,
            capabilities: mdraid_capabilities(running),
            metadata: BTreeMap::new(),
            name: array_name,
            uuid: Some(uuid),
            device_path: None,
            size_bytes: size.unwrap_or_default(),
            used_bytes: None,
            free_bytes: None,
            health_status: Some(if degraded { "degraded" } else { "ok" }.into()),
            progress_fraction: proxy
                .sync_completed()
                .await
                .ok()
                .map(ProgressRatio::from_fraction),
            members,
        });
        output.extend(children);
    }
    Ok(output)
}

async fn btrfs_filesystems(
    manager: &DiskManager,
    paths: &HashMap<zbus::zvariant::OwnedObjectPath, Vec<String>>,
    block_by_path: &HashMap<zbus::zvariant::OwnedObjectPath, super::resolve::ResolvedBlock>,
    generation: u64,
) -> Result<Vec<LogicalEntity>, StorageError> {
    let mut groups: BTreeMap<
        Uuid,
        Vec<(
            &zbus::zvariant::OwnedObjectPath,
            &super::resolve::ResolvedBlock,
        )>,
    > = BTreeMap::new();
    for (path, interfaces) in paths {
        if !interfaces
            .iter()
            .any(|interface| interface == BTRFS_INTERFACE)
        {
            continue;
        }
        let Some(block) = block_by_path.get(path) else {
            continue;
        };
        // IdUUID groups Btrfs members only.  It is deliberately not reused as
        // a per-device fingerprint.
        let Ok(filesystem_uuid) = Uuid::parse_str(block.id_uuid.trim()) else {
            continue;
        };
        groups
            .entry(filesystem_uuid)
            .or_default()
            .push((path, block));
    }

    let mut output = Vec::new();
    for (filesystem_uuid, mut group) in groups {
        group.sort_by(|(left_path, left), (right_path, right)| {
            left.id
                .major_minor()
                .cmp(&right.id.major_minor())
                .then(left.fingerprint.cmp(&right.fingerprint))
                .then(
                    left_path
                        .as_str()
                        .as_bytes()
                        .cmp(right_path.as_str().as_bytes()),
                )
        });
        let (primary_path, primary_block) = group
            .first()
            .expect("a Btrfs group has at least one member");
        let root_id = LogicalEntityId(format!("btrfs:{filesystem_uuid}"));
        let mut diagnostics = Vec::new();
        let proxy = BtrfsProxy::builder(manager.connection())
            .path(*primary_path)
            .map_err(native_error)?
            .build()
            .await;

        let mut members = Vec::new();
        for (_path, block) in &group {
            let member_id = LogicalMemberId(format!("btrfs-member:{}", block.id));
            let state = match block.read_only {
                Some(false) => BtrfsMemberState::Writable,
                Some(true) => BtrfsMemberState::ReadOnly,
                None => BtrfsMemberState::Unknown {
                    reason: "UDisks did not report Block.ReadOnly".into(),
                },
            };
            members.push(BtrfsMember {
                member_id,
                block: block.fingerprint.as_ref().map(|fingerprint| {
                    BlockDeviceRef::new(block.id.clone(), fingerprint.clone(), generation)
                }),
                display_path: if block.device_path.is_empty() {
                    LogicalDisplay::unknown("UDisks did not report the Btrfs member path")
                } else {
                    LogicalDisplay::known(block.device_path.clone())
                },
                label: if block.id_label.trim().is_empty() {
                    LogicalDisplay::unknown("UDisks did not report Block.IdLabel")
                } else {
                    LogicalDisplay::known(block.id_label.clone())
                },
                size: block
                    .size
                    .map(LogicalDisplay::known)
                    .unwrap_or_else(|| LogicalDisplay::unknown("UDisks did not report Block.Size")),
                state,
            });
        }
        let primary_member_id = members
            .first()
            .expect("Btrfs members were created from the nonempty group")
            .member_id
            .clone();

        let (default_subvolume, subvolumes, native_available) = match proxy {
            Ok(proxy) => {
                let default_subvolume = match proxy.get_default_subvolume_id(HashMap::new()).await {
                    Ok(0) => LogicalDisplay::known(None),
                    Ok(value) => match NonZeroU64::new(u64::from(value))
                        .filter(|value| value.get() <= u64::from(u32::MAX))
                    {
                        Some(value) => LogicalDisplay::known(Some(value)),
                        None => LogicalDisplay::unknown(
                            "UDisks returned an invalid default subvolume ID",
                        ),
                    },
                    Err(error) => LogicalDisplay::unknown(format!(
                        "GetDefaultSubvolumeID unavailable: {error}"
                    )),
                };
                let subvolumes = match proxy.get_subvolumes(false, HashMap::new()).await {
                    Ok((rows, _)) => {
                        let (subvolumes, topology_diagnostics) = map_btrfs_subvolumes(
                            rows,
                            default_subvolume.as_known().and_then(|id| *id),
                        );
                        diagnostics.extend(topology_diagnostics);
                        subvolumes
                    }
                    Err(error) => {
                        diagnostics.push(
                            storage_types::BtrfsTopologyDiagnostic::PrimaryUnavailable {
                                member_id: Some(primary_member_id.clone()),
                                reason: format!("GetSubvolumes unavailable: {error}"),
                            },
                        );
                        Vec::new()
                    }
                };
                (default_subvolume, subvolumes, true)
            }
            Err(error) => {
                diagnostics.push(storage_types::BtrfsTopologyDiagnostic::PrimaryUnavailable {
                    member_id: Some(primary_member_id.clone()),
                    reason: format!("UDisks Btrfs primary unavailable: {error}"),
                });
                (
                    LogicalDisplay::unknown("UDisks Btrfs primary is unavailable"),
                    Vec::new(),
                    false,
                )
            }
        };
        let primary_member = if native_available {
            BtrfsPrimaryMember::Selected {
                member_id: primary_member_id,
            }
        } else {
            BtrfsPrimaryMember::Unavailable {
                reason: "UDisks Btrfs primary is unavailable".into(),
            }
        };
        let capabilities = if native_available {
            LogicalCapabilities::normalized(
                vec![
                    LogicalOperation::Create,
                    LogicalOperation::Delete,
                    LogicalOperation::AddMember,
                    LogicalOperation::RemoveMember,
                    LogicalOperation::Resize,
                    LogicalOperation::SetLabel,
                    LogicalOperation::SetDefaultSubvolume,
                ],
                Vec::new(),
            )
        } else {
            LogicalCapabilities::block_all(
                "UDisks Btrfs support is unavailable for this filesystem.",
            )
        };
        let details = BtrfsFilesystemDetails {
            filesystem_uuid,
            label: LogicalDisplay::unknown(
                "UDisks has no audited filesystem-wide Btrfs label source",
            ),
            allocation: LogicalDisplay::unknown(
                "UDisks has no audited filesystem-wide Btrfs allocation source",
            ),
            mount_usage: btrfs_mount_usage(manager, primary_path, &primary_block.device_path).await,
            default_subvolume,
            primary_member,
            members: members.clone(),
            subvolumes: subvolumes.clone(),
            diagnostics,
        };
        let legacy_name = primary_block.device_path.clone();
        output.push(LogicalEntity {
            id: root_id.clone(),
            kind: LogicalEntityKind::BtrfsFilesystem,
            details: LogicalEntityDetails::BtrfsFilesystem(details),
            parent_id: None,
            capabilities: capabilities.clone(),
            metadata: BTreeMap::new(),
            name: if legacy_name.is_empty() {
                filesystem_uuid.to_string()
            } else {
                legacy_name.clone()
            },
            uuid: Some(filesystem_uuid.to_string()),
            device_path: (!legacy_name.is_empty()).then_some(legacy_name),
            size_bytes: 0,
            used_bytes: None,
            free_bytes: None,
            health_status: None,
            progress_fraction: None,
            members: Vec::new(),
        });
        for member in members {
            output.push(LogicalEntity {
                id: LogicalEntityId(format!(
                    "btrfs-device:{filesystem_uuid}:{}",
                    member.member_id
                )),
                kind: LogicalEntityKind::BtrfsDevice,
                details: LogicalEntityDetails::BtrfsDevice(storage_types::BtrfsDeviceDetails {
                    filesystem: root_id.clone(),
                    member: member.clone(),
                }),
                parent_id: Some(root_id.clone()),
                capabilities: LogicalCapabilities::default(),
                metadata: BTreeMap::new(),
                name: member
                    .display_path
                    .as_known()
                    .cloned()
                    .unwrap_or_else(|| "Unknown Btrfs device".into()),
                uuid: None,
                device_path: member.display_path.as_known().cloned(),
                size_bytes: member.size.as_known().copied().unwrap_or_default(),
                used_bytes: None,
                free_bytes: None,
                health_status: None,
                progress_fraction: None,
                members: Vec::new(),
            });
        }
        // Btrfs exposes a subvolume's parent as another native subvolume ID,
        // not as a logical-entity ID.  Preserve that relationship when the
        // native parent is unique and the child passed hierarchy validation.
        // Roots whose parent is not part of this report remain direct children
        // of the filesystem so they are still visible without inventing a
        // parent.
        let mut subvolume_entities_by_native_id: BTreeMap<NonZeroU64, Vec<LogicalEntityId>> =
            BTreeMap::new();
        for subvolume in &subvolumes {
            subvolume_entities_by_native_id
                .entry(subvolume.id)
                .or_default()
                .push(btrfs_subvolume_entity_id(filesystem_uuid, subvolume));
        }
        for subvolume in subvolumes {
            let id = btrfs_subvolume_entity_id(filesystem_uuid, &subvolume);
            let parent_id = btrfs_subvolume_parent_entity_id(
                &root_id,
                &subvolume,
                &subvolume_entities_by_native_id,
            );
            output.push(LogicalEntity {
                id,
                kind: LogicalEntityKind::BtrfsSubvolume,
                details: LogicalEntityDetails::BtrfsSubvolume(BtrfsSubvolumeEntityDetails {
                    filesystem: root_id.clone(),
                    subvolume: subvolume.clone(),
                }),
                parent_id: Some(parent_id),
                capabilities: LogicalCapabilities::default(),
                metadata: BTreeMap::new(),
                name: subvolume.relative_path.to_string(),
                uuid: None,
                device_path: None,
                size_bytes: 0,
                used_bytes: None,
                free_bytes: None,
                health_status: None,
                progress_fraction: None,
                members: Vec::new(),
            });
        }
    }
    Ok(output)
}

fn btrfs_subvolume_entity_id(
    filesystem_uuid: Uuid,
    subvolume: &BtrfsSubvolumeDetails,
) -> LogicalEntityId {
    LogicalEntityId(format!(
        "btrfs-subvolume:{filesystem_uuid}:{}:{}",
        subvolume.id, subvolume.row_key.occurrence
    ))
}

fn btrfs_subvolume_parent_entity_id(
    filesystem_id: &LogicalEntityId,
    subvolume: &BtrfsSubvolumeDetails,
    entities_by_native_id: &BTreeMap<NonZeroU64, Vec<LogicalEntityId>>,
) -> LogicalEntityId {
    if matches!(subvolume.hierarchy, BtrfsSubvolumeHierarchy::Attached)
        && let Some(parent_id) = subvolume.parent_id
        && let Some(parents) = entities_by_native_id.get(&parent_id)
        && let [parent] = parents.as_slice()
    {
        return parent.clone();
    }

    filesystem_id.clone()
}

/// Btrfs allocation and mount-point usage are deliberately distinct. The
/// former has no audited filesystem-wide UDisks source, while this read-only
/// statvfs value describes the first UDisks-reported mounted view of the
/// selected Btrfs member (normally `/` for the system mapping).
async fn btrfs_mount_usage(
    manager: &DiskManager,
    block_path: &zbus::zvariant::OwnedObjectPath,
    device_path: &str,
) -> Option<MountPointUsage> {
    let filesystem = FilesystemProxy::builder(manager.connection())
        .path(block_path)
        .ok()?
        .build()
        .await
        .ok()?;
    let mount_point =
        crate::dbus::bytestring::decode_mount_points(filesystem.mount_points().await.ok()?)
            .into_iter()
            .next()?;
    let usage = crate::usage_for_mount_point(&mount_point, Some(device_path)).ok()?;
    Some(MountPointUsage {
        mount_point,
        total: usage.blocks,
        used: usage.used,
        free: usage.available,
    })
}

fn map_btrfs_subvolumes(
    mut rows: Vec<(u64, u64, String)>,
    default_id: Option<NonZeroU64>,
) -> (
    Vec<BtrfsSubvolumeDetails>,
    Vec<storage_types::BtrfsTopologyDiagnostic>,
) {
    rows.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then(left.2.as_bytes().cmp(right.2.as_bytes()))
            .then(left.1.cmp(&right.1))
    });
    let mut occurrences: BTreeMap<(NonZeroU64, BtrfsRelativePath), u32> = BTreeMap::new();
    let mut output = Vec::new();
    for (id, parent_id, path) in rows {
        let (Some(id), Ok(relative_path)) = (NonZeroU64::new(id), BtrfsRelativePath::new(path))
        else {
            continue;
        };
        let occurrence = occurrences.entry((id, relative_path.clone())).or_default();
        *occurrence += 1;
        let occurrence = NonZeroU32::new(*occurrence).expect("occurrences start at one");
        output.push(BtrfsSubvolumeDetails {
            row_key: BtrfsSubvolumeRowKey {
                id,
                relative_path: relative_path.clone(),
                occurrence,
            },
            id,
            relative_path,
            parent_id: NonZeroU64::new(parent_id),
            hierarchy: BtrfsSubvolumeHierarchy::Attached,
            is_default: default_id == Some(id),
        });
    }
    let mut diagnostics = Vec::new();
    let mut by_id: BTreeMap<NonZeroU64, Vec<usize>> = BTreeMap::new();
    let mut by_path: BTreeMap<BtrfsRelativePath, Vec<usize>> = BTreeMap::new();
    for (index, subvolume) in output.iter().enumerate() {
        by_id.entry(subvolume.id).or_default().push(index);
        by_path
            .entry(subvolume.relative_path.clone())
            .or_default()
            .push(index);
    }
    for (id, indexes) in by_id.iter().filter(|(_, indexes)| indexes.len() > 1) {
        let diagnostic = storage_types::BtrfsTopologyDiagnostic::DuplicateSubvolumeId {
            id: *id,
            rows: indexes
                .iter()
                .map(|index| output[*index].row_key.clone())
                .collect(),
        };
        mark_unparented(&mut output, indexes, diagnostic.clone());
        diagnostics.push(diagnostic);
    }
    for (path, indexes) in by_path.iter().filter(|(_, indexes)| indexes.len() > 1) {
        let diagnostic = storage_types::BtrfsTopologyDiagnostic::DuplicateRelativePath {
            path: path.clone(),
            rows: indexes
                .iter()
                .map(|index| output[*index].row_key.clone())
                .collect(),
        };
        mark_unparented(&mut output, indexes, diagnostic.clone());
        diagnostics.push(diagnostic);
    }
    for index in 0..output.len() {
        let subvolume = &output[index];
        if subvolume.parent_id == Some(subvolume.id) {
            let diagnostic = storage_types::BtrfsTopologyDiagnostic::SelfParent {
                row: subvolume.row_key.clone(),
                id: subvolume.id,
            };
            mark_unparented(&mut output, &[index], diagnostic.clone());
            diagnostics.push(diagnostic);
        } else if let Some(parent_id) = subvolume.parent_id
            && !by_id.contains_key(&parent_id)
        {
            let diagnostic = storage_types::BtrfsTopologyDiagnostic::MissingParent {
                row: subvolume.row_key.clone(),
                parent_id,
            };
            mark_unparented(&mut output, &[index], diagnostic.clone());
            diagnostics.push(diagnostic);
        }
    }
    // A parent is traversed only when its ID is unique. Duplicate IDs cannot
    // be resolved by arbitrarily selecting one native row.
    for start in 0..output.len() {
        let mut order: Vec<usize> = Vec::new();
        let mut cursor = start;
        while let Some(parent_id) = output[cursor].parent_id {
            if by_id
                .get(&output[cursor].id)
                .is_none_or(|rows| rows.len() != 1)
                || by_id.get(&parent_id).is_none_or(|rows| rows.len() != 1)
            {
                break;
            }
            if let Some(position) = order.iter().position(|index| *index == cursor) {
                let cycle = order[position..].to_vec();
                let diagnostic = storage_types::BtrfsTopologyDiagnostic::Cycle {
                    rows: cycle
                        .iter()
                        .map(|index| output[*index].row_key.clone())
                        .collect(),
                };
                mark_unparented(&mut output, &cycle, diagnostic.clone());
                if !diagnostics.contains(&diagnostic) {
                    diagnostics.push(diagnostic);
                }
                break;
            }
            order.push(cursor);
            cursor = by_id[&parent_id][0];
        }
    }
    (output, diagnostics)
}

fn mark_unparented(
    rows: &mut [BtrfsSubvolumeDetails],
    indexes: &[usize],
    diagnostic: storage_types::BtrfsTopologyDiagnostic,
) {
    for index in indexes {
        match &mut rows[*index].hierarchy {
            BtrfsSubvolumeHierarchy::Attached => {
                rows[*index].hierarchy = BtrfsSubvolumeHierarchy::Unparented {
                    diagnostics: vec![diagnostic.clone()],
                };
            }
            BtrfsSubvolumeHierarchy::Unparented { diagnostics } => {
                if !diagnostics.contains(&diagnostic) {
                    diagnostics.push(diagnostic.clone());
                }
            }
        }
    }
}

fn logical_volume_capabilities(active: bool) -> LogicalCapabilities {
    let mut supported = vec![LogicalOperation::Delete, LogicalOperation::Resize];
    supported.push(if active {
        LogicalOperation::Deactivate
    } else {
        LogicalOperation::Activate
    });
    LogicalCapabilities::normalized(supported, Vec::new())
}

fn mdraid_capabilities(running: bool) -> LogicalCapabilities {
    let mut supported = vec![
        LogicalOperation::Delete,
        LogicalOperation::AddMember,
        LogicalOperation::RemoveMember,
        LogicalOperation::Check,
        LogicalOperation::Repair,
    ];
    supported.push(if running {
        LogicalOperation::Stop
    } else {
        LogicalOperation::Start
    });
    LogicalCapabilities::normalized(supported, Vec::<LogicalBlockedReason>::new())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        btrfs_subvolume_entity_id, btrfs_subvolume_parent_entity_id, map_btrfs_subvolumes,
    };
    use storage_types::{BtrfsSubvolumeHierarchy, LogicalEntityId};
    use uuid::Uuid;

    #[test]
    fn btrfs_subvolume_mapping_is_order_independent_and_retains_conflicts() {
        let rows = vec![
            (7, 5, "home/snapshot".into()),
            (5, 0, "home".into()),
            (7, 5, "home/snapshot-copy".into()),
            (9, 99, "orphan".into()),
        ];
        let (first, first_diagnostics) = map_btrfs_subvolumes(rows.clone(), None);
        let (second, second_diagnostics) =
            map_btrfs_subvolumes(rows.into_iter().rev().collect(), None);
        assert_eq!(first, second);
        assert_eq!(first_diagnostics, second_diagnostics);
        assert_eq!(first.len(), 4);
        assert!(first.iter().any(|row| {
            row.relative_path.as_str() == "orphan"
                && matches!(row.hierarchy, BtrfsSubvolumeHierarchy::Unparented { .. })
        }));
        assert_eq!(
            first
                .iter()
                .filter(|row| matches!(row.hierarchy, BtrfsSubvolumeHierarchy::Unparented { .. }))
                .count(),
            3
        );
    }

    #[test]
    fn attached_subvolume_is_parented_by_its_unique_native_parent() {
        let filesystem_uuid = Uuid::nil();
        let (subvolumes, _) = map_btrfs_subvolumes(
            vec![(256, 5, "@".into()), (262, 256, "@/.snapshots".into())],
            None,
        );
        let mut entities_by_native_id = BTreeMap::new();
        for subvolume in &subvolumes {
            entities_by_native_id
                .entry(subvolume.id)
                .or_insert_with(Vec::new)
                .push(btrfs_subvolume_entity_id(filesystem_uuid, subvolume));
        }
        let filesystem = LogicalEntityId("btrfs:00000000-0000-0000-0000-000000000000".into());
        let root = subvolumes
            .iter()
            .find(|subvolume| subvolume.relative_path.as_str() == "@")
            .expect("root subvolume is present");
        let snapshots = subvolumes
            .iter()
            .find(|subvolume| subvolume.relative_path.as_str() == "@/.snapshots")
            .expect("snapshot parent is present");

        // The native root's parent ID (5) is not part of this listing, so it
        // is placed directly below the filesystem. Its child remains safely
        // attached to that root instead of being flattened beside it.
        assert_eq!(
            btrfs_subvolume_parent_entity_id(&filesystem, root, &entities_by_native_id),
            filesystem
        );
        assert_eq!(
            btrfs_subvolume_parent_entity_id(&filesystem, snapshots, &entities_by_native_id),
            btrfs_subvolume_entity_id(filesystem_uuid, root)
        );
    }
}

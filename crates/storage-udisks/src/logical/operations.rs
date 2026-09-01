//! Typed logical-action dispatch to native UDisks methods.

use std::collections::{BTreeSet, HashMap};

use async_trait::async_trait;
use storage_contracts::{
    BtrfsResizeRequest, ByteSizeConstraint, CandidateBlockReason, ConfigurationCleanupPolicy,
    ConfirmedLogicalAction, DestructiveScopePolicy, LogicalAction, LogicalActionKind,
    LogicalActionOutcome, LogicalCandidateDisplay, LogicalDeviceCandidate, LogicalInputConstraints,
    LogicalOperations, LogicalPreflight, LogicalPreflightAvailability, LogicalPreflightRequest,
    LogicalPreflightTarget, LogicalReviewData, LvmWipePolicy, MdRaidCreateProfile, MdRaidLevel,
    MdRaidProfileOption, MdRaidSyncAction, StorageError, StorageErrorKind,
};
use storage_types::{
    BlockDeviceRef, BtrfsSubvolumeRef, ConfirmedDestructiveScope, LogicalCandidateAnchor,
    LogicalCandidateKind, LogicalDisplay, LogicalEntityDetails, LogicalEntityId,
};
use udisks2::{manager::ManagerProxy, mdraid::MDRaidProxy};
use zbus::zvariant::{ObjectPath, OwnedObjectPath, Value};

use crate::UdisksBackend;

use super::{
    error::{conflict, native_error, unsupported},
    proxy::{BtrfsProxy, LogicalVolumeProxy, LvmManagerProxy, VolumeGroupProxy},
    resolve::{
        BTRFS_INTERFACE, MDRAID_INTERFACE, VOLUME_GROUP_INTERFACE, blocks, managed_paths,
        resolve_block,
    },
};

type Options = HashMap<&'static str, Value<'static>>;

#[async_trait]
impl LogicalOperations for UdisksBackend {
    async fn capture_logical_candidate(
        &self,
        display_path: String,
    ) -> Result<LogicalCandidateAnchor, StorageError> {
        let epoch = self.logical_epoch();
        let blocks = blocks(self.manager()).await?;
        let block = blocks
            .iter()
            .find(|block| block.device_path == display_path)
            .ok_or_else(|| {
                StorageError::new(
                    StorageErrorKind::NotFound,
                    "The selected physical device is no longer present.",
                )
            })?;
        let topology = super::discover::entities(self.manager(), epoch)
            .await
            .unwrap_or_default();
        let kind = topology
            .iter()
            .find_map(|entity| match &entity.details {
                LogicalEntityDetails::BtrfsFilesystem(details)
                    if details.members.iter().any(|member| {
                        member
                            .block
                            .as_ref()
                            .is_some_and(|reference| reference.id == block.id)
                    }) =>
                {
                    Some(LogicalCandidateKind::Btrfs)
                }
                LogicalEntityDetails::LvmVolumeGroup(details)
                    if details.physical_volumes.iter().any(|member| {
                        member
                            .member
                            .block
                            .as_ref()
                            .is_some_and(|reference| reference.id == block.id)
                    }) =>
                {
                    Some(LogicalCandidateKind::LvmPhysicalVolume)
                }
                LogicalEntityDetails::MdRaidArray(details)
                    if details.members.iter().any(|member| {
                        member
                            .block
                            .as_ref()
                            .is_some_and(|reference| reference.id == block.id)
                    }) =>
                {
                    Some(LogicalCandidateKind::RaidMember)
                }
                _ => None,
            })
            .or_else(|| {
                block
                    .id_type
                    .eq_ignore_ascii_case("btrfs")
                    .then_some(LogicalCandidateKind::Btrfs)
            })
            .ok_or_else(|| {
                StorageError::new(
                    StorageErrorKind::NotFound,
                    "The selected device is not currently a logical-storage candidate.",
                )
            })?;
        Ok(LogicalCandidateAnchor {
            kind,
            block_id: block.id.clone(),
            fingerprint: block.fingerprint.clone(),
            observed_epoch: epoch,
            display_path,
        })
    }

    async fn preflight_logical_action(
        &self,
        request: LogicalPreflightRequest,
    ) -> Result<LogicalPreflight, StorageError> {
        let epoch = self.logical_epoch();
        let topology = super::discover::entities(self.manager(), epoch).await?;
        let availability = match &request.request_key.target {
            LogicalPreflightTarget::Landing => LogicalPreflightAvailability::Ready,
            LogicalPreflightTarget::Candidate(anchor) => {
                if anchor.observed_epoch != epoch && anchor.fingerprint.is_none() {
                    LogicalPreflightAvailability::Blocked {
                        reason:
                            "The selected device has no strong identity after the topology changed."
                                .into(),
                    }
                } else {
                    LogicalPreflightAvailability::Blocked {
                        reason: "Open the resolved logical filesystem before managing it.".into(),
                    }
                }
            }
            LogicalPreflightTarget::Root(root) => topology
                .iter()
                .find(|entity| &entity.id == root)
                .map(|entity| {
                    let operation = operation_for_kind(request.request_key.action_kind);
                    if entity.capabilities.is_allowed(operation) {
                        LogicalPreflightAvailability::Ready
                    } else {
                        LogicalPreflightAvailability::Blocked {
                            reason: entity
                                .capabilities
                                .blocked_reason(operation)
                                .unwrap_or(
                                    "This operation is unavailable for the selected logical item.",
                                )
                                .into(),
                        }
                    }
                })
                .unwrap_or_else(|| LogicalPreflightAvailability::Blocked {
                    reason: "This logical item is no longer present.".into(),
                }),
        };
        let candidates =
            preflight_candidates(self, epoch, &topology, &request.request_key.target).await?;
        let constraints = constraints_for(request.request_key.action_kind);
        let review = review_for_preflight(
            &topology,
            &request.request_key.target,
            request.request_key.action_kind,
        );
        Ok(LogicalPreflight {
            key: storage_contracts::LogicalPreflightKey {
                request_key: request.request_key,
                udisks_epoch: epoch,
            },
            availability,
            device_candidates: candidates,
            constraints,
            review,
        })
    }

    async fn execute_logical_action(
        &self,
        confirmed: ConfirmedLogicalAction,
    ) -> Result<LogicalActionOutcome, StorageError> {
        let action = confirmed.action;
        if confirmed.preflight_key.request_key.action_kind != action.kind() {
            return Err(conflict(
                "The confirmed action no longer matches its preflight.",
            ));
        }
        if confirmed.preflight_key.udisks_epoch != self.logical_epoch() {
            return Err(conflict(
                "Logical storage changed after review; refresh and review the action again.",
            ));
        }
        action.validate()?;
        let affected_entity_ids = action_affected_ids(&action);
        match &action {
            LogicalAction::CreateLvmVolumeGroup { name, devices } => {
                let blocks = resolve_fresh_blocks(self, devices).await?;
                for block in &blocks {
                    ensure_unstructured_candidate(block)?;
                }
                let manager = LvmManagerProxy::new(self.manager().connection())
                    .await
                    .map_err(native_error)?;
                let paths: Vec<ObjectPath<'_>> =
                    blocks.iter().map(|block| block.path.as_ref()).collect();
                manager
                    .volume_group_create(name, &paths, empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::DeleteLvmVolumeGroup {
                volume_group,
                confirmed_scope,
                ..
            } => {
                let path = find_volume_group(self, volume_group).await?;
                verify_vg_scope(self, &path, volume_group, confirmed_scope).await?;
                let proxy = VolumeGroupProxy::builder(self.manager().connection())
                    .path(path)
                    .map_err(native_error)?
                    .build()
                    .await
                    .map_err(native_error)?;
                proxy
                    .delete(preserve_delete_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::AddLvmPhysicalVolume {
                volume_group,
                device,
            } => {
                let path = find_volume_group(self, volume_group).await?;
                let device = resolve_fresh_block(self, device).await?;
                ensure_unstructured_candidate(&device)?;
                let proxy = VolumeGroupProxy::builder(self.manager().connection())
                    .path(path)
                    .map_err(native_error)?
                    .build()
                    .await
                    .map_err(native_error)?;
                proxy
                    .add_device(&device.path.as_ref(), empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::RemoveLvmPhysicalVolume {
                volume_group,
                device,
                ..
            } => {
                let path = find_volume_group(self, volume_group).await?;
                let device = resolve_fresh_block(self, device).await?;
                let proxy = VolumeGroupProxy::builder(self.manager().connection())
                    .path(path)
                    .map_err(native_error)?
                    .build()
                    .await
                    .map_err(native_error)?;
                proxy
                    .remove_device(&device.path.as_ref(), preserve_wipe_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::CreateLvmLogicalVolume {
                volume_group,
                name,
                size_bytes,
            } => {
                let path = find_volume_group(self, volume_group).await?;
                let proxy = VolumeGroupProxy::builder(self.manager().connection())
                    .path(path)
                    .map_err(native_error)?
                    .build()
                    .await
                    .map_err(native_error)?;
                proxy
                    .create_plain_volume(name, *size_bytes, empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::DeleteLvmLogicalVolume {
                logical_volume,
                confirmed_scope,
                ..
            } => {
                verify_empty_scope(logical_volume, confirmed_scope)?;
                let path = find_logical_volume(self, logical_volume).await?;
                let proxy = LogicalVolumeProxy::builder(self.manager().connection())
                    .path(path)
                    .map_err(native_error)?
                    .build()
                    .await
                    .map_err(native_error)?;
                proxy
                    .delete(preserve_teardown_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::ResizeLvmLogicalVolume {
                logical_volume,
                size_bytes,
            } => {
                let path = find_logical_volume(self, logical_volume).await?;
                let proxy = LogicalVolumeProxy::builder(self.manager().connection())
                    .path(path)
                    .map_err(native_error)?
                    .build()
                    .await
                    .map_err(native_error)?;
                proxy
                    .resize(*size_bytes, empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::ActivateLvmLogicalVolume { logical_volume } => {
                let path = find_logical_volume(self, logical_volume).await?;
                let proxy = LogicalVolumeProxy::builder(self.manager().connection())
                    .path(path)
                    .map_err(native_error)?
                    .build()
                    .await
                    .map_err(native_error)?;
                proxy
                    .activate(empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::DeactivateLvmLogicalVolume { logical_volume } => {
                let path = find_logical_volume(self, logical_volume).await?;
                let proxy = LogicalVolumeProxy::builder(self.manager().connection())
                    .path(path)
                    .map_err(native_error)?
                    .build()
                    .await
                    .map_err(native_error)?;
                proxy
                    .deactivate(empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::CreateMdRaidArray {
                name,
                level,
                devices,
                profile,
            } => {
                if profile.level != *level {
                    return Err(StorageError::new(
                        StorageErrorKind::InvalidInput,
                        "MD RAID profile does not match its level.",
                    ));
                }
                let blocks = resolve_fresh_blocks(self, devices).await?;
                for block in &blocks {
                    ensure_unstructured_candidate(block)?;
                }
                let manager = ManagerProxy::new(self.manager().connection())
                    .await
                    .map_err(native_error)?;
                let paths: Vec<ObjectPath<'_>> =
                    blocks.iter().map(|block| block.path.as_ref()).collect();
                manager
                    .mdraid_create(
                        &paths,
                        level.source_name(),
                        &name.0,
                        profile.chunk_bytes,
                        mdraid_profile_options(),
                    )
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::DeleteMdRaidArray {
                array,
                confirmed_scope,
                ..
            } => {
                let path = find_mdraid(self, array).await?;
                verify_mdraid_scope(self, &path, array, confirmed_scope).await?;
                let proxy = MDRaidProxy::builder(self.manager().connection())
                    .path(path)
                    .map_err(native_error)?
                    .build()
                    .await
                    .map_err(native_error)?;
                proxy
                    .delete(preserve_teardown_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::StartMdRaidArray { array } => {
                let proxy = mdraid_proxy(self, array).await?;
                proxy.start(empty_options()).await.map_err(native_error)?;
            }
            LogicalAction::StopMdRaidArray { array } => {
                let proxy = mdraid_proxy(self, array).await?;
                proxy.stop(empty_options()).await.map_err(native_error)?;
            }
            LogicalAction::AddMdRaidMember { array, device } => {
                let proxy = mdraid_proxy(self, array).await?;
                let device = resolve_fresh_block(self, device).await?;
                ensure_unstructured_candidate(&device)?;
                proxy
                    .add_device(&device.path.as_ref(), empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::RemoveMdRaidMember { array, device, .. } => {
                let proxy = mdraid_proxy(self, array).await?;
                let device = resolve_fresh_block(self, device).await?;
                proxy
                    .remove_device(&device.path.as_ref(), preserve_wipe_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::RequestMdRaidSync { array, action } => {
                let proxy = mdraid_proxy(self, array).await?;
                let command = match action {
                    MdRaidSyncAction::Check => "check",
                    MdRaidSyncAction::Repair => "repair",
                };
                proxy
                    .request_sync_action(command, empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::AddBtrfsDevice { filesystem, device } => {
                let proxy = btrfs_proxy(self, filesystem).await?;
                let device = resolve_fresh_block(self, device).await?;
                ensure_unstructured_candidate(&device)?;
                verify_not_btrfs_member(self, filesystem, &device.id).await?;
                proxy
                    .add_device(&device.path.as_ref(), empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::RemoveBtrfsDevice { filesystem, device } => {
                let proxy = btrfs_proxy(self, filesystem).await?;
                let device = resolve_fresh_block(self, device).await?;
                proxy
                    .remove_device(&device.path.as_ref(), empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::ResizeBtrfsFilesystem {
                filesystem,
                request,
            } => {
                let BtrfsResizeRequest::AbsoluteBytes(size) = request else {
                    return Err(unsupported(
                        "This Btrfs size syntax has no audited native UDisks mapping.",
                    ));
                };
                let proxy = btrfs_proxy(self, filesystem).await?;
                proxy
                    .resize(*size, empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::SetBtrfsLabel { filesystem, label } => {
                let proxy = btrfs_proxy(self, filesystem).await?;
                proxy
                    .set_label(label, empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::SetBtrfsDefaultSubvolume {
                filesystem,
                subvolume,
            } => {
                let proxy = btrfs_proxy(self, filesystem).await?;
                let _path = resolve_btrfs_subvolume(self, &proxy, filesystem, subvolume).await?;
                proxy
                    .set_default_subvolume_id(
                        u32::try_from(subvolume.id.get()).map_err(|_| {
                            StorageError::new(
                                StorageErrorKind::InvalidInput,
                                "Btrfs default subvolume ID exceeds the native UDisks range.",
                            )
                        })?,
                        empty_options(),
                    )
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::CreateBtrfsSubvolume { filesystem, name } => {
                let proxy = btrfs_proxy(self, filesystem).await?;
                proxy
                    .create_subvolume(name, empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::DeleteBtrfsSubvolume {
                filesystem,
                subvolume,
            } => {
                let proxy = btrfs_proxy(self, filesystem).await?;
                let path = resolve_btrfs_subvolume(self, &proxy, filesystem, subvolume).await?;
                proxy
                    .remove_subvolume(&path, empty_options())
                    .await
                    .map_err(native_error)?;
            }
            LogicalAction::CreateBtrfsSnapshot {
                filesystem,
                source,
                destination,
                readonly,
            } => {
                let proxy = btrfs_proxy(self, filesystem).await?;
                let source = resolve_btrfs_subvolume(self, &proxy, filesystem, source).await?;
                proxy
                    .create_snapshot(&source, destination, *readonly, empty_options())
                    .await
                    .map_err(native_error)?;
            }
        }
        Ok(LogicalActionOutcome {
            action,
            affected_entity_ids,
            native_job_id: None,
            progress: None,
        })
    }
}

fn operation_for_kind(kind: LogicalActionKind) -> storage_types::LogicalOperation {
    use storage_types::LogicalOperation;
    match kind {
        LogicalActionKind::CreateLvmVolumeGroup
        | LogicalActionKind::CreateLvmLogicalVolume
        | LogicalActionKind::CreateMdRaidArray
        | LogicalActionKind::CreateBtrfsSubvolume
        | LogicalActionKind::CreateBtrfsSnapshot => LogicalOperation::Create,
        LogicalActionKind::DeleteLvmVolumeGroup
        | LogicalActionKind::DeleteLvmLogicalVolume
        | LogicalActionKind::DeleteMdRaidArray
        | LogicalActionKind::DeleteBtrfsSubvolume => LogicalOperation::Delete,
        LogicalActionKind::ResizeLvmLogicalVolume | LogicalActionKind::ResizeBtrfsFilesystem => {
            LogicalOperation::Resize
        }
        LogicalActionKind::AddLvmPhysicalVolume
        | LogicalActionKind::AddMdRaidMember
        | LogicalActionKind::AddBtrfsDevice => LogicalOperation::AddMember,
        LogicalActionKind::RemoveLvmPhysicalVolume
        | LogicalActionKind::RemoveMdRaidMember
        | LogicalActionKind::RemoveBtrfsDevice => LogicalOperation::RemoveMember,
        LogicalActionKind::ActivateLvmLogicalVolume => LogicalOperation::Activate,
        LogicalActionKind::DeactivateLvmLogicalVolume => LogicalOperation::Deactivate,
        LogicalActionKind::StartMdRaidArray => LogicalOperation::Start,
        LogicalActionKind::StopMdRaidArray => LogicalOperation::Stop,
        LogicalActionKind::CheckMdRaidArray => LogicalOperation::Check,
        LogicalActionKind::RepairMdRaidArray => LogicalOperation::Repair,
        LogicalActionKind::SetBtrfsLabel => LogicalOperation::SetLabel,
        LogicalActionKind::SetBtrfsDefaultSubvolume => LogicalOperation::SetDefaultSubvolume,
    }
}

fn review_for_preflight(
    topology: &[storage_types::LogicalEntity],
    target: &LogicalPreflightTarget,
    kind: LogicalActionKind,
) -> LogicalReviewData {
    let LogicalPreflightTarget::Root(primary) = target else {
        return LogicalReviewData::None;
    };
    match kind {
        LogicalActionKind::DeleteLvmVolumeGroup => {
            let mut entity_ids = topology
                .iter()
                .filter(|entity| entity.parent_id.as_ref() == Some(primary))
                .filter(|entity| entity.kind == storage_types::LogicalEntityKind::LvmLogicalVolume)
                .map(|entity| entity.id.clone())
                .collect::<Vec<_>>();
            entity_ids.sort();
            let scope = ConfirmedDestructiveScope::new(entity_ids, Vec::new())
                .expect("logical-volume IDs are distinct and sorted");
            LogicalReviewData::DestructiveScope {
                primary: primary.clone(),
                scope,
                policy: DestructiveScopePolicy::DeleteLvmVolumeGroup {
                    pv_label_policy: LvmWipePolicy::Preserve,
                    configuration_policy: ConfigurationCleanupPolicy::Preserve,
                },
            }
        }
        LogicalActionKind::DeleteLvmLogicalVolume => LogicalReviewData::DestructiveScope {
            primary: primary.clone(),
            scope: ConfirmedDestructiveScope::new(Vec::new(), Vec::new())
                .expect("an empty destructive scope is valid"),
            policy: DestructiveScopePolicy::DeleteLvmLogicalVolume {
                configuration_policy: ConfigurationCleanupPolicy::Preserve,
            },
        },
        LogicalActionKind::DeleteMdRaidArray => {
            let mut device_refs = topology
                .iter()
                .find(|entity| &entity.id == primary)
                .and_then(|entity| match &entity.details {
                    LogicalEntityDetails::MdRaidArray(details) => Some(
                        details
                            .members
                            .iter()
                            .filter_map(|member| member.block.clone())
                            .collect::<Vec<_>>(),
                    ),
                    _ => None,
                })
                .unwrap_or_default();
            device_refs.sort();
            let scope = match ConfirmedDestructiveScope::new(Vec::new(), device_refs) {
                Ok(scope) => scope,
                Err(_) => return LogicalReviewData::None,
            };
            LogicalReviewData::DestructiveScope {
                primary: primary.clone(),
                scope,
                policy: DestructiveScopePolicy::DeleteMdRaidArray {
                    configuration_policy: ConfigurationCleanupPolicy::Preserve,
                },
            }
        }
        // The selected member/device is intentionally not part of the
        // payload-free request key. Its exact review is recalculated once the
        // typed draft supplies a fresh reference.
        LogicalActionKind::RemoveLvmPhysicalVolume
        | LogicalActionKind::RemoveMdRaidMember
        | LogicalActionKind::RemoveBtrfsDevice => LogicalReviewData::None,
        _ => LogicalReviewData::None,
    }
}

async fn preflight_candidates(
    backend: &UdisksBackend,
    epoch: u64,
    topology: &[storage_types::LogicalEntity],
    target: &LogicalPreflightTarget,
) -> Result<Vec<LogicalDeviceCandidate>, StorageError> {
    let target_members = target_member_ids(topology, target);
    let mut candidates = Vec::new();
    for block in blocks(backend.manager()).await? {
        let display = LogicalCandidateDisplay {
            label: if block.id_label.trim().is_empty() {
                LogicalDisplay::unknown("UDisks did not report Block.IdLabel")
            } else {
                LogicalDisplay::known(block.id_label.clone())
            },
            path: if block.device_path.trim().is_empty() {
                LogicalDisplay::unknown("UDisks did not report a device path")
            } else {
                LogicalDisplay::known(block.device_path.clone())
            },
            size: block
                .size
                .map(LogicalDisplay::known)
                .unwrap_or_else(|| LogicalDisplay::unknown("UDisks did not report Block.Size")),
        };
        if target_members.contains(&block.id) {
            candidates.push(LogicalDeviceCandidate::Blocked {
                display,
                reason: CandidateBlockReason::AlreadyTargetMember,
            });
        } else if !block.id_type.trim().is_empty() {
            candidates.push(LogicalDeviceCandidate::Blocked {
                display,
                reason: CandidateBlockReason::StructuredDataSignature {
                    signature: block.id_type.clone(),
                },
            });
        } else if let Some(fingerprint) = block.fingerprint {
            candidates.push(LogicalDeviceCandidate::Ready {
                device: BlockDeviceRef::new(block.id, fingerprint, epoch),
                display,
            });
        } else {
            candidates.push(LogicalDeviceCandidate::Blocked {
                display,
                reason: CandidateBlockReason::NoStrongIdentity,
            });
        }
    }
    candidates.sort_by_key(candidate_sort_key);
    Ok(candidates)
}

fn target_member_ids(
    topology: &[storage_types::LogicalEntity],
    target: &LogicalPreflightTarget,
) -> BTreeSet<storage_types::BlockDeviceId> {
    let LogicalPreflightTarget::Root(target) = target else {
        return BTreeSet::new();
    };
    let Some(entity) = topology.iter().find(|entity| &entity.id == target) else {
        return BTreeSet::new();
    };
    match &entity.details {
        LogicalEntityDetails::BtrfsFilesystem(details) => details
            .members
            .iter()
            .filter_map(|member| member.block.as_ref().map(|block| block.id.clone()))
            .collect(),
        LogicalEntityDetails::LvmVolumeGroup(details) => details
            .physical_volumes
            .iter()
            .filter_map(|member| member.member.block.as_ref().map(|block| block.id.clone()))
            .collect(),
        LogicalEntityDetails::MdRaidArray(details) => details
            .members
            .iter()
            .filter_map(|member| member.block.as_ref().map(|block| block.id.clone()))
            .collect(),
        _ => BTreeSet::new(),
    }
}

fn ensure_unstructured_candidate(
    block: &super::resolve::ResolvedBlock,
) -> Result<(), StorageError> {
    if block.id_type.trim().is_empty() {
        Ok(())
    } else {
        Err(StorageError::new(
            StorageErrorKind::Conflict,
            format!(
                "The selected device now contains the '{}' structured-data signature.",
                block.id_type
            ),
        ))
    }
}

async fn verify_not_btrfs_member(
    backend: &UdisksBackend,
    filesystem: &LogicalEntityId,
    device_id: &storage_types::BlockDeviceId,
) -> Result<(), StorageError> {
    let topology = super::discover::entities(backend.manager(), backend.logical_epoch()).await?;
    let already_member = topology
        .iter()
        .find(|entity| &entity.id == filesystem)
        .and_then(|entity| match &entity.details {
            LogicalEntityDetails::BtrfsFilesystem(details) => Some(details),
            _ => None,
        })
        .is_some_and(|details| {
            details.members.iter().any(|member| {
                member
                    .block
                    .as_ref()
                    .is_some_and(|reference| &reference.id == device_id)
            })
        });
    if already_member {
        Err(conflict(
            "The selected device is already a member of this Btrfs filesystem.",
        ))
    } else {
        Ok(())
    }
}

fn candidate_sort_key(candidate: &LogicalDeviceCandidate) -> String {
    match candidate {
        LogicalDeviceCandidate::Ready { device, .. } => device.id.to_string(),
        LogicalDeviceCandidate::Blocked { display, .. } => display
            .path
            .as_known()
            .cloned()
            .unwrap_or_else(|| "~unknown".into()),
    }
}

fn constraints_for(kind: LogicalActionKind) -> LogicalInputConstraints {
    let mut constraints = LogicalInputConstraints::default();
    if matches!(
        kind,
        LogicalActionKind::CreateLvmLogicalVolume
            | LogicalActionKind::ResizeLvmLogicalVolume
            | LogicalActionKind::ResizeBtrfsFilesystem
    ) {
        constraints.byte_size = Some(ByteSizeConstraint {
            minimum: std::num::NonZeroU64::MIN,
            maximum: None,
            alignment: std::num::NonZeroU64::MIN,
        });
    }
    if kind == LogicalActionKind::CreateMdRaidArray {
        constraints.md_profiles = [
            MdRaidLevel::Raid0,
            MdRaidLevel::Raid1,
            MdRaidLevel::Raid4,
            MdRaidLevel::Raid5,
            MdRaidLevel::Raid6,
            MdRaidLevel::Raid10,
        ]
        .into_iter()
        .map(|level| {
            let profile = MdRaidCreateProfile::for_level(level);
            MdRaidProfileOption {
                profile,
                label: level.source_name().into(),
                member_minimum: std::num::NonZeroU8::new(profile.minimum_members() as u8)
                    .expect("audited MD minimum member counts are nonzero"),
                chunk_bytes: profile.chunk_bytes,
                metadata_version: String::from_utf8_lossy(MdRaidCreateProfile::METADATA_VERSION)
                    .into_owned(),
            }
        })
        .collect();
    }
    constraints
}

async fn resolve_fresh_blocks(
    backend: &UdisksBackend,
    references: &[BlockDeviceRef],
) -> Result<Vec<super::resolve::ResolvedBlock>, StorageError> {
    let epoch = backend.logical_epoch();
    let mut blocks = Vec::with_capacity(references.len());
    for reference in references {
        let first = resolve_block(backend.manager(), epoch, reference).await?;
        let second = resolve_block(backend.manager(), backend.logical_epoch(), reference).await?;
        if first.path != second.path || first.fingerprint != second.fingerprint {
            return Err(conflict("The selected device changed; no action was sent."));
        }
        blocks.push(second);
    }
    Ok(blocks)
}

async fn resolve_fresh_block(
    backend: &UdisksBackend,
    reference: &BlockDeviceRef,
) -> Result<super::resolve::ResolvedBlock, StorageError> {
    Ok(
        resolve_fresh_blocks(backend, std::slice::from_ref(reference))
            .await?
            .remove(0),
    )
}

async fn find_volume_group(
    backend: &UdisksBackend,
    target: &LogicalEntityId,
) -> Result<OwnedObjectPath, StorageError> {
    let uuid = target.0.strip_prefix("lvm-vg:").ok_or_else(|| {
        StorageError::new(
            StorageErrorKind::InvalidInput,
            "Expected an LVM volume group ID.",
        )
    })?;
    for (path, interfaces) in managed_paths(backend.manager()).await? {
        if !interfaces
            .iter()
            .any(|interface| interface == VOLUME_GROUP_INTERFACE)
        {
            continue;
        }
        let proxy = VolumeGroupProxy::builder(backend.manager().connection())
            .path(&path)
            .map_err(native_error)?
            .build()
            .await
            .map_err(native_error)?;
        if proxy.uuid().await.map_err(native_error)? == uuid {
            return Ok(path);
        }
    }
    Err(StorageError::new(
        StorageErrorKind::NotFound,
        "LVM volume group no longer exists.",
    ))
}

async fn find_logical_volume(
    backend: &UdisksBackend,
    target: &LogicalEntityId,
) -> Result<OwnedObjectPath, StorageError> {
    let suffix = target.0.strip_prefix("lvm-lv:").ok_or_else(|| {
        StorageError::new(
            StorageErrorKind::InvalidInput,
            "Expected an LVM logical volume ID.",
        )
    })?;
    let Some((vg_uuid, lv_name)) = suffix.split_once(':') else {
        return Err(StorageError::new(
            StorageErrorKind::InvalidInput,
            "Malformed LVM logical volume ID.",
        ));
    };
    let vg_path = find_volume_group(backend, &LogicalEntityId(format!("lvm-vg:{vg_uuid}"))).await?;
    let vg = VolumeGroupProxy::builder(backend.manager().connection())
        .path(vg_path)
        .map_err(native_error)?
        .build()
        .await
        .map_err(native_error)?;
    for path in vg.logical_volumes().await.map_err(native_error)? {
        let lv = LogicalVolumeProxy::builder(backend.manager().connection())
            .path(&path)
            .map_err(native_error)?
            .build()
            .await
            .map_err(native_error)?;
        if lv.name().await.map_err(native_error)? == lv_name {
            return Ok(path);
        }
    }
    Err(StorageError::new(
        StorageErrorKind::NotFound,
        "LVM logical volume no longer exists.",
    ))
}

async fn find_mdraid(
    backend: &UdisksBackend,
    target: &LogicalEntityId,
) -> Result<OwnedObjectPath, StorageError> {
    let uuid = target.0.strip_prefix("mdraid:").ok_or_else(|| {
        StorageError::new(StorageErrorKind::InvalidInput, "Expected an MD RAID ID.")
    })?;
    for (path, interfaces) in managed_paths(backend.manager()).await? {
        if !interfaces
            .iter()
            .any(|interface| interface == MDRAID_INTERFACE)
        {
            continue;
        }
        let proxy = MDRaidProxy::builder(backend.manager().connection())
            .path(&path)
            .map_err(native_error)?
            .build()
            .await
            .map_err(native_error)?;
        if proxy.uuid().await.map_err(native_error)? == uuid {
            return Ok(path);
        }
    }
    Err(StorageError::new(
        StorageErrorKind::NotFound,
        "MD RAID array no longer exists.",
    ))
}

async fn mdraid_proxy(
    backend: &UdisksBackend,
    target: &LogicalEntityId,
) -> Result<MDRaidProxy<'static>, StorageError> {
    let path = find_mdraid(backend, target).await?;
    MDRaidProxy::builder(backend.manager().connection())
        .path(path)
        .map_err(native_error)?
        .build()
        .await
        .map_err(native_error)
}

async fn btrfs_proxy(
    backend: &UdisksBackend,
    target: &LogicalEntityId,
) -> Result<BtrfsProxy<'static>, StorageError> {
    let fsid = target.0.strip_prefix("btrfs:").ok_or_else(|| {
        StorageError::new(
            StorageErrorKind::InvalidInput,
            "Expected a Btrfs filesystem ID.",
        )
    })?;
    let blocks = blocks(backend.manager()).await?;
    let mut candidates = Vec::new();
    for (path, interfaces) in managed_paths(backend.manager()).await? {
        if !interfaces
            .iter()
            .any(|interface| interface == BTRFS_INTERFACE)
        {
            continue;
        }
        if let Some(block) = blocks
            .iter()
            .find(|block| block.path == path && block.id_uuid.eq_ignore_ascii_case(fsid))
        {
            candidates.push((path, block));
        }
    }
    candidates.sort_by(|(left_path, left), (right_path, right)| {
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
    if let Some((path, _)) = candidates.into_iter().next() {
        return BtrfsProxy::builder(backend.manager().connection())
            .path(path)
            .map_err(native_error)?
            .build()
            .await
            .map_err(native_error);
    }
    Err(StorageError::new(
        StorageErrorKind::Unsupported,
        "Btrfs UDisks plugin is unavailable.",
    ))
}

/// Resolve a selected Btrfs subvolume from the same freshly selected primary
/// proxy used for the native call.  The UI-supplied path is only an expected
/// value inside the ref; the native path is returned by UDisks here.
async fn resolve_btrfs_subvolume(
    backend: &UdisksBackend,
    proxy: &BtrfsProxy<'_>,
    filesystem: &LogicalEntityId,
    reference: &BtrfsSubvolumeRef,
) -> Result<String, StorageError> {
    if reference.filesystem != *filesystem
        || reference.observed_topology_epoch != backend.logical_epoch()
    {
        return Err(conflict(
            "The selected Btrfs subvolume changed; refresh and select it again.",
        ));
    }
    let (subvolumes, _) = proxy
        .get_subvolumes(false, empty_options())
        .await
        .map_err(native_error)?;
    let Some((id, parent_id, path)) = subvolumes.into_iter().find(|(id, _parent_id, path)| {
        *id == reference.id.get() && path == reference.expected_relative_path.as_str()
    }) else {
        return Err(conflict(
            "The selected Btrfs subvolume path or ID changed; no action was sent.",
        ));
    };
    if id != reference.id.get()
        || std::num::NonZeroU64::new(parent_id) != reference.expected_parent_id
    {
        return Err(conflict(
            "The selected Btrfs subvolume parent changed; refresh and review the action again.",
        ));
    }
    Ok(path)
}

async fn verify_vg_scope(
    backend: &UdisksBackend,
    path: &OwnedObjectPath,
    target: &LogicalEntityId,
    expected: &ConfirmedDestructiveScope,
) -> Result<(), StorageError> {
    let proxy = VolumeGroupProxy::builder(backend.manager().connection())
        .path(path)
        .map_err(native_error)?
        .build()
        .await
        .map_err(native_error)?;
    let uuid = proxy.uuid().await.map_err(native_error)?;
    let mut entity_ids = Vec::new();
    for lv_path in proxy.logical_volumes().await.map_err(native_error)? {
        let lv = LogicalVolumeProxy::builder(backend.manager().connection())
            .path(lv_path)
            .map_err(native_error)?
            .build()
            .await
            .map_err(native_error)?;
        entity_ids.push(LogicalEntityId(format!(
            "lvm-lv:{uuid}:{}",
            lv.name().await.map_err(native_error)?
        )));
    }
    entity_ids.sort();
    let actual = ConfirmedDestructiveScope::new(entity_ids, Vec::new())
        .map_err(|error| StorageError::new(StorageErrorKind::Conflict, error.to_string()))?;
    if &actual != expected || !expected.excludes(target) {
        return Err(conflict("The LVM deletion scope changed; review it again."));
    }
    Ok(())
}

fn verify_empty_scope(
    target: &LogicalEntityId,
    expected: &ConfirmedDestructiveScope,
) -> Result<(), StorageError> {
    let actual =
        ConfirmedDestructiveScope::new(Vec::new(), Vec::new()).expect("empty scope is valid");
    if &actual != expected || !expected.excludes(target) {
        return Err(conflict(
            "The logical-volume deletion scope changed; review it again.",
        ));
    }
    Ok(())
}

async fn verify_mdraid_scope(
    backend: &UdisksBackend,
    path: &OwnedObjectPath,
    target: &LogicalEntityId,
    expected: &ConfirmedDestructiveScope,
) -> Result<(), StorageError> {
    let proxy = MDRaidProxy::builder(backend.manager().connection())
        .path(path)
        .map_err(native_error)?
        .build()
        .await
        .map_err(native_error)?;
    let current_blocks = blocks(backend.manager()).await?;
    let mut device_refs = Vec::new();
    for (member_path, _, _, _, _) in proxy.active_devices().await.map_err(native_error)? {
        let Some(block) = current_blocks
            .iter()
            .find(|block| block.path == member_path)
        else {
            return Err(conflict(
                "An MD RAID member disappeared; no action was sent.",
            ));
        };
        let Some(fingerprint) = &block.fingerprint else {
            return Err(conflict(
                "An MD RAID member lost its strong identity; refresh and retry.",
            ));
        };
        device_refs.push(BlockDeviceRef::new(
            block.id.clone(),
            fingerprint.clone(),
            backend.logical_epoch(),
        ));
    }
    device_refs.sort();
    let actual = ConfirmedDestructiveScope::new(Vec::new(), device_refs)
        .map_err(|error| StorageError::new(StorageErrorKind::Conflict, error.to_string()))?;
    if &actual != expected || !expected.excludes(target) {
        return Err(conflict(
            "The MD RAID deletion scope changed; review it again.",
        ));
    }
    Ok(())
}

fn action_affected_ids(action: &LogicalAction) -> Vec<LogicalEntityId> {
    match action {
        LogicalAction::DeleteLvmVolumeGroup { volume_group, .. }
        | LogicalAction::AddLvmPhysicalVolume { volume_group, .. }
        | LogicalAction::RemoveLvmPhysicalVolume { volume_group, .. }
        | LogicalAction::CreateLvmLogicalVolume { volume_group, .. } => vec![volume_group.clone()],
        LogicalAction::DeleteLvmLogicalVolume { logical_volume, .. }
        | LogicalAction::ResizeLvmLogicalVolume { logical_volume, .. }
        | LogicalAction::ActivateLvmLogicalVolume { logical_volume }
        | LogicalAction::DeactivateLvmLogicalVolume { logical_volume } => {
            vec![logical_volume.clone()]
        }
        LogicalAction::DeleteMdRaidArray { array, .. }
        | LogicalAction::StartMdRaidArray { array }
        | LogicalAction::StopMdRaidArray { array }
        | LogicalAction::AddMdRaidMember { array, .. }
        | LogicalAction::RemoveMdRaidMember { array, .. }
        | LogicalAction::RequestMdRaidSync { array, .. } => vec![array.clone()],
        LogicalAction::AddBtrfsDevice { filesystem, .. }
        | LogicalAction::RemoveBtrfsDevice { filesystem, .. }
        | LogicalAction::ResizeBtrfsFilesystem { filesystem, .. }
        | LogicalAction::SetBtrfsLabel { filesystem, .. }
        | LogicalAction::SetBtrfsDefaultSubvolume { filesystem, .. }
        | LogicalAction::CreateBtrfsSubvolume { filesystem, .. }
        | LogicalAction::DeleteBtrfsSubvolume { filesystem, .. }
        | LogicalAction::CreateBtrfsSnapshot { filesystem, .. } => vec![filesystem.clone()],
        LogicalAction::CreateLvmVolumeGroup { .. } | LogicalAction::CreateMdRaidArray { .. } => {
            Vec::new()
        }
    }
}

fn empty_options() -> Options {
    HashMap::new()
}

fn preserve_wipe_options() -> Options {
    HashMap::from([("wipe", Value::from(false))])
}

fn preserve_teardown_options() -> Options {
    HashMap::from([("tear-down", Value::from(false))])
}

fn preserve_delete_options() -> Options {
    HashMap::from([
        ("wipe", Value::from(false)),
        ("tear-down", Value::from(false)),
    ])
}

fn mdraid_profile_options() -> Options {
    HashMap::from([(
        "version",
        Value::from(MdRaidCreateProfile::METADATA_VERSION.to_vec()),
    )])
}

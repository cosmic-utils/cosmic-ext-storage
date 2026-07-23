//! Object-safe contracts for logical topology discovery and native actions.

use std::{collections::BTreeSet, num::NonZeroU32};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use storage_types::{
    BlockDeviceRef, ConfirmedDestructiveScope, LogicalEntity, LogicalEntityId, LogicalOperation,
    LogicalSource, LogicalSourceAvailability,
};

use crate::{StorageError, StorageErrorKind};

/// A discovery source may be read-only and is intentionally independent of
/// [`LogicalOperations`].
#[async_trait]
pub trait LogicalTopologySource: Send + Sync {
    fn logical_source(&self) -> LogicalSource;
    fn logical_availability(&self) -> LogicalSourceAvailability;
    async fn list_logical_entities(&self) -> Result<Vec<LogicalEntity>, StorageError>;
}

/// The exhaustive, typed logical action set.  UI code cannot supply command
/// fragments, D-Bus paths, or a free-form native options map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicalAction {
    CreateLvmVolumeGroup {
        name: String,
        devices: Vec<BlockDeviceRef>,
    },
    DeleteLvmVolumeGroup {
        volume_group: LogicalEntityId,
        pv_label_policy: LvmWipePolicy,
        configuration_policy: ConfigurationCleanupPolicy,
        confirmed_scope: ConfirmedDestructiveScope,
    },
    AddLvmPhysicalVolume {
        volume_group: LogicalEntityId,
        device: BlockDeviceRef,
    },
    RemoveLvmPhysicalVolume {
        volume_group: LogicalEntityId,
        device: BlockDeviceRef,
        pv_label_policy: LvmWipePolicy,
    },
    CreateLvmLogicalVolume {
        volume_group: LogicalEntityId,
        name: String,
        size_bytes: u64,
    },
    DeleteLvmLogicalVolume {
        logical_volume: LogicalEntityId,
        configuration_policy: ConfigurationCleanupPolicy,
        confirmed_scope: ConfirmedDestructiveScope,
    },
    ResizeLvmLogicalVolume {
        logical_volume: LogicalEntityId,
        size_bytes: u64,
    },
    ActivateLvmLogicalVolume {
        logical_volume: LogicalEntityId,
    },
    DeactivateLvmLogicalVolume {
        logical_volume: LogicalEntityId,
    },
    CreateMdRaidArray {
        name: MdRaidName,
        level: MdRaidLevel,
        devices: Vec<BlockDeviceRef>,
        profile: MdRaidCreateProfile,
    },
    DeleteMdRaidArray {
        array: LogicalEntityId,
        configuration_policy: ConfigurationCleanupPolicy,
        confirmed_scope: ConfirmedDestructiveScope,
    },
    StartMdRaidArray {
        array: LogicalEntityId,
    },
    StopMdRaidArray {
        array: LogicalEntityId,
    },
    AddMdRaidMember {
        array: LogicalEntityId,
        device: BlockDeviceRef,
    },
    RemoveMdRaidMember {
        array: LogicalEntityId,
        device: BlockDeviceRef,
        member_signature_policy: MdRaidMemberWipePolicy,
    },
    RequestMdRaidSync {
        array: LogicalEntityId,
        action: MdRaidSyncAction,
    },
    AddBtrfsDevice {
        filesystem: LogicalEntityId,
        device: BlockDeviceRef,
    },
    RemoveBtrfsDevice {
        filesystem: LogicalEntityId,
        device: BlockDeviceRef,
    },
    ResizeBtrfsFilesystem {
        filesystem: LogicalEntityId,
        request: BtrfsResizeRequest,
    },
    SetBtrfsLabel {
        filesystem: LogicalEntityId,
        label: String,
    },
    SetBtrfsDefaultSubvolume {
        filesystem: LogicalEntityId,
        subvolume_id: NonZeroU32,
    },
}

impl LogicalAction {
    /// Validate structural input before adapter discovery or proxy resolution.
    pub fn validate(&self) -> Result<(), StorageError> {
        match self {
            Self::CreateLvmVolumeGroup { name, devices } => {
                validate_name(name, "volume group")?;
                validate_distinct_devices(devices, 1)?;
            }
            Self::DeleteLvmVolumeGroup {
                volume_group,
                confirmed_scope,
                ..
            } => validate_target_and_scope(volume_group, "lvm-vg:", confirmed_scope)?,
            Self::AddLvmPhysicalVolume {
                volume_group,
                device,
            }
            | Self::RemoveLvmPhysicalVolume {
                volume_group,
                device,
                ..
            } => {
                validate_target(volume_group, "lvm-vg:")?;
                validate_device(device)?;
            }
            Self::CreateLvmLogicalVolume {
                volume_group,
                name,
                size_bytes,
            } => {
                validate_target(volume_group, "lvm-vg:")?;
                validate_name(name, "logical volume")?;
                validate_nonzero(*size_bytes, "logical volume size")?;
            }
            Self::DeleteLvmLogicalVolume {
                logical_volume,
                confirmed_scope,
                ..
            } => validate_target_and_scope(logical_volume, "lvm-lv:", confirmed_scope)?,
            Self::ResizeLvmLogicalVolume {
                logical_volume,
                size_bytes,
            } => {
                validate_target(logical_volume, "lvm-lv:")?;
                validate_nonzero(*size_bytes, "logical volume size")?;
            }
            Self::ActivateLvmLogicalVolume { logical_volume }
            | Self::DeactivateLvmLogicalVolume { logical_volume } => {
                validate_target(logical_volume, "lvm-lv:")?;
            }
            Self::CreateMdRaidArray {
                name,
                level,
                devices,
                profile,
            } => {
                name.validate()?;
                if profile.level != *level {
                    return invalid("MD RAID profile does not match the selected level");
                }
                validate_distinct_devices(devices, profile.minimum_members())?;
                if devices.len() > 28 {
                    return invalid("MD RAID supports at most 28 members for the audited profile");
                }
            }
            Self::DeleteMdRaidArray {
                array,
                confirmed_scope,
                ..
            } => validate_target_and_scope(array, "mdraid:", confirmed_scope)?,
            Self::StartMdRaidArray { array }
            | Self::StopMdRaidArray { array }
            | Self::AddMdRaidMember { array, .. }
            | Self::RemoveMdRaidMember { array, .. }
            | Self::RequestMdRaidSync { array, .. } => {
                validate_target(array, "mdraid:")?;
                match self {
                    Self::AddMdRaidMember { device, .. }
                    | Self::RemoveMdRaidMember { device, .. } => validate_device(device)?,
                    _ => {}
                }
            }
            Self::AddBtrfsDevice { filesystem, device }
            | Self::RemoveBtrfsDevice { filesystem, device } => {
                validate_target(filesystem, "btrfs:")?;
                validate_device(device)?;
            }
            Self::ResizeBtrfsFilesystem {
                filesystem,
                request,
            } => {
                validate_target(filesystem, "btrfs:")?;
                if let BtrfsResizeRequest::AbsoluteBytes(size) = request {
                    validate_nonzero(*size, "Btrfs size")?;
                }
            }
            Self::SetBtrfsLabel { filesystem, label } => {
                validate_target(filesystem, "btrfs:")?;
                if label.contains('\0') {
                    return invalid("Btrfs label cannot contain NUL");
                }
            }
            Self::SetBtrfsDefaultSubvolume {
                filesystem,
                subvolume_id,
            } => {
                validate_target(filesystem, "btrfs:")?;
                if subvolume_id.get() == 0 {
                    return invalid("Btrfs default subvolume ID must be non-zero");
                }
            }
        }
        Ok(())
    }

    pub const fn operation(&self) -> LogicalOperation {
        match self {
            Self::CreateLvmVolumeGroup { .. }
            | Self::CreateLvmLogicalVolume { .. }
            | Self::CreateMdRaidArray { .. } => LogicalOperation::Create,
            Self::DeleteLvmVolumeGroup { .. }
            | Self::DeleteLvmLogicalVolume { .. }
            | Self::DeleteMdRaidArray { .. } => LogicalOperation::Delete,
            Self::ResizeLvmLogicalVolume { .. } | Self::ResizeBtrfsFilesystem { .. } => {
                LogicalOperation::Resize
            }
            Self::AddLvmPhysicalVolume { .. }
            | Self::AddMdRaidMember { .. }
            | Self::AddBtrfsDevice { .. } => LogicalOperation::AddMember,
            Self::RemoveLvmPhysicalVolume { .. }
            | Self::RemoveMdRaidMember { .. }
            | Self::RemoveBtrfsDevice { .. } => LogicalOperation::RemoveMember,
            Self::ActivateLvmLogicalVolume { .. } => LogicalOperation::Activate,
            Self::DeactivateLvmLogicalVolume { .. } => LogicalOperation::Deactivate,
            Self::StartMdRaidArray { .. } => LogicalOperation::Start,
            Self::StopMdRaidArray { .. } => LogicalOperation::Stop,
            Self::RequestMdRaidSync {
                action: MdRaidSyncAction::Check,
                ..
            } => LogicalOperation::Check,
            Self::RequestMdRaidSync {
                action: MdRaidSyncAction::Repair,
                ..
            } => LogicalOperation::Repair,
            Self::SetBtrfsLabel { .. } => LogicalOperation::SetLabel,
            Self::SetBtrfsDefaultSubvolume { .. } => LogicalOperation::SetDefaultSubvolume,
        }
    }
}

/// The branch semantics preserve an LVM PV label on removal/delete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LvmWipePolicy {
    Preserve,
}

/// The branch semantics preserve fstab/crypttab-style configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigurationCleanupPolicy {
    Preserve,
}

/// The branch semantics preserve known MD member signatures on removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MdRaidMemberWipePolicy {
    Preserve,
}

/// Source-compatible MD RAID array name after retaining the final slash part.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MdRaidName(pub String);

impl MdRaidName {
    pub fn from_source(value: &str) -> Result<Self, StorageError> {
        let component = value.rsplit('/').next().unwrap_or_default().trim();
        let name = Self(component.to_string());
        name.validate()?;
        Ok(name)
    }

    pub fn validate(&self) -> Result<(), StorageError> {
        validate_name(&self.0, "MD RAID name")
    }
}

/// Closed audited source levels; unknown source text remains a blocked UI form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MdRaidLevel {
    Raid0,
    Raid1,
    Raid4,
    Raid5,
    Raid6,
    Raid10,
}

impl MdRaidLevel {
    pub const fn source_name(self) -> &'static str {
        match self {
            Self::Raid0 => "raid0",
            Self::Raid1 => "raid1",
            Self::Raid4 => "raid4",
            Self::Raid5 => "raid5",
            Self::Raid6 => "raid6",
            Self::Raid10 => "raid10",
        }
    }
}

/// Fixed native translation of the legacy `mdadm --metadata=0.90` profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MdRaidCreateProfile {
    pub level: MdRaidLevel,
    pub chunk_bytes: u64,
}

impl MdRaidCreateProfile {
    pub const METADATA_VERSION: &'static [u8] = b"0.90";

    pub const fn for_level(level: MdRaidLevel) -> Self {
        let chunk_bytes = match level {
            MdRaidLevel::Raid1 => 0,
            _ => 524_288,
        };
        Self { level, chunk_bytes }
    }

    pub const fn minimum_members(self) -> usize {
        match self.level {
            MdRaidLevel::Raid0 | MdRaidLevel::Raid1 | MdRaidLevel::Raid10 => 2,
            MdRaidLevel::Raid4 | MdRaidLevel::Raid5 => 3,
            MdRaidLevel::Raid6 => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MdRaidSyncAction {
    Check,
    Repair,
}

/// Source-visible Btrfs size syntax.  Only absolute bytes currently dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BtrfsResizeRequest {
    AbsoluteBytes(u64),
    Maximum,
    GrowBy(u64),
    ShrinkBy(u64),
}

/// Opaque native result metadata, deliberately excluding an object path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalActionOutcome {
    pub action: LogicalAction,
    pub affected_entity_ids: Vec<LogicalEntityId>,
    pub native_job_id: Option<String>,
    pub progress: Option<storage_types::ProgressRatio>,
}

/// Native logical mutation authority.  There is exactly one executor in the
/// application registry, while sources may be many and read-only.
#[async_trait]
pub trait LogicalOperations: Send + Sync {
    async fn execute_logical_action(
        &self,
        action: LogicalAction,
    ) -> Result<LogicalActionOutcome, StorageError>;
}

fn validate_name(value: &str, noun: &str) -> Result<(), StorageError> {
    let value = value.trim();
    if value.is_empty() || value.contains('/') || value.contains('\0') || value.len() > 127 {
        return invalid(format!(
            "{noun} must be non-empty, at most 127 bytes, and contain no slash or NUL"
        ));
    }
    Ok(())
}

fn validate_target(target: &LogicalEntityId, prefix: &str) -> Result<(), StorageError> {
    if target.0.starts_with(prefix) {
        Ok(())
    } else {
        invalid(format!(
            "logical entity ID '{}' is not a {prefix} target",
            target.0
        ))
    }
}

fn validate_target_and_scope(
    target: &LogicalEntityId,
    prefix: &str,
    scope: &ConfirmedDestructiveScope,
) -> Result<(), StorageError> {
    validate_target(target, prefix)?;
    scope
        .validate()
        .map_err(|error| StorageError::new(StorageErrorKind::InvalidInput, error.to_string()))?;
    if !scope.excludes(target) {
        return invalid("destructive confirmation scope must not include the primary target");
    }
    Ok(())
}

fn validate_device(device: &BlockDeviceRef) -> Result<(), StorageError> {
    // Reparse both serialized values to defend callers that decoded legacy or
    // otherwise malformed input before reaching the contract boundary.
    device
        .id
        .as_str()
        .parse::<storage_types::BlockDeviceId>()
        .map_err(|error| StorageError::new(StorageErrorKind::InvalidInput, error.to_string()))?;
    device
        .fingerprint
        .as_str()
        .parse::<storage_types::BlockDeviceFingerprint>()
        .map_err(|error| StorageError::new(StorageErrorKind::InvalidInput, error.to_string()))?;
    Ok(())
}

fn validate_distinct_devices(
    devices: &[BlockDeviceRef],
    minimum: usize,
) -> Result<(), StorageError> {
    if devices.len() < minimum {
        return invalid(format!(
            "at least {minimum} distinct block devices are required"
        ));
    }
    let mut ids = BTreeSet::new();
    for device in devices {
        validate_device(device)?;
        if !ids.insert(device.id.clone()) {
            return invalid("block device list contains a duplicate device");
        }
    }
    Ok(())
}

fn validate_nonzero(value: u64, noun: &str) -> Result<(), StorageError> {
    if value == 0 {
        invalid(format!("{noun} must be non-zero"))
    } else {
        Ok(())
    }
}

fn invalid(message: impl Into<String>) -> Result<(), StorageError> {
    Err(StorageError::new(StorageErrorKind::InvalidInput, message))
}

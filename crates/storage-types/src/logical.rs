//! Typed logical-storage topology and identity values.
//!
//! The types in this module deliberately keep display values separate from
//! mutation identity.  In particular, a block-device path or a D-Bus object
//! path is never an action identifier: actions use a [`BlockDeviceRef`], which
//! binds a short-lived device number to an immutable observed fingerprint and
//! object-manager generation.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    num::{NonZeroU32, NonZeroU64},
    str::FromStr,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// Stable logical entity identity used for hierarchy and selection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LogicalEntityId(pub String);

impl LogicalEntityId {
    pub fn new(value: impl Into<String>) -> Result<Self, LogicalIdentityError> {
        let value = value.into();
        if value.trim().is_empty() || value.contains('\0') {
            return Err(LogicalIdentityError::InvalidEntityId);
        }
        Ok(Self(value))
    }
}

impl fmt::Display for LogicalEntityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Stable member identity within a logical entity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LogicalMemberId(pub String);

impl LogicalMemberId {
    pub fn new(value: impl Into<String>) -> Result<Self, LogicalIdentityError> {
        let value = value.into();
        if value.trim().is_empty() || value.contains('\0') {
            return Err(LogicalIdentityError::InvalidMemberId);
        }
        Ok(Self(value))
    }
}

impl fmt::Display for LogicalMemberId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// A value intended for display whose absence is meaningful.  Storage
/// discovery must never use a zero or an empty string as a stand-in for an
/// unreadable native property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicalDisplay<T> {
    Known(T),
    Unknown { reason: String },
}

impl<T> LogicalDisplay<T> {
    pub fn known(value: T) -> Self {
        Self::Known(value)
    }

    pub fn unknown(reason: impl Into<String>) -> Self {
        Self::Unknown {
            reason: reason.into(),
        }
    }

    pub fn as_known(&self) -> Option<&T> {
        match self {
            Self::Known(value) => Some(value),
            Self::Unknown { .. } => None,
        }
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Known(_) => None,
            Self::Unknown { reason } => Some(reason),
        }
    }
}

/// A validated relative Btrfs path.  It is used for rendering and creation
/// payloads; selected subvolumes use [`BtrfsSubvolumeRef`] instead.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BtrfsRelativePath(String);

impl BtrfsRelativePath {
    pub fn new(value: impl Into<String>) -> Result<Self, LogicalIdentityError> {
        let value = value.into();
        if value.is_empty()
            || value.starts_with('/')
            || value.contains('\0')
            || value
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(LogicalIdentityError::InvalidBtrfsRelativePath);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BtrfsRelativePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// A stable render-row key.  It deliberately includes an occurrence so a
/// malformed native result can be shown without silently choosing one row.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BtrfsSubvolumeRowKey {
    pub id: NonZeroU64,
    pub relative_path: BtrfsRelativePath,
    pub occurrence: NonZeroU32,
}

/// Fresh subvolume identity used by default/delete/snapshot actions.  A
/// display path alone can never be substituted for this reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BtrfsSubvolumeRef {
    pub filesystem: LogicalEntityId,
    pub id: NonZeroU64,
    pub expected_relative_path: BtrfsRelativePath,
    pub expected_parent_id: Option<NonZeroU64>,
    pub observed_topology_epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LvmActivationState {
    Active,
    Inactive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LvmPhysicalVolumeState {
    /// The adapter found the PV.  This does not imply allocation or health.
    Available,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MdRaidHealth {
    Healthy,
    Degraded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MdRaidMemberRole {
    Active,
    Spare,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MdRaidMemberState {
    Native { labels: Vec<String> },
}

/// The native MD level token, validated as non-empty lower-case ASCII.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MdRaidLevelName(String);

impl MdRaidLevelName {
    pub fn new(value: impl Into<String>) -> Result<Self, LogicalIdentityError> {
        let value = value.into();
        if value.is_empty()
            || value.trim() != value
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        {
            return Err(LogicalIdentityError::InvalidMdRaidLevelName);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LvmLogicalVolumeSummary {
    pub entity_id: LogicalEntityId,
    pub name: String,
    pub size: LogicalDisplay<u64>,
    pub activation: LogicalDisplay<LvmActivationState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LvmPhysicalVolumeSummary {
    pub entity_id: LogicalEntityId,
    pub member: LvmPhysicalVolumeDetails,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LvmVolumeGroupDetails {
    pub name: String,
    pub uuid: LogicalDisplay<String>,
    pub size: LogicalDisplay<u64>,
    pub used: LogicalDisplay<u64>,
    pub free: LogicalDisplay<u64>,
    pub logical_volumes: Vec<LvmLogicalVolumeSummary>,
    pub physical_volumes: Vec<LvmPhysicalVolumeSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LvmLogicalVolumeDetails {
    pub name: String,
    pub volume_group: LogicalEntityId,
    pub device_path: LogicalDisplay<String>,
    pub size: LogicalDisplay<u64>,
    pub activation: LogicalDisplay<LvmActivationState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LvmPhysicalVolumeDetails {
    pub block: Option<BlockDeviceRef>,
    pub display_path: LogicalDisplay<String>,
    pub size: LogicalDisplay<u64>,
    pub state: LogicalDisplay<LvmPhysicalVolumeState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MdRaidMemberDetails {
    pub member_id: LogicalMemberId,
    pub block: Option<BlockDeviceRef>,
    pub display_path: LogicalDisplay<String>,
    pub size: LogicalDisplay<u64>,
    pub role: LogicalDisplay<MdRaidMemberRole>,
    pub state: LogicalDisplay<MdRaidMemberState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MdRaidArrayDetails {
    pub name: String,
    pub uuid: LogicalDisplay<String>,
    pub level: LogicalDisplay<MdRaidLevelName>,
    pub size: LogicalDisplay<u64>,
    pub running: LogicalDisplay<bool>,
    pub health: LogicalDisplay<MdRaidHealth>,
    pub sync_progress: LogicalDisplay<ProgressRatio>,
    pub members: Vec<MdRaidMemberDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BtrfsAllocation {
    pub total: u64,
    pub used: u64,
    pub free: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountPointUsage {
    pub mount_point: String,
    pub total: u64,
    pub used: u64,
    pub free: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BtrfsMemberState {
    Writable,
    ReadOnly,
    Unknown { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BtrfsPrimaryMember {
    Selected { member_id: LogicalMemberId },
    Unavailable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BtrfsTopologyDiagnostic {
    DuplicateSubvolumeId {
        id: NonZeroU64,
        rows: Vec<BtrfsSubvolumeRowKey>,
    },
    DuplicateRelativePath {
        path: BtrfsRelativePath,
        rows: Vec<BtrfsSubvolumeRowKey>,
    },
    SelfParent {
        row: BtrfsSubvolumeRowKey,
        id: NonZeroU64,
    },
    Cycle {
        rows: Vec<BtrfsSubvolumeRowKey>,
    },
    MissingParent {
        row: BtrfsSubvolumeRowKey,
        parent_id: NonZeroU64,
    },
    PrimaryUnavailable {
        member_id: Option<LogicalMemberId>,
        reason: String,
    },
    FilesystemValueDisagreement {
        field: BtrfsFilesystemValue,
        member_ids: Vec<LogicalMemberId>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BtrfsFilesystemValue {
    Label,
    Allocation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BtrfsSubvolumeHierarchy {
    Attached,
    Unparented {
        diagnostics: Vec<BtrfsTopologyDiagnostic>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BtrfsMember {
    pub member_id: LogicalMemberId,
    pub block: Option<BlockDeviceRef>,
    pub display_path: LogicalDisplay<String>,
    pub label: LogicalDisplay<String>,
    pub size: LogicalDisplay<u64>,
    pub state: BtrfsMemberState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BtrfsSubvolumeDetails {
    pub row_key: BtrfsSubvolumeRowKey,
    pub id: NonZeroU64,
    pub relative_path: BtrfsRelativePath,
    pub parent_id: Option<NonZeroU64>,
    pub hierarchy: BtrfsSubvolumeHierarchy,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BtrfsFilesystemDetails {
    pub filesystem_uuid: Uuid,
    pub label: LogicalDisplay<String>,
    pub allocation: LogicalDisplay<BtrfsAllocation>,
    pub mount_usage: Option<MountPointUsage>,
    pub default_subvolume: LogicalDisplay<Option<NonZeroU64>>,
    pub primary_member: BtrfsPrimaryMember,
    pub members: Vec<BtrfsMember>,
    pub subvolumes: Vec<BtrfsSubvolumeDetails>,
    pub diagnostics: Vec<BtrfsTopologyDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BtrfsDeviceDetails {
    pub filesystem: LogicalEntityId,
    pub member: BtrfsMember,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BtrfsSubvolumeEntityDetails {
    pub filesystem: LogicalEntityId,
    pub subvolume: BtrfsSubvolumeDetails,
}

/// Display ownership for a logical entity.  Renderers use this discriminant
/// instead of generic strings or the diagnostic `metadata` map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicalEntityDetails {
    LvmVolumeGroup(LvmVolumeGroupDetails),
    LvmLogicalVolume(LvmLogicalVolumeDetails),
    LvmPhysicalVolume(LvmPhysicalVolumeDetails),
    MdRaidArray(MdRaidArrayDetails),
    MdRaidMember(MdRaidMemberDetails),
    BtrfsFilesystem(BtrfsFilesystemDetails),
    BtrfsDevice(BtrfsDeviceDetails),
    BtrfsSubvolume(BtrfsSubvolumeEntityDetails),
}

/// A current UDisks lookup key made from a Linux major/minor device number.
///
/// It is intentionally not stable and must never authorize a mutation by
/// itself.  Use [`BlockDeviceRef`] instead.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockDeviceId(String);

impl BlockDeviceId {
    pub fn new(major: u64, minor: u64) -> Self {
        Self(format!("block:{major}:{minor}"))
    }

    pub fn major_minor(&self) -> (u64, u64) {
        // Construction and deserialization validate this representation.
        let mut fields = self.0["block:".len()..].split(':');
        let major = fields
            .next()
            .unwrap_or_default()
            .parse()
            .unwrap_or_default();
        let minor = fields
            .next()
            .unwrap_or_default()
            .parse()
            .unwrap_or_default();
        (major, minor)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BlockDeviceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for BlockDeviceId {
    type Err = LogicalIdentityError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let Some(numbers) = value.strip_prefix("block:") else {
            return Err(LogicalIdentityError::InvalidBlockDeviceId);
        };
        let mut parts = numbers.split(':');
        let (Some(major), Some(minor), None) = (parts.next(), parts.next(), parts.next()) else {
            return Err(LogicalIdentityError::InvalidBlockDeviceId);
        };
        if !canonical_unsigned_decimal(major) || !canonical_unsigned_decimal(minor) {
            return Err(LogicalIdentityError::InvalidBlockDeviceId);
        }
        let major = major
            .parse::<u64>()
            .map_err(|_| LogicalIdentityError::InvalidBlockDeviceId)?;
        let minor = minor
            .parse::<u64>()
            .map_err(|_| LogicalIdentityError::InvalidBlockDeviceId)?;
        Ok(Self::new(major, minor))
    }
}

impl Serialize for BlockDeviceId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for BlockDeviceId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

/// Immutable identity observed by the native adapter for a block device.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockDeviceFingerprint(String);

impl BlockDeviceFingerprint {
    /// A partition identity is strong only when bound to its containing drive.
    pub fn partition_uuid_bound(
        partition_uuid: impl AsRef<str>,
        drive_wwn: impl AsRef<str>,
        drive_serial: impl AsRef<str>,
    ) -> Result<Self, LogicalIdentityError> {
        Self::from_parts(
            "partition",
            &[
                &normalize_identity(partition_uuid.as_ref()),
                &normalize_identity(drive_wwn.as_ref()),
                &normalize_identity(drive_serial.as_ref()),
            ],
        )
    }

    pub fn loop_backing_file(device: u64, inode: u64) -> Self {
        Self::from_parts("loop", &[&device.to_string(), &inode.to_string()])
            .expect("numeric loop identity is valid")
    }

    pub fn partition_uuid(uuid: impl AsRef<str>) -> Result<Self, LogicalIdentityError> {
        Self::from_parts("partuuid", &[&normalize_identity(uuid.as_ref())])
    }

    pub fn filesystem_uuid(
        uuid: impl AsRef<str>,
        filesystem_type: impl AsRef<str>,
    ) -> Result<Self, LogicalIdentityError> {
        Self::from_parts(
            "fsuuid",
            &[
                &normalize_identity(uuid.as_ref()),
                &normalize_identity(filesystem_type.as_ref()),
            ],
        )
    }

    pub fn drive_wwn_serial(
        wwn: impl AsRef<str>,
        serial: impl AsRef<str>,
    ) -> Result<Self, LogicalIdentityError> {
        Self::from_parts(
            "drive",
            &[
                &normalize_identity(wwn.as_ref()),
                &normalize_identity(serial.as_ref()),
            ],
        )
    }

    pub fn from_parts(tier: &str, fields: &[&str]) -> Result<Self, LogicalIdentityError> {
        let expected_count = match tier {
            "loop" | "fsuuid" | "drive" => 2,
            "partuuid" => 1,
            "partition" => 3,
            _ => return Err(LogicalIdentityError::InvalidFingerprint),
        };
        if fields.len() != expected_count
            || fields
                .iter()
                .any(|field| field.is_empty() || field.contains('\0'))
        {
            return Err(LogicalIdentityError::InvalidFingerprint);
        }
        let mut encoded = String::from("v1");
        encoded.push('\0');
        encoded.push_str(tier);
        for field in fields {
            encoded.push('\0');
            encoded.push_str(field);
        }
        Ok(Self(encoded))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn tier(&self) -> &str {
        self.0.split('\0').nth(1).unwrap_or_default()
    }
}

impl fmt::Display for BlockDeviceFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for BlockDeviceFingerprint {
    type Err = LogicalIdentityError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let fields: Vec<_> = value.split('\0').collect();
        let Some((version, rest)) = fields.split_first() else {
            return Err(LogicalIdentityError::InvalidFingerprint);
        };
        let Some((tier, values)) = rest.split_first() else {
            return Err(LogicalIdentityError::InvalidFingerprint);
        };
        if *version != "v1" {
            return Err(LogicalIdentityError::InvalidFingerprint);
        }
        Self::from_parts(tier, values)
    }
}

impl Serialize for BlockDeviceFingerprint {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for BlockDeviceFingerprint {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

/// A device selected at a specific observed object-manager generation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BlockDeviceRef {
    pub id: BlockDeviceId,
    pub fingerprint: BlockDeviceFingerprint,
    pub observed_generation: u64,
}

/// Logical category discovered from a physical sidebar item.  This is used
/// only for anchored navigation and cannot authorize a mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogicalCandidateKind {
    Btrfs,
    LvmPhysicalVolume,
    RaidMember,
}

/// Snapshot-bound logical-navigation identity.  `display_path` is retained
/// for an unavailable-page explanation only and is never compared during
/// resolution or converted to a device reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalCandidateAnchor {
    pub kind: LogicalCandidateKind,
    pub block_id: BlockDeviceId,
    pub fingerprint: Option<BlockDeviceFingerprint>,
    pub observed_epoch: u64,
    pub display_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LogicalLoadRequest {
    pub anchor: Option<LogicalCandidateAnchor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicalCandidateResolution {
    Resolved { root_id: LogicalEntityId },
    Missing,
    Unavailable { source: String, reason: String },
}

impl BlockDeviceRef {
    pub fn new(
        id: BlockDeviceId,
        fingerprint: BlockDeviceFingerprint,
        observed_generation: u64,
    ) -> Self {
        Self {
            id,
            fingerprint,
            observed_generation,
        }
    }
}

/// Collateral native effects explicitly reviewed by the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmedDestructiveScope {
    pub entity_ids: Vec<LogicalEntityId>,
    pub device_refs: Vec<BlockDeviceRef>,
}

impl ConfirmedDestructiveScope {
    pub fn new(
        entity_ids: Vec<LogicalEntityId>,
        device_refs: Vec<BlockDeviceRef>,
    ) -> Result<Self, LogicalIdentityError> {
        let scope = Self {
            entity_ids,
            device_refs,
        };
        scope.validate()?;
        Ok(scope)
    }

    pub fn validate(&self) -> Result<(), LogicalIdentityError> {
        if !strictly_sorted(&self.entity_ids) || !strictly_sorted(&self.device_refs) {
            return Err(LogicalIdentityError::UncanonicalDestructiveScope);
        }
        Ok(())
    }

    pub fn excludes(&self, primary: &LogicalEntityId) -> bool {
        !self.entity_ids.contains(primary)
    }
}

/// Entity category represented in logical topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LogicalEntityKind {
    LvmVolumeGroup,
    LvmLogicalVolume,
    LvmPhysicalVolume,
    MdRaidArray,
    MdRaidMember,
    BtrfsFilesystem,
    BtrfsDevice,
    BtrfsSubvolume,
}

/// A capability exposed by a logical entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LogicalOperation {
    Create,
    Delete,
    Resize,
    AddMember,
    RemoveMember,
    Activate,
    Deactivate,
    Start,
    Stop,
    Check,
    Repair,
    SetLabel,
    SetDefaultSubvolume,
}

/// Why a source-visible action cannot currently be submitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalBlockedReason {
    pub operation: LogicalOperation,
    pub reason: String,
}

/// Normalized action availability.  A blocked action always takes precedence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LogicalCapabilities {
    pub supported: Vec<LogicalOperation>,
    pub blocked: Vec<LogicalBlockedReason>,
}

impl LogicalCapabilities {
    pub fn normalized(
        mut supported: Vec<LogicalOperation>,
        mut blocked: Vec<LogicalBlockedReason>,
    ) -> Self {
        supported.sort_unstable();
        supported.dedup();
        blocked.sort_by_key(|entry| entry.operation);
        blocked.dedup_by(|left, right| left.operation == right.operation);
        Self { supported, blocked }
    }

    pub fn is_supported(&self, operation: LogicalOperation) -> bool {
        self.supported.contains(&operation)
    }

    pub fn blocked_reason(&self, operation: LogicalOperation) -> Option<&str> {
        self.blocked
            .iter()
            .find(|entry| entry.operation == operation)
            .map(|entry| entry.reason.as_str())
    }

    pub fn is_allowed(&self, operation: LogicalOperation) -> bool {
        self.is_supported(operation) && self.blocked_reason(operation).is_none()
    }

    pub fn block_all(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        let operations = all_logical_operations();
        Self::normalized(
            Vec::new(),
            operations
                .iter()
                .copied()
                .map(|operation| LogicalBlockedReason {
                    operation,
                    reason: reason.clone(),
                })
                .collect(),
        )
    }
}

pub const fn all_logical_operations() -> [LogicalOperation; 13] {
    [
        LogicalOperation::Create,
        LogicalOperation::Delete,
        LogicalOperation::Resize,
        LogicalOperation::AddMember,
        LogicalOperation::RemoveMember,
        LogicalOperation::Activate,
        LogicalOperation::Deactivate,
        LogicalOperation::Start,
        LogicalOperation::Stop,
        LogicalOperation::Check,
        LogicalOperation::Repair,
        LogicalOperation::SetLabel,
        LogicalOperation::SetDefaultSubvolume,
    ]
}

/// Progress represented as ten-thousandths, preventing invalid fractions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProgressRatio(u32);

impl ProgressRatio {
    pub fn from_fraction(fraction: f64) -> Self {
        let clamped = fraction.clamp(0.0, 1.0);
        Self((clamped * 10_000.0).round() as u32)
    }

    pub fn from_ten_thousandths(value: u32) -> Self {
        Self(value.min(10_000))
    }

    pub fn as_fraction(self) -> f64 {
        f64::from(self.0) / 10_000.0
    }
}

/// A source-discovered child/member of an aggregate logical entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalMember {
    pub id: LogicalMemberId,
    pub kind: LogicalEntityKind,
    pub name: String,
    pub device_ref: Option<BlockDeviceRef>,
    pub device_path: Option<String>,
    pub role: Option<String>,
    pub state: Option<String>,
    pub size_bytes: Option<u64>,
}

/// Canonical logical topology entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalEntity {
    pub id: LogicalEntityId,
    pub kind: LogicalEntityKind,
    /// Typed display data.  This is the only source a logical page, form, or
    /// review may use for user-visible topology values.
    pub details: LogicalEntityDetails,
    pub parent_id: Option<LogicalEntityId>,
    pub capabilities: LogicalCapabilities,
    /// Diagnostic data retained for logs and support reports.  It is not a UI
    /// or mutation API.
    pub metadata: BTreeMap<String, String>,

    // Kept temporarily for source compatibility with read-only local tooling.
    // New logical code must use `details`; these fields are removed once the
    // local source has completed its typed migration.
    pub name: String,
    pub uuid: Option<String>,
    pub device_path: Option<String>,
    pub size_bytes: u64,
    pub used_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    pub health_status: Option<String>,
    pub progress_fraction: Option<ProgressRatio>,
    pub members: Vec<LogicalMember>,
}

impl LogicalEntity {
    pub fn display_name(&self) -> &str {
        match &self.details {
            LogicalEntityDetails::LvmVolumeGroup(details) => &details.name,
            LogicalEntityDetails::LvmLogicalVolume(details) => &details.name,
            LogicalEntityDetails::LvmPhysicalVolume(details) => details
                .display_path
                .as_known()
                .map(String::as_str)
                .unwrap_or("Unknown physical volume"),
            LogicalEntityDetails::MdRaidArray(details) => &details.name,
            LogicalEntityDetails::MdRaidMember(details) => details
                .display_path
                .as_known()
                .map(String::as_str)
                .unwrap_or("Unknown MD RAID member"),
            LogicalEntityDetails::BtrfsFilesystem(details) => details
                .label
                .as_known()
                .filter(|label| !label.is_empty())
                .map(String::as_str)
                .unwrap_or_else(|| {
                    self.id
                        .0
                        .strip_prefix("btrfs:")
                        .unwrap_or("Btrfs filesystem")
                }),
            LogicalEntityDetails::BtrfsDevice(details) => details
                .member
                .display_path
                .as_known()
                .map(String::as_str)
                .unwrap_or("Unknown Btrfs device"),
            LogicalEntityDetails::BtrfsSubvolume(details) => {
                details.subvolume.relative_path.as_str()
            }
        }
    }

    pub fn display_device_path(&self) -> Option<&str> {
        match &self.details {
            LogicalEntityDetails::LvmLogicalVolume(details) => {
                details.device_path.as_known().map(String::as_str)
            }
            LogicalEntityDetails::LvmPhysicalVolume(details) => {
                details.display_path.as_known().map(String::as_str)
            }
            LogicalEntityDetails::MdRaidMember(details) => {
                details.display_path.as_known().map(String::as_str)
            }
            LogicalEntityDetails::BtrfsFilesystem(details) => details
                .members
                .first()
                .and_then(|member| member.display_path.as_known().map(String::as_str)),
            LogicalEntityDetails::BtrfsDevice(details) => {
                details.member.display_path.as_known().map(String::as_str)
            }
            _ => None,
        }
    }

    pub fn inferred_used_bytes(&self) -> Option<u64> {
        match &self.details {
            LogicalEntityDetails::LvmVolumeGroup(details) => {
                details.used.as_known().copied().or_else(|| {
                    match (details.size.as_known(), details.free.as_known()) {
                        (Some(size), Some(free)) => Some(size.saturating_sub(*free)),
                        _ => None,
                    }
                })
            }
            LogicalEntityDetails::BtrfsFilesystem(details) => details
                .allocation
                .as_known()
                .map(|allocation| allocation.used),
            _ => None,
        }
    }

    pub fn inferred_free_bytes(&self) -> Option<u64> {
        match &self.details {
            LogicalEntityDetails::LvmVolumeGroup(details) => {
                details.free.as_known().copied().or_else(|| {
                    match (details.size.as_known(), details.used.as_known()) {
                        (Some(size), Some(used)) => Some(size.saturating_sub(*used)),
                        _ => None,
                    }
                })
            }
            LogicalEntityDetails::BtrfsFilesystem(details) => details
                .allocation
                .as_known()
                .map(|allocation| allocation.free),
            _ => None,
        }
    }

    pub fn display_size_bytes(&self) -> Option<u64> {
        match &self.details {
            LogicalEntityDetails::LvmVolumeGroup(details) => details.size.as_known().copied(),
            LogicalEntityDetails::LvmLogicalVolume(details) => details.size.as_known().copied(),
            LogicalEntityDetails::LvmPhysicalVolume(details) => details.size.as_known().copied(),
            LogicalEntityDetails::MdRaidArray(details) => details.size.as_known().copied(),
            LogicalEntityDetails::MdRaidMember(details) => details.size.as_known().copied(),
            LogicalEntityDetails::BtrfsFilesystem(details) => details
                .allocation
                .as_known()
                .map(|allocation| allocation.total),
            LogicalEntityDetails::BtrfsDevice(details) => details.member.size.as_known().copied(),
            LogicalEntityDetails::BtrfsSubvolume(_) => None,
        }
    }
}

/// Discovery source identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LogicalSource {
    Udisks,
    LocalTools,
}

/// Availability for one independent source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicalSourceAvailability {
    Available,
    Unavailable { reason: String },
    Failed { reason: String },
}

impl LogicalSourceAvailability {
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Available => None,
            Self::Unavailable { reason } | Self::Failed { reason } => Some(reason),
        }
    }
}

/// Status emitted alongside an individual source's topology contribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalSourceStatus {
    pub source: LogicalSource,
    pub availability: LogicalSourceAvailability,
}

/// Deterministically ordered, duplicate-free logical topology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LogicalTopology {
    pub entities: Vec<LogicalEntity>,
    pub sources: Vec<LogicalSourceStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalLoadResult {
    pub topology: LogicalTopology,
    pub candidate_resolution: LogicalCandidateResolution,
}

impl LogicalTopology {
    pub fn new(
        mut entities: Vec<LogicalEntity>,
        mut sources: Vec<LogicalSourceStatus>,
    ) -> Result<Self, LogicalTopologyError> {
        let entity_ids: BTreeSet<_> = entities.iter().map(|entity| entity.id.clone()).collect();
        if entity_ids.len() != entities.len() {
            return Err(LogicalTopologyError::DuplicateEntity);
        }
        let source_ids: BTreeSet<_> = sources.iter().map(|status| status.source).collect();
        if source_ids.len() != sources.len() {
            return Err(LogicalTopologyError::DuplicateSource);
        }
        entities.sort_by(|left, right| {
            left.display_name()
                .cmp(right.display_name())
                .then(left.id.cmp(&right.id))
        });
        sources.sort_by_key(|status| status.source);
        Ok(Self { entities, sources })
    }
}

/// Aggregate summary used by the logical overview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LogicalAggregateSummary {
    pub entity_count: usize,
    pub total_size_bytes: u64,
    pub total_used_bytes: u64,
    pub total_free_bytes: u64,
    pub degraded_count: usize,
}

pub fn summarize_entities(entities: &[LogicalEntity]) -> LogicalAggregateSummary {
    let mut summary = LogicalAggregateSummary {
        entity_count: entities.len(),
        ..Default::default()
    };
    for entity in entities {
        summary.total_size_bytes = summary
            .total_size_bytes
            .saturating_add(entity.display_size_bytes().unwrap_or_default());
        if let Some(used) = entity.inferred_used_bytes() {
            summary.total_used_bytes = summary.total_used_bytes.saturating_add(used);
        }
        if let Some(free) = entity.inferred_free_bytes() {
            summary.total_free_bytes = summary.total_free_bytes.saturating_add(free);
        }
        if matches!(
            &entity.details,
            LogicalEntityDetails::MdRaidArray(MdRaidArrayDetails {
                health: LogicalDisplay::Known(MdRaidHealth::Degraded),
                ..
            })
        ) {
            summary.degraded_count += 1;
        }
    }
    summary
}

/// Validated Btrfs default-subvolume input at the native API boundary.
pub fn btrfs_default_subvolume_id(value: u64) -> Result<NonZeroU32, LogicalIdentityError> {
    let value = u32::try_from(value).map_err(|_| LogicalIdentityError::BtrfsDefaultIdOutOfRange)?;
    NonZeroU32::new(value).ok_or(LogicalIdentityError::BtrfsDefaultIdOutOfRange)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalIdentityError {
    InvalidEntityId,
    InvalidMemberId,
    InvalidBlockDeviceId,
    InvalidFingerprint,
    UncanonicalDestructiveScope,
    BtrfsDefaultIdOutOfRange,
    InvalidBtrfsRelativePath,
    InvalidMdRaidLevelName,
}

impl fmt::Display for LogicalIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidEntityId => "logical entity ID must be non-empty and contain no NUL",
            Self::InvalidMemberId => "logical member ID must be non-empty and contain no NUL",
            Self::InvalidBlockDeviceId => {
                "block device ID must use canonical block:<major>:<minor> encoding"
            }
            Self::InvalidFingerprint => {
                "block device fingerprint is not a valid v1 strong identity"
            }
            Self::UncanonicalDestructiveScope => {
                "destructive scope must be strictly sorted and duplicate-free"
            }
            Self::BtrfsDefaultIdOutOfRange => {
                "Btrfs default subvolume ID must be within 1..=u32::MAX"
            }
            Self::InvalidBtrfsRelativePath => {
                "Btrfs relative path must be non-empty and contain no absolute or traversal component"
            }
            Self::InvalidMdRaidLevelName => {
                "MD RAID level name must be a non-empty lower-case native token"
            }
        })
    }
}

impl std::error::Error for LogicalIdentityError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalTopologyError {
    DuplicateEntity,
    DuplicateSource,
}

impl fmt::Display for LogicalTopologyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::DuplicateEntity => "logical topology contains duplicate entity IDs",
            Self::DuplicateSource => "logical topology contains duplicate source statuses",
        })
    }
}

impl std::error::Error for LogicalTopologyError {}

fn canonical_unsigned_decimal(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && (value == "0" || !value.starts_with('0'))
}

fn normalize_identity(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn strictly_sorted<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

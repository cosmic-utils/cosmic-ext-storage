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
    num::NonZeroU32,
    str::FromStr,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
    pub name: String,
    pub uuid: Option<String>,
    pub parent_id: Option<LogicalEntityId>,
    pub device_path: Option<String>,
    pub size_bytes: u64,
    pub used_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    pub health_status: Option<String>,
    pub progress_fraction: Option<ProgressRatio>,
    pub members: Vec<LogicalMember>,
    pub capabilities: LogicalCapabilities,
    pub metadata: BTreeMap<String, String>,
}

impl LogicalEntity {
    pub fn inferred_used_bytes(&self) -> Option<u64> {
        self.used_bytes.or_else(|| {
            self.free_bytes
                .map(|free| self.size_bytes.saturating_sub(free))
        })
    }

    pub fn inferred_free_bytes(&self) -> Option<u64> {
        self.free_bytes.or_else(|| {
            self.used_bytes
                .map(|used| self.size_bytes.saturating_sub(used))
        })
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
        entities.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
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
        summary.total_size_bytes = summary.total_size_bytes.saturating_add(entity.size_bytes);
        if let Some(used) = entity.inferred_used_bytes() {
            summary.total_used_bytes = summary.total_used_bytes.saturating_add(used);
        }
        if let Some(free) = entity.inferred_free_bytes() {
            summary.total_free_bytes = summary.total_free_bytes.saturating_add(free);
        }
        if entity
            .health_status
            .as_deref()
            .is_some_and(|status| status.eq_ignore_ascii_case("degraded"))
        {
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

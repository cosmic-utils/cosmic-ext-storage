// SPDX-License-Identifier: GPL-3.0-only

//! Closed scenario-fixture DTOs.
//!
//! This module parses bytes supplied by [`ScenarioStore`](crate::ScenarioStore).
//! It deliberately has no host-I/O API.  Schema v2 is intentionally verbose:
//! an E2E fixture must describe the state it relies on instead of receiving an
//! accidental success from a generic empty backend.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use storage_contracts::{StorageError, StorageErrorKind};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioSpec {
    pub schema_version: u8,
    /// Stable kebab-case fixture ID. It is rendered in the scenario-mode UI
    /// marker and is never inferred from the input file name.
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub clock: ClockDto,
    #[serde(default)]
    pub backend: BackendDto,
    #[serde(default)]
    pub world: WorldDto,
    #[serde(default)]
    pub behaviour: BehaviourDto,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClockDto {
    #[serde(default)]
    pub start_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BackendDto {
    #[serde(default = "default_backend_id")]
    pub id: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

fn default_backend_id() -> String {
    "ui-scenario".into()
}

impl Default for BackendDto {
    fn default() -> Self {
        Self {
            id: default_backend_id(),
            capabilities: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorldDto {
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub disks: Vec<DiskDto>,
    #[serde(default)]
    pub partitions: Vec<PartitionDto>,
    #[serde(default)]
    pub filesystems: Vec<FilesystemDto>,
    #[serde(default)]
    pub luks: Vec<LuksDto>,
    #[serde(default)]
    pub processes: Vec<ProcessDto>,
    #[serde(default)]
    pub network: NetworkDto,
    #[serde(default)]
    pub workflows: WorkflowDto,
    #[serde(default)]
    pub logical: LogicalDto,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiskDto {
    pub id: String,
    pub device: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub size_bytes: u64,
    #[serde(default = "default_partition_table")]
    pub partition_table: String,
}

fn default_partition_table() -> String {
    "none".into()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PartitionDto {
    pub id: String,
    pub disk_id: String,
    pub device: String,
    pub start_bytes: u64,
    pub size_bytes: u64,
    #[serde(default = "default_partition_kind")]
    pub kind: String,
    #[serde(default)]
    pub name: String,
}

fn default_partition_kind() -> String {
    "primary".into()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FilesystemDto {
    pub id: String,
    pub device: String,
    #[serde(rename = "type")]
    pub fs_type: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub mount_points: Vec<String>,
    #[serde(default)]
    pub size_bytes: u64,
    #[serde(default)]
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LuksDto {
    pub id: String,
    pub device: String,
    #[serde(default = "default_luks_version")]
    pub version: String,
    pub mapper: String,
    /// A scenario-secret ID, never a passphrase.  Test code provides the
    /// corresponding value in memory when it needs to exercise unlock.
    pub secret_id: String,
    #[serde(default)]
    pub unlocked: bool,
    #[serde(default)]
    pub filesystem_id: Option<String>,
}

fn default_luks_version() -> String {
    "luks2".into()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessDto {
    pub id: String,
    pub mount: String,
    pub pid: i32,
    pub command: String,
    #[serde(default)]
    pub uid: u32,
    #[serde(default = "default_username")]
    pub username: String,
}

fn default_username() -> String {
    "fixture".into()
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkDto {
    #[serde(default)]
    pub providers: Vec<NetworkProviderDto>,
    #[serde(default)]
    pub configs: Vec<NetworkConfigDto>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkProviderDto {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub required_fields: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkConfigDto {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    #[serde(default)]
    pub options: BTreeMap<String, String>,
    #[serde(default)]
    pub mounted: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowDto {
    #[serde(default)]
    pub images: Vec<ImageAssetDto>,
    #[serde(default)]
    pub image_copy: Option<ProgressScriptDto>,
    #[serde(default)]
    pub usage_scan: Option<ProgressScriptDto>,
    #[serde(default)]
    pub usage_files: Vec<UsageFileDto>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImageAssetDto {
    pub id: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressScriptDto {
    pub total_bytes: u64,
    pub terminal: WorkflowTerminalDto,
    pub effects: Vec<ProgressEffectDto>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTerminalDto {
    Completed,
    Failed,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressEffectDto {
    pub at_ms: u64,
    pub completed_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UsageFileDto {
    pub id: String,
    pub mount: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LogicalDto {
    #[serde(default)]
    pub candidate_path: Option<String>,
    #[serde(default)]
    pub confirmation_message: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviourDto {
    #[serde(default)]
    pub rules: Vec<RuleDto>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuleDto {
    pub operation: String,
    #[serde(default)]
    pub selector: RuleSelectorDto,
    pub outcome: OutcomeDto,
    #[serde(default)]
    pub events: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSelectorDto {
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub filesystem_id: Option<String>,
    #[serde(default)]
    pub luks_id: Option<String>,
    #[serde(default)]
    pub config_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "tag", rename_all = "snake_case", deny_unknown_fields)]
pub enum OutcomeDto {
    Success,
    Error {
        kind: StorageErrorKind,
        message: String,
    },
    Unsupported,
    Delayed {
        effects: Vec<ProgressEffectDto>,
        terminal: WorkflowTerminalDto,
    },
}

pub fn parse_fixture(bytes: &[u8]) -> Result<(ScenarioSpec, String), StorageError> {
    first_key_is_schema_version(bytes)?;
    let spec: ScenarioSpec = toml::from_str(std::str::from_utf8(bytes).map_err(|error| {
        StorageError::new(
            StorageErrorKind::InvalidInput,
            format!("scenario fixture is not UTF-8: {error}"),
        )
    })?)
    .map_err(|error| {
        StorageError::new(
            StorageErrorKind::InvalidInput,
            format!("invalid scenario fixture: {error}"),
        )
    })?;
    if spec.schema_version != 2 {
        return Err(StorageError::new(
            StorageErrorKind::Unsupported,
            "scenario fixtures require schema_version = 2; v1 fixtures must be migrated",
        ));
    }
    validate_spec(&spec)?;
    Ok((spec, format!("{:x}", Sha256::digest(bytes))))
}

pub fn first_key_is_schema_version(bytes: &[u8]) -> Result<(), StorageError> {
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(StorageError::new(
            StorageErrorKind::InvalidInput,
            "scenario fixture must not include a UTF-8 BOM",
        ));
    }
    let text = std::str::from_utf8(bytes).map_err(|error| {
        StorageError::new(
            StorageErrorKind::InvalidInput,
            format!("scenario fixture is not UTF-8: {error}"),
        )
    })?;
    let first = text.lines().find(|line| {
        let trimmed = line.trim_start_matches([' ', '\t']);
        !trimmed.is_empty() && !trimmed.starts_with('#')
    });
    if first != Some("schema_version = 2") {
        return Err(StorageError::new(
            StorageErrorKind::InvalidInput,
            "schema_version = 2 must be the first non-comment fixture key",
        ));
    }
    Ok(())
}

fn validate_spec(spec: &ScenarioSpec) -> Result<(), StorageError> {
    if !is_safe_id(&spec.id) || spec.description.trim().is_empty() || !is_safe_id(&spec.backend.id)
    {
        return invalid(
            "scenario id/backend id must be kebab-case and description must be non-empty",
        );
    }
    if spec.backend.id != "ui-scenario" {
        return invalid("scenario backend.id must be ui-scenario");
    }
    validate_unique(
        spec.backend.capabilities.iter().map(String::as_str),
        "backend capability",
    )?;
    let known_capabilities = [
        "partition",
        "filesystem",
        "encryption",
        "logical",
        "network",
        "usage",
        "image",
    ];
    if spec
        .backend
        .capabilities
        .iter()
        .any(|capability| !known_capabilities.contains(&capability.as_str()))
    {
        return invalid("scenario declares an unknown backend capability");
    }

    validate_unique(
        spec.world.disks.iter().map(|item| item.id.as_str()),
        "disk id",
    )?;
    validate_unique(
        spec.world.partitions.iter().map(|item| item.id.as_str()),
        "partition id",
    )?;
    validate_unique(
        spec.world.filesystems.iter().map(|item| item.id.as_str()),
        "filesystem id",
    )?;
    validate_unique(
        spec.world.luks.iter().map(|item| item.id.as_str()),
        "luks id",
    )?;
    validate_unique(
        spec.world.processes.iter().map(|item| item.id.as_str()),
        "process id",
    )?;
    validate_unique(
        spec.world
            .network
            .configs
            .iter()
            .map(|item| item.id.as_str()),
        "network config id",
    )?;
    validate_unique(
        spec.world
            .network
            .providers
            .iter()
            .map(|item| item.id.as_str()),
        "network provider id",
    )?;
    validate_unique(
        spec.world
            .workflows
            .images
            .iter()
            .map(|item| item.id.as_str()),
        "image asset id",
    )?;
    validate_unique(
        spec.world
            .workflows
            .usage_files
            .iter()
            .map(|item| item.id.as_str()),
        "usage file id",
    )?;

    let disk_ids = spec
        .world
        .disks
        .iter()
        .map(|item| item.id.as_str())
        .collect::<BTreeSet<_>>();
    let filesystem_ids = spec
        .world
        .filesystems
        .iter()
        .map(|item| item.id.as_str())
        .collect::<BTreeSet<_>>();
    for disk in &spec.world.disks {
        if !is_safe_id(&disk.id)
            || !is_synthetic_device(&disk.device)
            || !matches!(disk.partition_table.as_str(), "none" | "gpt")
        {
            return invalid("disk IDs/devices/partition tables are invalid");
        }
    }
    for partition in &spec.world.partitions {
        if !is_safe_id(&partition.id)
            || !disk_ids.contains(partition.disk_id.as_str())
            || !is_synthetic_device(&partition.device)
            || partition.size_bytes == 0
            || !partition.start_bytes.is_multiple_of(1_048_576)
            || !partition.size_bytes.is_multiple_of(1_048_576)
        {
            return invalid("partition has an invalid ID, parent, device, or byte alignment");
        }
    }
    for filesystem in &spec.world.filesystems {
        if !is_safe_id(&filesystem.id)
            || !is_synthetic_device(&filesystem.device)
            || filesystem.fs_type.is_empty()
            || filesystem
                .mount_points
                .iter()
                .any(|mount| !is_synthetic_mount(mount))
        {
            return invalid("filesystem has an invalid ID, device, type, or mount point");
        }
    }
    for luks in &spec.world.luks {
        if !is_safe_id(&luks.id)
            || !is_synthetic_device(&luks.device)
            || !is_synthetic_mapper(&luks.mapper)
            || !is_safe_id(&luks.secret_id)
            || !matches!(luks.version.as_str(), "luks1" | "luks2")
            || luks
                .filesystem_id
                .as_deref()
                .is_some_and(|id| !filesystem_ids.contains(id))
        {
            return invalid("luks entry has an invalid reference or exposes a secret");
        }
    }
    for process in &spec.world.processes {
        if !is_safe_id(&process.id)
            || process.pid <= 0
            || process.command.trim().is_empty()
            || !is_synthetic_mount(&process.mount)
        {
            return invalid("process entry is invalid");
        }
    }
    for config in &spec.world.network.configs {
        if !is_safe_id(&config.id)
            || !is_safe_id(&config.provider_id)
            || !spec
                .world
                .network
                .providers
                .iter()
                .any(|provider| provider.id == config.provider_id)
        {
            return invalid("network config has an unknown provider or invalid ID");
        }
    }
    for asset in &spec.world.workflows.images {
        if !is_safe_id(&asset.id) {
            return invalid("image asset id is invalid");
        }
    }
    for file in &spec.world.workflows.usage_files {
        if !is_safe_id(&file.id) || !is_synthetic_mount(&file.mount) {
            return invalid("usage file has an invalid ID or mount");
        }
    }
    validate_progress_script(spec.world.workflows.image_copy.as_ref())?;
    validate_progress_script(spec.world.workflows.usage_scan.as_ref())?;

    let mut rule_keys = BTreeSet::new();
    for rule in &spec.behaviour.rules {
        if storage_contracts::ScenarioOperation::parse(&rule.operation).is_none() {
            return invalid(format!("unknown scenario operation '{}'", rule.operation));
        }
        if !rule_keys.insert((rule.operation.as_str(), rule.selector.clone())) {
            return invalid("ambiguous scenario behaviour rules are not allowed");
        }
        if let OutcomeDto::Delayed { effects, .. } = &rule.outcome {
            validate_progress_effects(effects)?;
        }
    }
    Ok(())
}

fn validate_progress_script(script: Option<&ProgressScriptDto>) -> Result<(), StorageError> {
    if let Some(script) = script {
        if script.total_bytes == 0 || script.effects.is_empty() {
            return invalid("workflow progress scripts require bytes and at least one effect");
        }
        validate_progress_effects(&script.effects)?;
        if script
            .effects
            .last()
            .is_none_or(|effect| effect.completed_bytes != script.total_bytes)
        {
            return invalid("workflow progress scripts must finish at total_bytes");
        }
    }
    Ok(())
}

fn validate_progress_effects(effects: &[ProgressEffectDto]) -> Result<(), StorageError> {
    let mut last_time = None;
    let mut last_bytes = None;
    for effect in effects {
        if last_time.is_some_and(|time| effect.at_ms <= time)
            || last_bytes.is_some_and(|bytes| effect.completed_bytes < bytes)
        {
            return invalid(
                "progress effects must have strictly increasing time and monotonic bytes",
            );
        }
        last_time = Some(effect.at_ms);
        last_bytes = Some(effect.completed_bytes);
    }
    Ok(())
}

fn validate_unique<'a>(
    values: impl Iterator<Item = &'a str>,
    kind: &str,
) -> Result<(), StorageError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return invalid(format!("duplicate {kind}"));
        }
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> Result<(), StorageError> {
    Err(StorageError::new(StorageErrorKind::InvalidInput, message))
}

fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 63
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn is_synthetic_device(value: &str) -> bool {
    (value.starts_with("/dev/ui-") || value.starts_with("/dev/mapper/ui-")) && !value.contains("..")
}

fn is_synthetic_mapper(value: &str) -> bool {
    value.starts_with("/dev/mapper/ui-") && !value.contains("..")
}

fn is_synthetic_mount(value: &str) -> bool {
    value.starts_with("/mnt/ui-") && !value.contains("..")
}

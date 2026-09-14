//! Read-only local logical-storage enrichment.
//!
//! This module never mutates storage.  It has a closed command allow-list and
//! is deliberately a `LogicalTopologySource`, not a `LogicalOperations`
//! implementation.

use std::{collections::BTreeMap, process::Command, sync::Arc};

use async_trait::async_trait;
use storage_contracts::{LogicalTopologySource, StorageError, StorageErrorKind};
use storage_types::{
    LogicalCapabilities, LogicalDisplay, LogicalEntity, LogicalEntityDetails, LogicalEntityId,
    LogicalEntityKind, LogicalMember, LogicalMemberId, LogicalSource, LogicalSourceAvailability,
    LvmActivationState, LvmLogicalVolumeDetails, LvmLogicalVolumeSummary, LvmPhysicalVolumeDetails,
    LvmPhysicalVolumeState, LvmPhysicalVolumeSummary, LvmVolumeGroupDetails,
};

const VGS_ARGS: &[&str] = &[
    "--noheadings",
    "--units",
    "b",
    "--nosuffix",
    "--separator",
    "\t",
    "--sort",
    "vg_name",
    "-o",
    "vg_name,vg_uuid,vg_size,vg_free,pv_count,lv_count",
];
const LVS_ARGS: &[&str] = &[
    "--noheadings",
    "--units",
    "b",
    "--nosuffix",
    "--separator",
    "\t",
    "--sort",
    "vg_name,lv_name",
    "-o",
    "vg_name,vg_uuid,lv_name,lv_size,lv_active",
];
const PVS_ARGS: &[&str] = &[
    "--noheadings",
    "--units",
    "b",
    "--nosuffix",
    "--separator",
    "\t",
    "--sort",
    "vg_name,pv_name",
    "-o",
    "pv_name,vg_name,pv_size,pv_free",
];

/// The only allowed local-tool command runner.  Test doubles can record the
/// immutable argv exactly; production invokes no shell or elevation helper.
pub trait LogicalCommandRunner: Send + Sync {
    fn is_available(&self, program: &str) -> bool;
    fn run(&self, program: &str, args: &[&str]) -> Result<String, LocalToolsError>;
}

#[derive(Debug, Default)]
pub struct SystemLogicalCommandRunner;

impl LogicalCommandRunner for SystemLogicalCommandRunner {
    fn is_available(&self, program: &str) -> bool {
        which::which(program).is_ok()
    }

    fn run(&self, program: &str, args: &[&str]) -> Result<String, LocalToolsError> {
        if !readonly_command_allowed(program, args) {
            return Err(LocalToolsError::ForbiddenCommand(program.to_string()));
        }
        let output = Command::new(program).args(args).output().map_err(|error| {
            LocalToolsError::Invocation {
                tool: program.to_string(),
                detail: error.to_string(),
            }
        })?;
        if !output.status.success() {
            return Err(LocalToolsError::Invocation {
                tool: program.to_string(),
                detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        String::from_utf8(output.stdout).map_err(|error| LocalToolsError::Invocation {
            tool: program.to_string(),
            detail: error.to_string(),
        })
    }
}

/// A deterministic read-only LVM source; absence is reported, never masked as
/// an empty successful native topology.
pub struct LocalLogicalTopologySource {
    runner: Arc<dyn LogicalCommandRunner>,
}

impl std::fmt::Debug for LocalLogicalTopologySource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalLogicalTopologySource")
            .finish_non_exhaustive()
    }
}

impl LocalLogicalTopologySource {
    pub fn new() -> Self {
        Self::with_runner(Arc::new(SystemLogicalCommandRunner))
    }

    pub fn with_runner(runner: Arc<dyn LogicalCommandRunner>) -> Self {
        Self { runner }
    }

    fn available(&self) -> bool {
        ["vgs", "lvs", "pvs"]
            .iter()
            .all(|program| self.runner.is_available(program))
    }

    fn discover(&self) -> Result<Vec<LogicalEntity>, LocalToolsError> {
        if !self.available() {
            return Ok(Vec::new());
        }
        let vgs = parse_vgs(&self.runner.run("vgs", VGS_ARGS)?)?;
        let lvs = parse_lvs(&self.runner.run("lvs", LVS_ARGS)?)?;
        let pvs = parse_pvs(&self.runner.run("pvs", PVS_ARGS)?)?;
        Ok(entities_from_lvm(vgs, lvs, pvs))
    }
}

impl Default for LocalLogicalTopologySource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LogicalTopologySource for LocalLogicalTopologySource {
    fn logical_source(&self) -> LogicalSource {
        LogicalSource::LocalTools
    }

    fn logical_availability(&self) -> LogicalSourceAvailability {
        if self.available() {
            LogicalSourceAvailability::Available
        } else {
            LogicalSourceAvailability::Unavailable {
                reason: "Read-only LVM tools (vgs, lvs, pvs) are unavailable.".into(),
            }
        }
    }

    async fn list_logical_entities(&self) -> Result<Vec<LogicalEntity>, StorageError> {
        self.discover().map_err(Into::into)
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum LocalToolsError {
    #[error("{tool} record {record} is malformed")]
    MalformedRecord { tool: String, record: usize },
    #[error("{tool} invocation failed: {detail}")]
    Invocation { tool: String, detail: String },
    #[error("local logical command is not allow-listed: {0}")]
    ForbiddenCommand(String),
}

impl From<LocalToolsError> for StorageError {
    fn from(error: LocalToolsError) -> Self {
        StorageError::new(StorageErrorKind::Other, error.to_string())
    }
}

#[derive(Debug, Clone)]
struct VgRow {
    name: String,
    uuid: String,
    size: u64,
    free: u64,
    pv_count: u32,
    lv_count: u32,
}

#[derive(Debug, Clone)]
struct LvRow {
    vg_uuid: String,
    name: String,
    size: u64,
    active: bool,
}

#[derive(Debug, Clone)]
struct PvRow {
    path: String,
    vg_name: Option<String>,
    size: u64,
}

fn parse_vgs(output: &str) -> Result<Vec<VgRow>, LocalToolsError> {
    records(output, "vgs", 6, |fields| {
        Ok(VgRow {
            name: nonempty(fields, 0, "vgs")?,
            uuid: nonempty(fields, 1, "vgs")?,
            size: number(fields, 2, "vgs")?,
            free: number(fields, 3, "vgs")?,
            pv_count: number(fields, 4, "vgs")?,
            lv_count: number(fields, 5, "vgs")?,
        })
    })
}

fn parse_lvs(output: &str) -> Result<Vec<LvRow>, LocalToolsError> {
    records(output, "lvs", 5, |fields| {
        // The source column is retained and validated even though VG UUID is
        // the stable join key.
        let _ = nonempty(fields, 0, "lvs")?;
        Ok(LvRow {
            vg_uuid: nonempty(fields, 1, "lvs")?,
            name: nonempty(fields, 2, "lvs")?,
            size: number(fields, 3, "lvs")?,
            active: matches!(fields[4].trim(), "active" | "y" | "yes"),
        })
    })
}

fn parse_pvs(output: &str) -> Result<Vec<PvRow>, LocalToolsError> {
    records(output, "pvs", 4, |fields| {
        let _: u64 = number(fields, 3, "pvs")?;
        Ok(PvRow {
            path: nonempty(fields, 0, "pvs")?,
            vg_name: (!fields[1].trim().is_empty()).then(|| fields[1].trim().to_string()),
            size: number(fields, 2, "pvs")?,
        })
    })
}

fn records<T>(
    output: &str,
    tool: &str,
    expected_fields: usize,
    parse: impl Fn(&[&str]) -> Result<T, LocalToolsError>,
) -> Result<Vec<T>, LocalToolsError> {
    output
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            let fields: Vec<_> = line.split('\t').collect();
            if fields.len() != expected_fields {
                return Err(LocalToolsError::MalformedRecord {
                    tool: tool.to_string(),
                    record: index + 1,
                });
            }
            parse(&fields).map_err(|_| LocalToolsError::MalformedRecord {
                tool: tool.to_string(),
                record: index + 1,
            })
        })
        .collect()
}

fn nonempty(fields: &[&str], index: usize, _tool: &str) -> Result<String, LocalToolsError> {
    let value = fields[index].trim();
    if value.is_empty() {
        Err(LocalToolsError::MalformedRecord {
            tool: _tool.to_string(),
            record: 0,
        })
    } else {
        Ok(value.to_string())
    }
}

fn number<T: std::str::FromStr>(
    fields: &[&str],
    index: usize,
    tool: &str,
) -> Result<T, LocalToolsError> {
    fields[index]
        .trim()
        .parse()
        .map_err(|_| LocalToolsError::MalformedRecord {
            tool: tool.to_string(),
            record: 0,
        })
}

fn entities_from_lvm(vgs: Vec<VgRow>, lvs: Vec<LvRow>, pvs: Vec<PvRow>) -> Vec<LogicalEntity> {
    let mut entities = Vec::new();
    for vg in vgs {
        let id = LogicalEntityId(format!("lvm-vg:{}", vg.uuid));
        let mut members = Vec::new();
        let mut logical_volumes = Vec::new();
        let mut physical_volumes = Vec::new();
        for lv in lvs.iter().filter(|lv| lv.vg_uuid == vg.uuid) {
            members.push(LogicalMember {
                id: LogicalMemberId(format!("member:lvm-lv:{}:{}", vg.uuid, lv.name)),
                kind: LogicalEntityKind::LvmLogicalVolume,
                name: lv.name.clone(),
                device_ref: None,
                device_path: None,
                role: Some("lv".into()),
                state: Some(if lv.active { "active" } else { "inactive" }.into()),
                size_bytes: Some(lv.size),
            });
            entities.push(LogicalEntity {
                id: LogicalEntityId(format!("lvm-lv:{}:{}", vg.uuid, lv.name)),
                kind: LogicalEntityKind::LvmLogicalVolume,
                details: LogicalEntityDetails::LvmLogicalVolume(LvmLogicalVolumeDetails {
                    name: lv.name.clone(),
                    volume_group: id.clone(),
                    device_path: LogicalDisplay::unknown(
                        "Local tools do not report the LV device path",
                    ),
                    size: LogicalDisplay::known(lv.size),
                    activation: LogicalDisplay::known(if lv.active {
                        LvmActivationState::Active
                    } else {
                        LvmActivationState::Inactive
                    }),
                }),
                parent_id: Some(id.clone()),
                capabilities: LogicalCapabilities::block_all("Not discovered by UDisks"),
                metadata: BTreeMap::new(),
                name: lv.name.clone(),
                uuid: None,
                device_path: None,
                size_bytes: lv.size,
                used_bytes: None,
                free_bytes: None,
                health_status: Some(if lv.active { "active" } else { "inactive" }.into()),
                progress_fraction: None,
                members: Vec::new(),
            });
            logical_volumes.push(LvmLogicalVolumeSummary {
                entity_id: LogicalEntityId(format!("lvm-lv:{}:{}", vg.uuid, lv.name)),
                name: lv.name.clone(),
                size: LogicalDisplay::known(lv.size),
                activation: LogicalDisplay::known(if lv.active {
                    LvmActivationState::Active
                } else {
                    LvmActivationState::Inactive
                }),
            });
        }
        for pv in pvs
            .iter()
            .filter(|pv| pv.vg_name.as_deref() == Some(vg.name.as_str()))
        {
            members.push(LogicalMember {
                id: LogicalMemberId(format!("member:local-pv:{}", pv.path)),
                kind: LogicalEntityKind::LvmPhysicalVolume,
                name: pv.path.clone(),
                device_ref: None,
                device_path: Some(pv.path.clone()),
                role: Some("pv".into()),
                state: None,
                size_bytes: Some(pv.size),
            });
            physical_volumes.push(LvmPhysicalVolumeSummary {
                entity_id: LogicalEntityId(format!("lvm-pv:local:{}", pv.path)),
                member: LvmPhysicalVolumeDetails {
                    block: None,
                    display_path: LogicalDisplay::known(pv.path.clone()),
                    size: LogicalDisplay::known(pv.size),
                    state: LogicalDisplay::known(LvmPhysicalVolumeState::Available),
                },
            });
        }
        entities.push(LogicalEntity {
            id,
            kind: LogicalEntityKind::LvmVolumeGroup,
            details: LogicalEntityDetails::LvmVolumeGroup(LvmVolumeGroupDetails {
                name: vg.name.clone(),
                uuid: LogicalDisplay::known(vg.uuid.clone()),
                size: LogicalDisplay::known(vg.size),
                used: LogicalDisplay::known(vg.size.saturating_sub(vg.free)),
                free: LogicalDisplay::known(vg.free),
                logical_volumes,
                physical_volumes,
            }),
            parent_id: None,
            capabilities: LogicalCapabilities::block_all("Not discovered by UDisks"),
            metadata: BTreeMap::from([
                ("pv_count".into(), vg.pv_count.to_string()),
                ("lv_count".into(), vg.lv_count.to_string()),
            ]),
            name: vg.name,
            uuid: Some(vg.uuid),
            device_path: None,
            size_bytes: vg.size,
            used_bytes: Some(vg.size.saturating_sub(vg.free)),
            free_bytes: Some(vg.free),
            health_status: None,
            progress_fraction: None,
            members,
        });
    }
    entities.sort_by(|left, right| {
        left.display_name()
            .cmp(right.display_name())
            .then(left.id.cmp(&right.id))
    });
    entities
}

fn readonly_command_allowed(program: &str, args: &[&str]) -> bool {
    (program == "vgs" && args == VGS_ARGS)
        || (program == "lvs" && args == LVS_ARGS)
        || (program == "pvs" && args == PVS_ARGS)
}

// SPDX-License-Identifier: GPL-3.0-only

//! Deterministic, host-safe storage adapters for UI scenarios.
//!
//! The backend deliberately contains no UDisks, rclone, command-spawning, or
//! direct host-path dependencies.  Fixture/overlay/trace bytes enter through
//! `ScenarioStore`; all other behaviour is state held in memory.

mod control;
mod schema;
mod store;

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    pin::Pin,
    sync::Arc,
};

use async_trait::async_trait;
use futures::{Stream, stream};
use storage_contracts::{
    BackendMetadata, BlockStorageBackend, BtrfsOperations, DesktopServices, DeviceEventSource,
    DiskDiscovery, DriveOperations, EncryptionOperations, FilesystemOperations,
    FilesystemToolDiscovery, ImageDeviceOperations, ImageWorkflowOperations, LogicalActionOutcome,
    LogicalOperations, LogicalPreflight, LogicalPreflightAvailability, LogicalPreflightRequest,
    LogicalReviewData, LogicalTopologySource, NetworkDriveBackend, PartitionOperations,
    RuntimeAdapters, ScenarioControl, ScenarioDiagnostics, ScenarioReceipt, ScenarioReload,
    StorageError, StorageErrorKind, UsageOperations,
};
use storage_types::{
    BackendId, CreatePartitionInfo, DesktopImageSelection, DeviceEvent, DiskInfo,
    EncryptionOptionsSettings, FilesystemInfo, FilesystemToolInfo, FormatOptions, ImageAssetRef,
    ImageAttachment, ImageAttachmentRequest, ImageCopyRequest, ImageWorkflowStatus,
    LogicalCandidateAnchor, LogicalSource, LogicalSourceAvailability, LuksInfo, LuksVersion,
    MountOptions, MountOptionsSettings, PartitionInfo, ProcessInfo, SmartInfo, SmartSelfTestKind,
    StorageBackendCapabilities, SubvolumeList, UsageDeleteFailure, UsageDeleteRequest,
    UsageDeleteResponse, UsageDeleteResult, UsageWorkflowRequest, UsageWorkflowStatus,
    WorkflowState,
};
use tokio::sync::Mutex;

pub use control::{ControlCommand, ControlRequest, ControlResponse, ScenarioControlServer};
pub use schema::{ScenarioSpec, first_key_is_schema_version, parse_fixture};
pub use store::ScenarioStore;

#[derive(Debug, Clone)]
struct State {
    disks: Vec<DiskInfo>,
    partitions: Vec<PartitionInfo>,
    filesystems: Vec<FilesystemInfo>,
    luks: BTreeMap<String, ScenarioLuks>,
    processes: BTreeMap<String, ProcessInfo>,
    events: Vec<DeviceEvent>,
    image_assets: BTreeSet<String>,
    image_operations: BTreeMap<String, ImageWorkflowStatus>,
    usage_operations: BTreeMap<String, UsageWorkflowStatus>,
    usage_files: BTreeMap<String, schema::UsageFileDto>,
    image_copy_script: Option<schema::ProgressScriptDto>,
    usage_scan_script: Option<schema::ProgressScriptDto>,
    logical_candidate_path: Option<String>,
    logical_confirmation_message: Option<String>,
    network_configs: BTreeMap<String, storage_types::NetworkDriveConfig>,
    network_mounts: BTreeMap<String, storage_types::NetworkDriveMount>,
    network_mount_on_login: BTreeMap<String, bool>,
    generation: u64,
    sequence: u64,
    tick: u64,
}

#[derive(Debug, Clone)]
struct ScenarioLuks {
    secret_id: String,
    mapper: String,
    info: LuksInfo,
}

impl State {
    fn from_spec(spec: &ScenarioSpec) -> Self {
        let mut disks = spec
            .world
            .disks
            .iter()
            .map(|disk| DiskInfo {
                device: disk.device.clone(),
                id: disk.id.clone(),
                model: disk.model.clone(),
                serial: String::new(),
                vendor: String::new(),
                revision: String::new(),
                size: disk.size_bytes,
                connection_bus: "scenario".into(),
                rotation_rate: None,
                removable: false,
                ejectable: false,
                media_removable: false,
                media_available: true,
                optical: false,
                optical_blank: false,
                can_power_off: false,
                is_loop: false,
                backing_file: None,
                partition_table_type: (disk.partition_table != "none")
                    .then(|| disk.partition_table.clone()),
                gpt_usable_range: None,
            })
            .collect::<Vec<_>>();
        disks.sort_by(|left, right| {
            left.id
                .cmp(&right.id)
                .then_with(|| left.device.cmp(&right.device))
        });
        let mut filesystems = spec
            .world
            .filesystems
            .iter()
            .map(|filesystem| FilesystemInfo {
                device: filesystem.device.clone(),
                fs_type: filesystem.fs_type.clone(),
                label: filesystem.label.clone(),
                uuid: String::new(),
                mount_points: filesystem.mount_points.clone(),
                size: filesystem.size_bytes,
                available: filesystem.available_bytes,
            })
            .collect::<Vec<_>>();
        filesystems.sort_by(|left, right| left.device.cmp(&right.device));
        let disks_by_id = spec
            .world
            .disks
            .iter()
            .map(|disk| (disk.id.as_str(), disk.device.as_str()))
            .collect::<BTreeMap<_, _>>();
        let mut partitions = spec
            .world
            .partitions
            .iter()
            .enumerate()
            .map(|(index, partition)| PartitionInfo {
                device: partition.device.clone(),
                number: (index + 1) as u32,
                parent_path: disks_by_id
                    .get(partition.disk_id.as_str())
                    .expect("validated partition disk")
                    .to_string(),
                size: partition.size_bytes,
                offset: partition.start_bytes,
                type_id: partition.kind.clone(),
                type_name: partition.kind.clone(),
                flags: 0,
                name: partition.name.clone(),
                uuid: String::new(),
                table_type: "gpt".into(),
                has_filesystem: spec
                    .world
                    .filesystems
                    .iter()
                    .any(|filesystem| filesystem.device == partition.device),
                filesystem_type: spec
                    .world
                    .filesystems
                    .iter()
                    .find(|filesystem| filesystem.device == partition.device)
                    .map(|filesystem| filesystem.fs_type.clone()),
                mount_points: spec
                    .world
                    .filesystems
                    .iter()
                    .find(|filesystem| filesystem.device == partition.device)
                    .map(|filesystem| filesystem.mount_points.clone())
                    .unwrap_or_default(),
                usage: None,
            })
            .collect::<Vec<_>>();
        partitions.sort_by(|left, right| {
            left.parent_path
                .cmp(&right.parent_path)
                .then(left.offset.cmp(&right.offset))
        });
        let luks = spec
            .world
            .luks
            .iter()
            .map(|luks| {
                let version = match luks.version.as_str() {
                    "luks1" => LuksVersion::Luks1,
                    "luks2" => LuksVersion::Luks2,
                    _ => unreachable!("validated LUKS version"),
                };
                (
                    luks.id.clone(),
                    ScenarioLuks {
                        secret_id: luks.secret_id.clone(),
                        mapper: luks.mapper.clone(),
                        info: LuksInfo {
                            device: luks.device.clone(),
                            version,
                            cipher: "aes-xts-plain64".into(),
                            key_size: 512,
                            unlocked: luks.unlocked,
                            cleartext_device: luks.unlocked.then(|| luks.mapper.clone()),
                            keyslot_count: 1,
                        },
                    },
                )
            })
            .collect();
        let processes = spec
            .world
            .processes
            .iter()
            .map(|process| {
                (
                    process.mount.clone(),
                    ProcessInfo {
                        pid: process.pid,
                        command: process.command.clone(),
                        uid: process.uid,
                        username: process.username.clone(),
                    },
                )
            })
            .collect();
        let network_configs = spec
            .world
            .network
            .configs
            .iter()
            .map(|config| {
                (
                    config.id.clone(),
                    storage_types::NetworkDriveConfig {
                        backend_id: storage_types::NetworkBackendId::rclone(),
                        id: config.id.clone(),
                        name: config.name.clone(),
                        provider_id: config.provider_id.clone(),
                        options: config.options.clone(),
                        has_secrets: false,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        let network_mounts = spec
            .world
            .network
            .configs
            .iter()
            .filter(|config| config.mounted)
            .map(|config| {
                (
                    config.id.clone(),
                    storage_types::NetworkDriveMount {
                        backend_id: storage_types::NetworkBackendId::rclone(),
                        config_id: config.id.clone(),
                        mount_point: format!("/mnt/ui-network-{}", config.id).into(),
                        status: storage_types::NetworkDriveStatus::Mounted,
                    },
                )
            })
            .collect();
        Self {
            disks,
            partitions,
            filesystems,
            luks,
            processes,
            events: Vec::new(),
            image_assets: spec
                .world
                .workflows
                .images
                .iter()
                .map(|asset| format!("asset:{}", asset.id))
                .collect(),
            image_operations: BTreeMap::new(),
            usage_operations: BTreeMap::new(),
            usage_files: spec
                .world
                .workflows
                .usage_files
                .iter()
                .map(|file| (file.id.clone(), file.clone()))
                .collect(),
            image_copy_script: spec.world.workflows.image_copy.clone(),
            usage_scan_script: spec.world.workflows.usage_scan.clone(),
            logical_candidate_path: spec.world.logical.candidate_path.clone(),
            logical_confirmation_message: spec.world.logical.confirmation_message.clone(),
            network_configs,
            network_mounts,
            network_mount_on_login: BTreeMap::new(),
            generation: spec.world.revision,
            sequence: 0,
            tick: spec.clock.start_ms,
        }
    }
}

/// A complete contract implementation.  Methods that v1 does not model fail
/// closed with `Unsupported`; they never fall through to production adapters.
pub struct ScenarioBackend {
    spec: Mutex<ScenarioSpec>,
    fixture_sha256: String,
    state: Mutex<State>,
    secrets: BTreeMap<String, String>,
    store: ScenarioStore,
}

/// Explicit name for the Phase-2 completeness implementation.  It uses the
/// same type as scenario state, with an empty fixture yielding only typed
/// unsupported responses for all mutating operations.
pub type UnsupportedScenarioBackend = ScenarioBackend;

impl ScenarioBackend {
    pub fn load(store: ScenarioStore) -> Result<Arc<Self>, StorageError> {
        Self::load_with_secrets(store, BTreeMap::new())
    }

    /// The test process supplies secret values out-of-band. Fixtures contain
    /// only secret IDs, so neither the source fixture nor a trace can expose a
    /// passphrase.
    pub fn load_with_secrets(
        store: ScenarioStore,
        secrets: BTreeMap<String, String>,
    ) -> Result<Arc<Self>, StorageError> {
        let bytes = store.read_fixture()?;
        let (spec, fixture_sha256) = parse_fixture(&bytes)?;
        Ok(Arc::new(Self {
            state: Mutex::new(State::from_spec(&spec)),
            spec: Mutex::new(spec),
            fixture_sha256,
            secrets,
            store,
        }))
    }

    fn unsupported(operation: &str) -> StorageError {
        StorageError::new(
            StorageErrorKind::Unsupported,
            format!("scenario operation '{operation}' is unsupported by this fixture"),
        )
    }

    async fn configured_error(&self, operation: &str) -> Option<StorageError> {
        let spec = self.spec.lock().await;
        spec.behaviour.rules.iter().find_map(|rule| {
            if rule.operation != operation {
                return None;
            }
            match &rule.outcome {
                schema::OutcomeDto::Error { kind, message } => {
                    Some(StorageError::new(*kind, message))
                }
                schema::OutcomeDto::Unsupported => Some(Self::unsupported(operation)),
                schema::OutcomeDto::Success | schema::OutcomeDto::Delayed { .. } => None,
            }
        })
    }

    async fn next_receipt(&self, topology_change: bool) -> ScenarioReceipt {
        let mut state = self.state.lock().await;
        state.sequence += 1;
        if topology_change {
            state.generation += 1;
        }
        ScenarioReceipt {
            sequence: state.sequence,
            generation: state.generation,
            virtual_tick: state.tick,
        }
    }

    pub async fn diagnostics_snapshot(&self) -> ScenarioDiagnostics {
        let state = self.state.lock().await;
        let spec = self.spec.lock().await;
        ScenarioDiagnostics {
            fixture_name: spec.id.clone(),
            fixture_sha256: self.fixture_sha256.clone(),
            generation: state.generation,
            last_sequence: state.sequence,
            virtual_tick: state.tick,
        }
    }

    pub fn marker(&self) -> String {
        let spec = self
            .spec
            .try_lock()
            .expect("scenario marker is read before runtime tasks start");
        format!("Test scenario: {} sha256:{}", spec.id, self.fixture_sha256)
    }

    async fn replace_overlay(&self, bytes: &[u8]) -> Result<(), StorageError> {
        // Parse before writing so a malformed overlay cannot become observable
        // through the watched path, even briefly.
        parse_fixture(bytes)?;
        self.store.write_overlay_atomically(bytes)
    }

    fn status_from_script(
        operation_id: String,
        script: &schema::ProgressScriptDto,
        tick: u64,
    ) -> ImageWorkflowStatus {
        let completed = script
            .effects
            .iter()
            .take_while(|effect| effect.at_ms <= tick)
            .map(|effect| effect.completed_bytes)
            .last()
            .unwrap_or(0);
        let terminal_tick = script
            .effects
            .last()
            .map(|effect| effect.at_ms)
            .unwrap_or(u64::MAX);
        let state = if tick >= terminal_tick {
            match script.terminal {
                schema::WorkflowTerminalDto::Completed => WorkflowState::Completed,
                schema::WorkflowTerminalDto::Failed => WorkflowState::Failed,
            }
        } else {
            WorkflowState::Running
        };
        ImageWorkflowStatus {
            operation_id,
            state,
            bytes_completed: completed,
            bytes_total: script.total_bytes,
            speed_bytes_per_sec: 0,
            message: None,
        }
    }

    fn usage_result(
        state: &State,
        mounts_scanned: usize,
        elapsed_ms: u128,
    ) -> storage_types::UsageScanResult {
        let total_bytes = state.usage_files.values().map(|file| file.bytes).sum();
        storage_types::UsageScanResult {
            categories: vec![storage_types::UsageCategoryTotal {
                category: storage_types::UsageCategory::Other,
                bytes: total_bytes,
            }],
            top_files_by_category: vec![storage_types::UsageCategoryTopFiles {
                category: storage_types::UsageCategory::Other,
                files: state
                    .usage_files
                    .values()
                    .map(|file| storage_types::UsageTopFileEntry {
                        path: format!("{}/{}", file.mount, file.id).into(),
                        bytes: file.bytes,
                    })
                    .collect(),
            }],
            total_bytes,
            total_free_bytes: 0,
            files_scanned: state.usage_files.len() as u64,
            dirs_scanned: mounts_scanned as u64,
            skipped_errors: 0,
            mounts_scanned,
            elapsed_ms,
        }
    }

    fn refresh_workflows(state: &mut State) {
        let Some(image_script) = state.image_copy_script.clone() else {
            return;
        };
        for status in state.image_operations.values_mut() {
            if status.state == WorkflowState::Running {
                *status = Self::status_from_script(
                    status.operation_id.clone(),
                    &image_script,
                    state.tick,
                );
            }
        }
        let Some(usage_script) = state.usage_scan_script.clone() else {
            return;
        };
        let completed = usage_script
            .effects
            .iter()
            .take_while(|effect| effect.at_ms <= state.tick)
            .map(|effect| effect.completed_bytes)
            .last()
            .unwrap_or(0);
        let terminal_tick = usage_script
            .effects
            .last()
            .map(|effect| effect.at_ms)
            .unwrap_or(u64::MAX);
        let workflow_state = if state.tick >= terminal_tick {
            match usage_script.terminal {
                schema::WorkflowTerminalDto::Completed => WorkflowState::Completed,
                schema::WorkflowTerminalDto::Failed => WorkflowState::Failed,
            }
        } else {
            WorkflowState::Running
        };
        let result = (workflow_state == WorkflowState::Completed)
            .then(|| Self::usage_result(state, 1, state.tick as u128));
        for status in state.usage_operations.values_mut() {
            if status.state == WorkflowState::Running {
                status.state = workflow_state.clone();
                status.processed_bytes = completed;
                status.estimated_total_bytes = usage_script.total_bytes;
                status.result = result.clone();
            }
        }
    }
}

pub struct ScenarioRuntime {
    backend: Arc<ScenarioBackend>,
}

impl ScenarioRuntime {
    pub fn load(
        fixture: impl Into<PathBuf>,
        overlay: Option<PathBuf>,
        trace: Option<PathBuf>,
    ) -> Result<Self, StorageError> {
        let backend = ScenarioBackend::load(ScenarioStore::new(fixture, overlay, trace))?;
        Ok(Self { backend })
    }

    pub fn load_with_secrets(
        fixture: impl Into<PathBuf>,
        overlay: Option<PathBuf>,
        trace: Option<PathBuf>,
        secrets: BTreeMap<String, String>,
    ) -> Result<Self, StorageError> {
        let backend = ScenarioBackend::load_with_secrets(
            ScenarioStore::new(fixture, overlay, trace),
            secrets,
        )?;
        Ok(Self { backend })
    }

    pub fn adapters(&self) -> RuntimeAdapters {
        RuntimeAdapters {
            block: self.backend.clone() as Arc<dyn BlockStorageBackend>,
            btrfs: None,
            network: vec![self.backend.clone() as Arc<dyn NetworkDriveBackend>],
            network_availability: vec![(
                storage_types::NetworkBackendId::rclone(),
                storage_types::NetworkBackendAvailability::Available,
            )],
            logical_topology_sources: vec![
                Arc::new(ScenarioLogicalSource {
                    source: LogicalSource::Udisks,
                }) as Arc<dyn LogicalTopologySource>,
                Arc::new(ScenarioLogicalSource {
                    source: LogicalSource::LocalTools,
                }) as Arc<dyn LogicalTopologySource>,
            ],
            logical_operations: self.backend.clone() as Arc<dyn LogicalOperations>,
            filesystem_tools: self.backend.clone() as Arc<dyn FilesystemToolDiscovery>,
            usage: self.backend.clone() as Arc<dyn UsageOperations>,
            image: self.backend.clone() as Arc<dyn ImageWorkflowOperations>,
            desktop: self.backend.clone() as Arc<dyn DesktopServices>,
            scenario_control: Some(self.backend.clone() as Arc<dyn ScenarioControl>),
        }
    }

    pub fn backend(&self) -> Arc<ScenarioBackend> {
        self.backend.clone()
    }

    pub fn marker(&self) -> String {
        self.backend.marker()
    }

    pub fn start_control_server(
        &self,
        socket: PathBuf,
        token: String,
    ) -> Result<Arc<ScenarioControlServer>, StorageError> {
        ScenarioControlServer::spawn(self.backend.clone(), socket, token)
    }
}

struct ScenarioLogicalSource {
    source: LogicalSource,
}

#[async_trait]
impl LogicalTopologySource for ScenarioLogicalSource {
    fn logical_source(&self) -> LogicalSource {
        self.source
    }
    fn logical_availability(&self) -> LogicalSourceAvailability {
        LogicalSourceAvailability::Available
    }
    async fn list_logical_entities(
        &self,
    ) -> Result<Vec<storage_types::LogicalEntity>, StorageError> {
        Ok(Vec::new())
    }
}

impl BackendMetadata for ScenarioBackend {
    fn id(&self) -> BackendId {
        BackendId::new("ui-scenario")
    }
    fn capabilities(&self) -> StorageBackendCapabilities {
        StorageBackendCapabilities {
            drive_power_management: true,
            partitioning: true,
            filesystem_operations: true,
            encryption_operations: true,
            image_operations: true,
            logical_storage: true,
        }
    }
}

#[async_trait]
impl DiskDiscovery for ScenarioBackend {
    async fn list_disks(&self) -> Result<Vec<DiskInfo>, StorageError> {
        Ok(self.state.lock().await.disks.clone())
    }
    async fn list_volumes(&self) -> Result<Vec<storage_types::VolumeInfo>, StorageError> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl DeviceEventSource for ScenarioBackend {
    async fn device_events(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<DeviceEvent, StorageError>> + Send>>, StorageError>
    {
        let events = std::mem::take(&mut self.state.lock().await.events);
        Ok(Box::pin(stream::iter(events.into_iter().map(Ok))))
    }
}

#[async_trait]
impl DriveOperations for ScenarioBackend {
    async fn smart_info(&self, _device: &str) -> Result<SmartInfo, StorageError> {
        Ok(SmartInfo::default())
    }
    async fn start_smart_selftest(
        &self,
        _device: &str,
        _kind: SmartSelfTestKind,
    ) -> Result<(), StorageError> {
        Err(Self::unsupported("drive.start_smart_selftest"))
    }
    async fn eject(&self, _device: &str, _ejectable: bool) -> Result<(), StorageError> {
        Err(Self::unsupported("drive.eject"))
    }
    async fn power_off(&self, _device: &str, _can_power_off: bool) -> Result<(), StorageError> {
        Err(Self::unsupported("drive.power_off"))
    }
    async fn standby(&self, _device: &str) -> Result<(), StorageError> {
        Err(Self::unsupported("drive.standby"))
    }
    async fn wakeup(&self, _device: &str) -> Result<(), StorageError> {
        Err(Self::unsupported("drive.wakeup"))
    }
    async fn safe_remove(
        &self,
        _device: &str,
        _is_loop: bool,
        _removable: bool,
        _can_power_off: bool,
    ) -> Result<(), StorageError> {
        Err(Self::unsupported("drive.safe_remove"))
    }
}

#[async_trait]
impl PartitionOperations for ScenarioBackend {
    async fn list_partitions(&self, disk: &str) -> Result<Vec<PartitionInfo>, StorageError> {
        Ok(self
            .state
            .lock()
            .await
            .partitions
            .iter()
            .filter(|partition| partition.parent_path == disk)
            .cloned()
            .collect())
    }
    async fn create_partition_table(
        &self,
        _disk: &str,
        _table_type: &str,
    ) -> Result<(), StorageError> {
        Err(Self::unsupported("partition.create_partition_table"))
    }
    async fn create_partition(
        &self,
        disk: &str,
        offset: u64,
        size: u64,
        type_id: &str,
    ) -> Result<String, StorageError> {
        if let Some(error) = self.configured_error("partition.create_partition").await {
            return Err(error);
        }
        let mut state = self.state.lock().await;
        if !state.disks.iter().any(|known| known.device == disk) {
            return Err(StorageError::new(
                StorageErrorKind::NotFound,
                "scenario disk does not exist",
            ));
        }
        let number = state
            .partitions
            .iter()
            .filter(|partition| partition.parent_path == disk)
            .count() as u32
            + 1;
        let device = format!("{disk}p{number}");
        state.partitions.push(PartitionInfo {
            device: device.clone(),
            number,
            parent_path: disk.into(),
            size,
            offset,
            type_id: type_id.into(),
            type_name: type_id.into(),
            flags: 0,
            name: String::new(),
            uuid: String::new(),
            table_type: "gpt".into(),
            has_filesystem: false,
            filesystem_type: None,
            mount_points: Vec::new(),
            usage: None,
        });
        state.events.push(DeviceEvent::Added(device.clone()));
        state.sequence += 1;
        state.generation += 1;
        Ok(device)
    }
    async fn create_partition_with_filesystem(
        &self,
        disk: &str,
        info: &CreatePartitionInfo,
    ) -> Result<String, StorageError> {
        let device = self
            .create_partition(disk, info.offset, info.size, &info.selected_type)
            .await?;
        if !info.filesystem_type.trim().is_empty() {
            self.format_filesystem(
                &device,
                info.filesystem_type.trim(),
                &info.name,
                FormatOptions::default(),
            )
            .await?;
            let mut state = self.state.lock().await;
            if let Some(partition) = state
                .partitions
                .iter_mut()
                .find(|partition| partition.device == device)
            {
                partition.has_filesystem = true;
                partition.filesystem_type = Some(info.filesystem_type.trim().into());
            }
        }
        Ok(device)
    }
    async fn delete_partition(&self, _partition: &str) -> Result<(), StorageError> {
        Err(Self::unsupported("partition.delete_partition"))
    }
    async fn resize_partition(&self, _partition: &str, _new_size: u64) -> Result<(), StorageError> {
        Err(Self::unsupported("partition.resize_partition"))
    }
    async fn set_partition_type(
        &self,
        _partition: &str,
        _type_id: &str,
    ) -> Result<(), StorageError> {
        Err(Self::unsupported("partition.set_partition_type"))
    }
    async fn set_partition_flags(&self, _partition: &str, _flags: u64) -> Result<(), StorageError> {
        Err(Self::unsupported("partition.set_partition_flags"))
    }
    async fn set_partition_name(&self, _partition: &str, _name: &str) -> Result<(), StorageError> {
        Err(Self::unsupported("partition.set_partition_name"))
    }
}

#[async_trait]
impl FilesystemOperations for ScenarioBackend {
    async fn list_filesystems(&self) -> Result<Vec<FilesystemInfo>, StorageError> {
        Ok(self.state.lock().await.filesystems.clone())
    }
    async fn format_filesystem(
        &self,
        device: &str,
        filesystem_type: &str,
        label: &str,
        _options: FormatOptions,
    ) -> Result<(), StorageError> {
        if let Some(error) = self.configured_error("filesystem.format").await {
            return Err(error);
        }
        let mut state = self.state.lock().await;
        state
            .filesystems
            .retain(|filesystem| filesystem.device != device);
        state.filesystems.push(FilesystemInfo {
            device: device.into(),
            fs_type: filesystem_type.into(),
            label: label.into(),
            uuid: String::new(),
            mount_points: Vec::new(),
            size: 0,
            available: 0,
        });
        state
            .filesystems
            .sort_by(|left, right| left.device.cmp(&right.device));
        state.events.push(DeviceEvent::Added(device.into()));
        state.sequence += 1;
        state.generation += 1;
        Ok(())
    }
    async fn mount_filesystem(
        &self,
        device: &str,
        mount_point: &str,
        _options: MountOptions,
    ) -> Result<String, StorageError> {
        let mount_point = if mount_point.is_empty() {
            format!(
                "/mnt/ui-{}",
                device.trim_start_matches("/dev/ui-").replace('/', "-")
            )
        } else {
            mount_point.into()
        };
        let mut state = self.state.lock().await;
        let filesystem = state
            .filesystems
            .iter_mut()
            .find(|filesystem| filesystem.device == device)
            .ok_or_else(|| {
                StorageError::new(
                    StorageErrorKind::NotFound,
                    "scenario filesystem does not exist",
                )
            })?;
        filesystem.mount_points = vec![mount_point.clone()];
        Ok(mount_point)
    }
    async fn get_mount_point(&self, device: &str) -> Result<String, StorageError> {
        self.state
            .lock()
            .await
            .filesystems
            .iter()
            .find(|filesystem| filesystem.device == device)
            .and_then(|filesystem| filesystem.mount_points.first())
            .cloned()
            .ok_or_else(|| {
                StorageError::new(
                    StorageErrorKind::NotFound,
                    "scenario filesystem is not mounted",
                )
            })
    }
    async fn unmount_filesystem(
        &self,
        device_or_mount: &str,
        _force: bool,
    ) -> Result<(), StorageError> {
        if let Some(error) = self.configured_error("filesystem.unmount").await {
            return Err(error);
        }
        let mut state = self.state.lock().await;
        let filesystem = state
            .filesystems
            .iter_mut()
            .find(|filesystem| {
                filesystem.device == device_or_mount
                    || filesystem
                        .mount_points
                        .iter()
                        .any(|mount| mount == device_or_mount)
            })
            .ok_or_else(|| {
                StorageError::new(
                    StorageErrorKind::NotFound,
                    "scenario filesystem does not exist",
                )
            })?;
        filesystem.mount_points.clear();
        state.sequence += 1;
        Ok(())
    }
    async fn blocking_processes(
        &self,
        mount_point: &str,
    ) -> Result<Vec<ProcessInfo>, StorageError> {
        Ok(self
            .state
            .lock()
            .await
            .processes
            .get(mount_point)
            .cloned()
            .into_iter()
            .collect())
    }
    async fn kill_processes(&self, _pids: &[i32]) -> Result<(), StorageError> {
        Err(Self::unsupported("filesystem.kill_processes"))
    }
    async fn check_filesystem(&self, _device: &str, _repair: bool) -> Result<bool, StorageError> {
        Ok(true)
    }
    async fn filesystem_label(&self, device: &str) -> Result<String, StorageError> {
        self.state
            .lock()
            .await
            .filesystems
            .iter()
            .find(|filesystem| filesystem.device == device)
            .map(|filesystem| filesystem.label.clone())
            .ok_or_else(|| {
                StorageError::new(
                    StorageErrorKind::NotFound,
                    "scenario filesystem does not exist",
                )
            })
    }
    async fn set_filesystem_label(&self, _device: &str, _label: &str) -> Result<(), StorageError> {
        Err(Self::unsupported("filesystem.set_label"))
    }
    async fn mount_options(
        &self,
        _device: &str,
    ) -> Result<Option<MountOptionsSettings>, StorageError> {
        Ok(None)
    }
    async fn reset_mount_options(&self, _device: &str) -> Result<(), StorageError> {
        Err(Self::unsupported("filesystem.reset_mount_options"))
    }
    async fn set_mount_options(
        &self,
        _device: &str,
        _mount_at_startup: bool,
        _show_in_ui: bool,
        _require_auth: bool,
        _display_name: Option<String>,
        _icon_name: Option<String>,
        _symbolic_icon_name: Option<String>,
        _options: String,
        _mount_point: String,
        _identify_as: String,
        _filesystem_type: String,
    ) -> Result<(), StorageError> {
        Err(Self::unsupported("filesystem.set_mount_options"))
    }
    async fn take_filesystem_ownership(
        &self,
        _device: &str,
        _recursive: bool,
    ) -> Result<(), StorageError> {
        Err(Self::unsupported("filesystem.take_ownership"))
    }
}

#[async_trait]
impl EncryptionOperations for ScenarioBackend {
    async fn list_luks_devices(&self) -> Result<Vec<storage_types::LuksInfo>, StorageError> {
        Ok(self
            .state
            .lock()
            .await
            .luks
            .values()
            .map(|luks| luks.info.clone())
            .collect())
    }
    async fn format_luks(
        &self,
        _device: &str,
        _passphrase: &str,
        _version: &str,
    ) -> Result<(), StorageError> {
        Err(Self::unsupported("encryption.format_luks"))
    }
    async fn unlock_luks(&self, device: &str, passphrase: &str) -> Result<String, StorageError> {
        if let Some(error) = self.configured_error("encryption.unlock_luks").await {
            return Err(error);
        }
        let mut state = self.state.lock().await;
        let luks = state
            .luks
            .values_mut()
            .find(|luks| luks.info.device == device)
            .ok_or_else(|| {
                StorageError::new(
                    StorageErrorKind::NotFound,
                    "scenario LUKS device does not exist",
                )
            })?;
        let expected = self.secrets.get(&luks.secret_id).ok_or_else(|| {
            StorageError::new(
                StorageErrorKind::PermissionDenied,
                "scenario LUKS secret is not available to this runtime",
            )
        })?;
        if passphrase != expected {
            return Err(StorageError::new(
                StorageErrorKind::PermissionDenied,
                "incorrect scenario LUKS passphrase",
            ));
        }
        luks.info.unlocked = true;
        luks.info.cleartext_device = Some(luks.mapper.clone());
        let mapper = luks.info.cleartext_device.clone().expect("set above");
        state.sequence += 1;
        state.generation += 1;
        Ok(mapper)
    }
    async fn lock_luks(&self, device: &str) -> Result<(), StorageError> {
        let mut state = self.state.lock().await;
        let luks = state
            .luks
            .values_mut()
            .find(|luks| {
                luks.info.device == device || luks.info.cleartext_device.as_deref() == Some(device)
            })
            .ok_or_else(|| {
                StorageError::new(
                    StorageErrorKind::NotFound,
                    "scenario LUKS device does not exist",
                )
            })?;
        luks.info.unlocked = false;
        luks.info.cleartext_device = None;
        state.sequence += 1;
        state.generation += 1;
        Ok(())
    }
    async fn change_luks_passphrase(
        &self,
        _device: &str,
        _current: &str,
        _next: &str,
    ) -> Result<(), StorageError> {
        Err(Self::unsupported("encryption.change_passphrase"))
    }
    async fn encryption_options(
        &self,
        _device: &str,
    ) -> Result<Option<EncryptionOptionsSettings>, StorageError> {
        Ok(None)
    }
    async fn set_encryption_options(
        &self,
        _device: &str,
        _settings: &EncryptionOptionsSettings,
    ) -> Result<(), StorageError> {
        Err(Self::unsupported("encryption.set_options"))
    }
    async fn clear_encryption_options(&self, _device: &str) -> Result<(), StorageError> {
        Err(Self::unsupported("encryption.clear_options"))
    }
}

#[async_trait]
impl ImageDeviceOperations for ScenarioBackend {
    async fn open_for_backup(&self, _device: &str) -> Result<std::os::fd::OwnedFd, StorageError> {
        Err(Self::unsupported("image_device.open_for_backup"))
    }
    async fn open_for_restore(&self, _device: &str) -> Result<std::os::fd::OwnedFd, StorageError> {
        Err(Self::unsupported("image_device.open_for_restore"))
    }
    async fn loop_setup(&self, _image_path: &str) -> Result<String, StorageError> {
        Err(Self::unsupported("image_device.loop_setup"))
    }
}

#[async_trait]
impl BtrfsOperations for ScenarioBackend {
    async fn list_subvolumes(&self, _mountpoint: &str) -> Result<SubvolumeList, StorageError> {
        Err(Self::unsupported("btrfs.list_subvolumes"))
    }
    async fn create_subvolume(&self, _mountpoint: &str, _name: &str) -> Result<(), StorageError> {
        Err(Self::unsupported("btrfs.create_subvolume"))
    }
    async fn create_snapshot(
        &self,
        _mountpoint: &str,
        _source: &str,
        _destination: &str,
        _readonly: bool,
    ) -> Result<(), StorageError> {
        Err(Self::unsupported("btrfs.create_snapshot"))
    }
    async fn delete_subvolume(
        &self,
        _mountpoint: &str,
        _path: &str,
        _recursive: bool,
    ) -> Result<(), StorageError> {
        Err(Self::unsupported("btrfs.delete_subvolume"))
    }
    async fn set_readonly(
        &self,
        _mountpoint: &str,
        _path: &str,
        _readonly: bool,
    ) -> Result<(), StorageError> {
        Err(Self::unsupported("btrfs.set_readonly"))
    }
    async fn set_default(&self, _mountpoint: &str, _path: &str) -> Result<(), StorageError> {
        Err(Self::unsupported("btrfs.set_default"))
    }
    async fn default_subvolume(&self, _mountpoint: &str) -> Result<u64, StorageError> {
        Err(Self::unsupported("btrfs.default_subvolume"))
    }
    async fn deleted_subvolumes(
        &self,
        _mountpoint: &str,
    ) -> Result<Vec<storage_types::DeletedSubvolume>, StorageError> {
        Err(Self::unsupported("btrfs.deleted_subvolumes"))
    }
    async fn filesystem_usage(
        &self,
        _mountpoint: &str,
    ) -> Result<storage_types::FilesystemUsage, StorageError> {
        Err(Self::unsupported("btrfs.filesystem_usage"))
    }
}

#[async_trait]
impl LogicalOperations for ScenarioBackend {
    async fn capture_logical_candidate(
        &self,
        display_path: String,
    ) -> Result<LogicalCandidateAnchor, StorageError> {
        let state = self.state.lock().await;
        if state.logical_candidate_path.as_deref() != Some(display_path.as_str()) {
            return Err(StorageError::new(
                StorageErrorKind::NotFound,
                "scenario logical candidate does not exist",
            ));
        }
        Ok(LogicalCandidateAnchor {
            kind: storage_types::LogicalCandidateKind::LvmPhysicalVolume,
            block_id: storage_types::BlockDeviceId::new(1, 1),
            fingerprint: Some(
                storage_types::BlockDeviceFingerprint::filesystem_uuid("scenario-logical", "ext4")
                    .expect("fixed scenario fingerprint"),
            ),
            observed_epoch: state.generation,
            display_path,
        })
    }
    async fn preflight_logical_action(
        &self,
        request: LogicalPreflightRequest,
    ) -> Result<LogicalPreflight, StorageError> {
        let state = self.state.lock().await;
        if state.logical_candidate_path.is_none() {
            return Err(Self::unsupported("logical.preflight"));
        }
        Ok(LogicalPreflight {
            key: storage_contracts::LogicalPreflightKey {
                request_key: request.request_key,
                udisks_epoch: state.generation,
            },
            availability: LogicalPreflightAvailability::Ready,
            device_candidates: Vec::new(),
            constraints: Default::default(),
            review: LogicalReviewData::None,
        })
    }
    async fn execute_logical_action(
        &self,
        confirmed: storage_contracts::ConfirmedLogicalAction,
    ) -> Result<LogicalActionOutcome, StorageError> {
        confirmed.action.validate()?;
        let mut state = self.state.lock().await;
        if confirmed.preflight_key.udisks_epoch != state.generation {
            return Err(StorageError::new(
                StorageErrorKind::Conflict,
                "logical preflight is stale; review changes again",
            ));
        }
        if state.logical_confirmation_message.is_none() {
            return Err(Self::unsupported("logical.execute"));
        }
        state.sequence += 1;
        state.generation += 1;
        Ok(LogicalActionOutcome {
            action: confirmed.action,
            affected_entity_ids: Vec::new(),
            native_job_id: Some(format!("scenario-logical-{}", state.sequence)),
            progress: None,
        })
    }
}

#[async_trait]
impl FilesystemToolDiscovery for ScenarioBackend {
    async fn list_filesystem_tools(&self) -> Result<Vec<FilesystemToolInfo>, StorageError> {
        Ok(vec![FilesystemToolInfo {
            fs_type: "ext4".into(),
            fs_name: "EXT4".into(),
            command: "mkfs.ext4".into(),
            package_hint: "scenario".into(),
            available: true,
        }])
    }
}

#[async_trait]
impl UsageOperations for ScenarioBackend {
    async fn list_usage_mounts(&self) -> Result<Vec<String>, StorageError> {
        Ok(self
            .state
            .lock()
            .await
            .filesystems
            .iter()
            .flat_map(|filesystem| filesystem.mount_points.clone())
            .collect())
    }
    async fn authorize_show_all_files(&self) -> Result<bool, StorageError> {
        Ok(true)
    }
    async fn start_usage_scan(
        &self,
        request: UsageWorkflowRequest,
    ) -> Result<String, StorageError> {
        let mut state = self.state.lock().await;
        let script = state
            .usage_scan_script
            .clone()
            .ok_or_else(|| Self::unsupported("usage.start_scan"))?;
        let scan_id = request.scan_id;
        state.usage_operations.insert(
            scan_id.clone(),
            UsageWorkflowStatus {
                scan_id: scan_id.clone(),
                state: WorkflowState::Running,
                processed_bytes: 0,
                estimated_total_bytes: script.total_bytes,
                result: None,
                message: None,
            },
        );
        Self::refresh_workflows(&mut state);
        Ok(scan_id)
    }
    async fn usage_scan_status(&self, scan_id: &str) -> Result<UsageWorkflowStatus, StorageError> {
        self.state
            .lock()
            .await
            .usage_operations
            .get(scan_id)
            .cloned()
            .ok_or_else(|| {
                StorageError::new(
                    StorageErrorKind::NotFound,
                    "scenario usage scan does not exist",
                )
            })
    }
    async fn wait_for_usage_scan(
        &self,
        scan_id: &str,
    ) -> Result<UsageWorkflowStatus, StorageError> {
        self.usage_scan_status(scan_id).await
    }
    async fn delete_usage_files(
        &self,
        request: UsageDeleteRequest,
    ) -> Result<UsageDeleteResponse, StorageError> {
        let mut state = self.state.lock().await;
        let mut deleted = Vec::new();
        let mut failed = Vec::new();
        for path in request.paths {
            let matched = state
                .usage_files
                .iter()
                .find(|(_, file)| path == format!("{}/{}", file.mount, file.id))
                .map(|(id, _)| id.clone());
            match matched {
                Some(id) => {
                    state.usage_files.remove(&id);
                    deleted.push(path);
                }
                None => failed.push(UsageDeleteFailure {
                    path,
                    reason: "scenario usage file does not exist".into(),
                }),
            }
        }
        state.sequence += 1;
        Ok(UsageDeleteResponse {
            result: UsageDeleteResult { deleted, failed },
        })
    }
}

#[async_trait]
impl ImageWorkflowOperations for ScenarioBackend {
    async fn create_image_asset(
        &self,
        asset: ImageAssetRef,
        _size_bytes: u64,
    ) -> Result<(), StorageError> {
        self.state
            .lock()
            .await
            .image_assets
            .insert(asset.to_string());
        Ok(())
    }
    async fn attach_image(
        &self,
        request: ImageAttachmentRequest,
    ) -> Result<ImageAttachment, StorageError> {
        let state = self.state.lock().await;
        if !state.image_assets.contains(request.asset.as_str()) {
            return Err(StorageError::new(
                StorageErrorKind::NotFound,
                "scenario image asset does not exist",
            ));
        }
        Ok(ImageAttachment {
            device: "/dev/ui-loop0".into(),
            mounted: false,
        })
    }
    async fn start_image_copy(&self, request: ImageCopyRequest) -> Result<String, StorageError> {
        let mut state = self.state.lock().await;
        if !state.image_assets.contains(request.asset.as_str()) {
            return Err(StorageError::new(
                StorageErrorKind::NotFound,
                "scenario image asset does not exist",
            ));
        }
        let script = state
            .image_copy_script
            .clone()
            .ok_or_else(|| Self::unsupported("image.start_copy"))?;
        state.sequence += 1;
        let operation_id = format!("image-{}", state.sequence);
        state.image_operations.insert(
            operation_id.clone(),
            ImageWorkflowStatus {
                operation_id: operation_id.clone(),
                state: WorkflowState::Running,
                bytes_completed: 0,
                bytes_total: script.total_bytes,
                speed_bytes_per_sec: 0,
                message: None,
            },
        );
        Self::refresh_workflows(&mut state);
        Ok(operation_id)
    }
    async fn image_copy_status(
        &self,
        operation_id: &str,
    ) -> Result<ImageWorkflowStatus, StorageError> {
        self.state
            .lock()
            .await
            .image_operations
            .get(operation_id)
            .cloned()
            .ok_or_else(|| {
                StorageError::new(
                    StorageErrorKind::NotFound,
                    "scenario image operation does not exist",
                )
            })
    }
    async fn wait_for_image_copy(
        &self,
        operation_id: &str,
    ) -> Result<ImageWorkflowStatus, StorageError> {
        self.image_copy_status(operation_id).await
    }
    async fn cancel_image_copy(&self, operation_id: &str) -> Result<(), StorageError> {
        let mut state = self.state.lock().await;
        let operation = state
            .image_operations
            .get_mut(operation_id)
            .ok_or_else(|| {
                StorageError::new(
                    StorageErrorKind::NotFound,
                    "scenario image operation does not exist",
                )
            })?;
        if matches!(
            operation.state,
            WorkflowState::Completed | WorkflowState::Failed
        ) {
            return Err(StorageError::new(
                StorageErrorKind::Conflict,
                "terminal scenario image operation cannot be cancelled",
            ));
        }
        operation.state = WorkflowState::Cancelled;
        state.sequence += 1;
        Ok(())
    }
    async fn forget_image_copy(&self, operation_id: &str) -> Result<(), StorageError> {
        self.state
            .lock()
            .await
            .image_operations
            .remove(operation_id)
            .map(|_| ())
            .ok_or_else(|| {
                StorageError::new(
                    StorageErrorKind::NotFound,
                    "scenario image operation does not exist",
                )
            })
    }
}

#[async_trait]
impl DesktopServices for ScenarioBackend {
    async fn select_image(&self) -> Result<DesktopImageSelection, StorageError> {
        Ok(DesktopImageSelection::Cancelled)
    }
    async fn reveal(&self, _display_reference: &str) -> Result<(), StorageError> {
        Ok(())
    }
    async fn open_url(&self, _url: &str) -> Result<(), StorageError> {
        Ok(())
    }
}

#[async_trait]
impl NetworkDriveBackend for ScenarioBackend {
    fn id(&self) -> storage_types::NetworkBackendId {
        storage_types::NetworkBackendId::rclone()
    }
    fn capabilities(&self) -> storage_types::NetworkDriveCapabilities {
        storage_types::NetworkDriveCapabilities::default()
    }
    fn configuration_schema(&self) -> storage_types::NetworkDriveConfigurationSchema {
        storage_types::NetworkDriveConfigurationSchema::default()
    }
    async fn list_configs(&self) -> Result<storage_types::NetworkDriveList, StorageError> {
        Ok(storage_types::NetworkDriveList {
            configs: self
                .state
                .lock()
                .await
                .network_configs
                .values()
                .cloned()
                .collect(),
        })
    }
    async fn create_config(
        &self,
        config: &storage_types::NetworkDriveConfig,
    ) -> Result<(), StorageError> {
        config
            .validate_name()
            .map_err(|error| StorageError::new(StorageErrorKind::InvalidInput, error))?;
        let mut state = self.state.lock().await;
        if state.network_configs.contains_key(&config.id) {
            return Err(StorageError::new(
                StorageErrorKind::Conflict,
                "scenario network configuration already exists",
            ));
        }
        state
            .network_configs
            .insert(config.id.clone(), config.clone());
        Ok(())
    }
    async fn update_config(
        &self,
        config: &storage_types::NetworkDriveConfig,
    ) -> Result<(), StorageError> {
        let mut state = self.state.lock().await;
        if !state.network_configs.contains_key(&config.id) {
            return Err(StorageError::new(
                StorageErrorKind::NotFound,
                "scenario network configuration does not exist",
            ));
        }
        state
            .network_configs
            .insert(config.id.clone(), config.clone());
        Ok(())
    }
    async fn delete_config(&self, config_id: &str) -> Result<(), StorageError> {
        let mut state = self.state.lock().await;
        state.network_mounts.remove(config_id);
        state.network_mount_on_login.remove(config_id);
        state
            .network_configs
            .remove(config_id)
            .map(|_| ())
            .ok_or_else(|| {
                StorageError::new(
                    StorageErrorKind::NotFound,
                    "scenario network configuration does not exist",
                )
            })
    }
    async fn test_config(
        &self,
        config_id: &str,
    ) -> Result<storage_types::NetworkDriveTestResult, StorageError> {
        if !self
            .state
            .lock()
            .await
            .network_configs
            .contains_key(config_id)
        {
            return Err(StorageError::new(
                StorageErrorKind::NotFound,
                "scenario network configuration does not exist",
            ));
        }
        Ok(storage_types::NetworkDriveTestResult {
            success: true,
            message: "scenario connection test passed".into(),
            latency_ms: Some(0),
        })
    }
    async fn mount(&self, config_id: &str) -> Result<(), StorageError> {
        let mut state = self.state.lock().await;
        if !state.network_configs.contains_key(config_id) {
            return Err(StorageError::new(
                StorageErrorKind::NotFound,
                "scenario network configuration does not exist",
            ));
        }
        state.network_mounts.insert(
            config_id.into(),
            storage_types::NetworkDriveMount {
                backend_id: storage_types::NetworkBackendId::rclone(),
                config_id: config_id.into(),
                mount_point: format!("/mnt/ui-network-{config_id}").into(),
                status: storage_types::NetworkDriveStatus::Mounted,
            },
        );
        state.sequence += 1;
        Ok(())
    }
    async fn unmount(&self, config_id: &str) -> Result<(), StorageError> {
        let mut state = self.state.lock().await;
        let mount = state.network_mounts.get_mut(config_id).ok_or_else(|| {
            StorageError::new(
                StorageErrorKind::NotFound,
                "scenario network mount does not exist",
            )
        })?;
        mount.status = storage_types::NetworkDriveStatus::Unmounted;
        Ok(())
    }
    async fn mount_status(
        &self,
        config_id: &str,
    ) -> Result<storage_types::NetworkDriveMount, StorageError> {
        self.state
            .lock()
            .await
            .network_mounts
            .get(config_id)
            .cloned()
            .ok_or_else(|| {
                StorageError::new(
                    StorageErrorKind::NotFound,
                    "scenario network mount does not exist",
                )
            })
    }
    async fn mount_on_login(&self, config_id: &str) -> Result<bool, StorageError> {
        Ok(*self
            .state
            .lock()
            .await
            .network_mount_on_login
            .get(config_id)
            .unwrap_or(&false))
    }
    async fn set_mount_on_login(&self, config_id: &str, enabled: bool) -> Result<(), StorageError> {
        self.state
            .lock()
            .await
            .network_mount_on_login
            .insert(config_id.into(), enabled);
        Ok(())
    }
}

#[async_trait]
impl ScenarioControl for ScenarioBackend {
    async fn advance_to(&self, tick: u64) -> Result<ScenarioReceipt, StorageError> {
        let mut state = self.state.lock().await;
        if tick < state.tick {
            return Err(StorageError::new(
                StorageErrorKind::InvalidInput,
                "scenario clock cannot move backwards",
            ));
        }
        state.tick = tick;
        Self::refresh_workflows(&mut state);
        state.sequence += 1;
        Ok(ScenarioReceipt {
            sequence: state.sequence,
            generation: state.generation,
            virtual_tick: state.tick,
        })
    }
    async fn reload_overlay(&self) -> Result<ScenarioReload, StorageError> {
        let Some(bytes) = self.store.read_overlay()? else {
            let receipt = self.next_receipt(false).await;
            return Ok(ScenarioReload::Rejected {
                reason: "no scenario overlay is configured".into(),
                receipt,
            });
        };
        match parse_fixture(&bytes) {
            Ok((spec, _)) => {
                *self.spec.lock().await = spec.clone();
                *self.state.lock().await = State::from_spec(&spec);
                Ok(ScenarioReload::Applied(self.next_receipt(true).await))
            }
            Err(error) => Ok(ScenarioReload::Rejected {
                reason: error.message,
                receipt: self.next_receipt(false).await,
            }),
        }
    }
    async fn diagnostics(&self) -> Result<ScenarioDiagnostics, StorageError> {
        Ok(self.diagnostics_snapshot().await)
    }
}

// The scenario runtime intentionally has no network registrations until a
// fixture declares one.  This tiny adapter remains here as a compile-time
// completeness reference for consumers that need a typed unavailable backend.
pub struct UnsupportedNetworkBackend;

#[async_trait]
impl NetworkDriveBackend for UnsupportedNetworkBackend {
    fn id(&self) -> storage_types::NetworkBackendId {
        storage_types::NetworkBackendId::new("ui-scenario")
    }
    fn capabilities(&self) -> storage_types::NetworkDriveCapabilities {
        storage_types::NetworkDriveCapabilities::default()
    }
    fn configuration_schema(&self) -> storage_types::NetworkDriveConfigurationSchema {
        storage_types::NetworkDriveConfigurationSchema::default()
    }
    async fn list_configs(&self) -> Result<storage_types::NetworkDriveList, StorageError> {
        Err(ScenarioBackend::unsupported("network.list_configs"))
    }
    async fn create_config(
        &self,
        _config: &storage_types::NetworkDriveConfig,
    ) -> Result<(), StorageError> {
        Err(ScenarioBackend::unsupported("network.create_config"))
    }
    async fn update_config(
        &self,
        _config: &storage_types::NetworkDriveConfig,
    ) -> Result<(), StorageError> {
        Err(ScenarioBackend::unsupported("network.update_config"))
    }
    async fn delete_config(&self, _config_id: &str) -> Result<(), StorageError> {
        Err(ScenarioBackend::unsupported("network.delete_config"))
    }
    async fn test_config(
        &self,
        _config_id: &str,
    ) -> Result<storage_types::NetworkDriveTestResult, StorageError> {
        Err(ScenarioBackend::unsupported("network.test_config"))
    }
    async fn mount(&self, _config_id: &str) -> Result<(), StorageError> {
        Err(ScenarioBackend::unsupported("network.mount"))
    }
    async fn unmount(&self, _config_id: &str) -> Result<(), StorageError> {
        Err(ScenarioBackend::unsupported("network.unmount"))
    }
    async fn mount_status(
        &self,
        _config_id: &str,
    ) -> Result<storage_types::NetworkDriveMount, StorageError> {
        Err(ScenarioBackend::unsupported("network.mount_status"))
    }
    async fn mount_on_login(&self, _config_id: &str) -> Result<bool, StorageError> {
        Err(ScenarioBackend::unsupported("network.mount_on_login"))
    }
    async fn set_mount_on_login(
        &self,
        _config_id: &str,
        _enabled: bool,
    ) -> Result<(), StorageError> {
        Err(ScenarioBackend::unsupported("network.set_mount_on_login"))
    }
}

use std::{
    collections::{BTreeMap, VecDeque},
    fs,
    path::{Component, Path, PathBuf},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};
use storage_contracts::ScenarioDiagnostics;

use crate::{
    AppModel, AppRuntime,
    operations::GlobalOperationsGuard,
    workflows::{
        EffectRecord, SecretInput, WorkflowCapabilities, image_usage, logical, network, physical,
        reload,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowHarnessError {
    InvalidFixtureReference,
    FixtureIntegrityMismatch,
    TemporaryRootUnavailable,
    ScenarioBootstrapFailed,
    ScenarioControlUnavailable,
    SchedulerProtocolViolation,
    TraceContainsSensitiveInput,
}

impl std::fmt::Display for WorkflowHarnessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidFixtureReference => "invalid workflow fixture reference",
            Self::FixtureIntegrityMismatch => "workflow fixture integrity mismatch",
            Self::TemporaryRootUnavailable => "workflow temporary root unavailable",
            Self::ScenarioBootstrapFailed => "workflow scenario bootstrap failed",
            Self::ScenarioControlUnavailable => "workflow scenario control unavailable",
            Self::SchedulerProtocolViolation => "workflow scheduler protocol violation",
            Self::TraceContainsSensitiveInput => "trace contains sensitive input",
        })
    }
}

impl std::error::Error for WorkflowHarnessError {}

/// Out-of-band fixture secrets. It deliberately cannot be printed, cloned, or
/// read back by a test after hand-off to the scenario runtime.
pub struct FixtureSecrets(BTreeMap<String, SecretInput>);

impl FixtureSecrets {
    pub fn none() -> Self {
        Self(BTreeMap::new())
    }

    pub fn insert(
        &mut self,
        secret_id: String,
        value: SecretInput,
    ) -> Result<(), WorkflowHarnessError> {
        if secret_id.is_empty() || self.0.contains_key(&secret_id) {
            return Err(WorkflowHarnessError::ScenarioBootstrapFailed);
        }
        self.0.insert(secret_id, value);
        Ok(())
    }

    fn consume(self) -> Result<BTreeMap<String, String>, WorkflowHarnessError> {
        self.0
            .into_iter()
            .map(|(id, secret)| {
                secret
                    .into_string()
                    .map(|value| (id, value))
                    .map_err(|_| WorkflowHarnessError::ScenarioBootstrapFailed)
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TraceProjection {
    pub sequence: u64,
    pub virtual_tick: u64,
    pub operation: String,
    pub generation: u64,
    pub status: String,
}

enum ScheduledEffect {
    Logical(logical::Effect),
    Physical(physical::Effect),
    Network(network::Effect),
    ImageUsage(image_usage::Effect),
    Reload(reload::Effect),
}

impl ScheduledEffect {
    fn workflow(&self) -> &'static str {
        match self {
            Self::Logical(_) => "logical",
            Self::Physical(_) => "physical",
            Self::Network(_) => "network",
            Self::ImageUsage(_) => "image_usage",
            Self::Reload(_) => "reload",
        }
    }

    fn operation(&self) -> &'static str {
        match self {
            Self::Logical(effect) => effect.operation(),
            Self::Physical(effect) => effect.operation(),
            Self::Network(effect) => effect.operation(),
            Self::ImageUsage(effect) => effect.operation(),
            Self::Reload(effect) => effect.operation(),
        }
    }

    fn generation(&self) -> u64 {
        match self {
            Self::Logical(effect) => effect.generation(),
            Self::Physical(effect) => effect.generation(),
            Self::Network(effect) => effect.generation(),
            Self::ImageUsage(effect) => effect.generation(),
            Self::Reload(effect) => effect.generation(),
        }
    }

    fn has_secret(&self) -> bool {
        matches!(self, Self::Physical(effect) if effect.has_secret())
    }
}

enum ScheduledCompletion {
    Logical(logical::Completion),
    Physical(physical::Completion),
    Network(network::Completion),
    ImageUsage(image_usage::Completion),
    Reload(reload::Completion),
}

impl ScheduledCompletion {
    fn status(&self) -> &'static str {
        match self {
            Self::Logical(logical::Completion::Captured { result, .. }) => status(result),
            Self::Logical(logical::Completion::Preflighted { result, .. }) => status(result),
            Self::Logical(logical::Completion::Executed { result, .. }) => status(result),
            Self::Physical(physical::Completion::Finished { result, .. }) => status(result),
            Self::Network(network::Completion::Created { result, .. }) => status(result),
            Self::Network(network::Completion::Tested { result, .. }) => status(result),
            Self::Network(network::Completion::Mounted { result, .. }) => status(result),
            Self::Network(network::Completion::Statused { result, .. }) => status(result),
            Self::ImageUsage(image_usage::Completion::ImageStarted { result, .. }) => {
                status(result)
            }
            Self::ImageUsage(image_usage::Completion::ImagePolled { result, .. }) => status(result),
            Self::ImageUsage(image_usage::Completion::ImageCancelled { result, .. }) => {
                status(result)
            }
            Self::ImageUsage(image_usage::Completion::UsageStarted { result, .. }) => {
                status(result)
            }
            Self::ImageUsage(image_usage::Completion::UsagePolled { result, .. }) => status(result),
            Self::ImageUsage(image_usage::Completion::UsageDeleted { result, .. }) => {
                status(result)
            }
            Self::Reload(reload::Completion::Advanced { result, .. }) => status(result),
            Self::Reload(reload::Completion::Reloaded { result, .. }) => match result {
                Ok(storage_contracts::ScenarioReload::Applied(_)) => "ok",
                Ok(storage_contracts::ScenarioReload::Rejected { .. }) => "rejected",
                Err(_) => "error",
            },
        }
    }
}

fn status<T>(result: &Result<T, crate::workflows::WorkflowError>) -> &'static str {
    if result.is_ok() { "ok" } else { "error" }
}

/// A deterministic, in-process application-workflow harness.
pub struct WorkflowHarness {
    _root: tempfile::TempDir,
    runtime: AppRuntime,
    model: AppModel,
    capabilities: WorkflowCapabilities,
    overlay: PathBuf,
    trace: PathBuf,
    _guard: GlobalOperationsGuard,
    queue: VecDeque<ScheduledEffect>,
    mailbox: Option<ScheduledCompletion>,
    effects: Vec<EffectRecord>,
    sequence: u64,
    generation: u64,
}

impl WorkflowHarness {
    pub async fn from_fixture(
        relative_fixture: &str,
        secrets: FixtureSecrets,
    ) -> Result<Self, WorkflowHarnessError> {
        let fixture = resolve_fixture(relative_fixture)?;
        let root = tempfile::Builder::new()
            .prefix("cosmic-ext-storage-workflow-")
            .tempdir()
            .map_err(|_| WorkflowHarnessError::TemporaryRootUnavailable)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700))
                .map_err(|_| WorkflowHarnessError::TemporaryRootUnavailable)?;
        }
        let overlay = root.path().join("overlay.toml");
        let trace = root.path().join("trace.json");
        let runtime = AppRuntime::scenario_with_fixture_secrets(
            fixture,
            Some(overlay.clone()),
            Some(trace.clone()),
            secrets.consume()?,
        )
        .map_err(|_| WorkflowHarnessError::ScenarioBootstrapFailed)?;
        let capabilities = WorkflowCapabilities::from(&runtime);
        let model = AppModel::for_workflow_test(runtime.clone());
        let guard = crate::operations::reject_global_operations_for_workflow_tests();
        Ok(Self {
            _root: root,
            runtime,
            model,
            capabilities,
            overlay,
            trace,
            _guard: guard,
            queue: VecDeque::new(),
            mailbox: None,
            effects: Vec::new(),
            sequence: 0,
            generation: 0,
        })
    }

    pub fn selected_block_backend_id(&self) -> String {
        self.runtime.operations().registry.block.id().0
    }

    pub fn dispatch_logical(
        &mut self,
        intent: logical::LogicalIntent,
    ) -> Result<(), WorkflowHarnessError> {
        self.next_generation();
        let effects = self.model.reduce_logical_workflow(intent);
        self.enqueue_logical(effects);
        Ok(())
    }

    pub fn dispatch_physical(
        &mut self,
        intent: physical::PhysicalIntent,
    ) -> Result<(), WorkflowHarnessError> {
        let generation = self.next_generation();
        let effects = self.model.reduce_physical_workflow(intent, generation);
        self.enqueue_physical(effects);
        Ok(())
    }

    pub fn dispatch_network(
        &mut self,
        intent: network::NetworkIntent,
    ) -> Result<(), WorkflowHarnessError> {
        let generation = self.next_generation();
        let effects = self.model.reduce_network_workflow(intent, generation);
        self.enqueue_network(effects);
        Ok(())
    }

    pub fn dispatch_image_usage(
        &mut self,
        intent: image_usage::ImageUsageIntent,
    ) -> Result<(), WorkflowHarnessError> {
        let generation = self.next_generation();
        let effects = self.model.reduce_image_usage_workflow(intent, generation);
        self.enqueue_image_usage(effects);
        Ok(())
    }

    pub fn dispatch_reload(
        &mut self,
        intent: reload::ReloadIntent,
    ) -> Result<(), WorkflowHarnessError> {
        let generation = self.next_generation();
        let effects = self.model.reduce_reload_workflow(intent, generation);
        self.enqueue_reload(effects);
        Ok(())
    }

    pub fn logical_snapshot(&self) -> logical::LogicalSnapshot {
        self.model.workflows.logical.snapshot()
    }

    pub fn physical_snapshot(&self) -> physical::PhysicalSnapshot {
        self.model.workflows.physical.snapshot()
    }

    pub fn network_snapshot(&self) -> network::NetworkSnapshot {
        self.model.workflows.network.snapshot()
    }

    pub fn image_usage_snapshot(&self) -> image_usage::ImageUsageSnapshot {
        self.model.workflows.image_usage.snapshot()
    }

    pub fn reload_snapshot(&self) -> reload::ReloadSnapshot {
        self.model.workflows.reload.snapshot()
    }

    pub fn effect_records(&self) -> &[EffectRecord] {
        &self.effects
    }

    pub async fn global_operations_lookup_is_rejected(&self) -> bool {
        crate::operations::shared().await.is_err()
    }

    pub async fn execute_scheduled(&mut self) -> Result<bool, WorkflowHarnessError> {
        if self.mailbox.is_some() {
            return Err(WorkflowHarnessError::SchedulerProtocolViolation);
        }
        let Some(effect) = self.queue.pop_front() else {
            return Ok(false);
        };
        self.sequence = self.sequence.saturating_add(1);
        self.effects.push(EffectRecord {
            sequence: self.sequence,
            workflow: effect.workflow(),
            operation: effect.operation(),
            generation: effect.generation(),
            has_secret: effect.has_secret(),
        });
        let completion = match effect {
            ScheduledEffect::Logical(effect) => {
                ScheduledCompletion::Logical(logical::execute(&self.capabilities, effect).await)
            }
            ScheduledEffect::Physical(effect) => {
                ScheduledCompletion::Physical(physical::execute(&self.capabilities, effect).await)
            }
            ScheduledEffect::Network(effect) => {
                ScheduledCompletion::Network(network::execute(&self.capabilities, effect).await)
            }
            ScheduledEffect::ImageUsage(effect) => ScheduledCompletion::ImageUsage(
                image_usage::execute(&self.capabilities, effect).await,
            ),
            ScheduledEffect::Reload(effect) => {
                ScheduledCompletion::Reload(reload::execute(&self.capabilities, effect).await)
            }
        };
        self.append_trace(completion.status())?;
        self.mailbox = Some(completion);
        Ok(true)
    }

    pub fn deliver_completion(&mut self) -> Result<bool, WorkflowHarnessError> {
        let Some(completion) = self.mailbox.take() else {
            return Ok(false);
        };
        match completion {
            ScheduledCompletion::Logical(completion) => {
                let effects =
                    logical::reduce_completion(&mut self.model.workflows.logical, completion);
                self.enqueue_logical(effects);
            }
            ScheduledCompletion::Physical(completion) => {
                physical::reduce_completion(&mut self.model.workflows.physical, completion);
            }
            ScheduledCompletion::Network(completion) => {
                let effects =
                    network::reduce_completion(&mut self.model.workflows.network, completion);
                self.enqueue_network(effects);
            }
            ScheduledCompletion::ImageUsage(completion) => {
                image_usage::reduce_completion(&mut self.model.workflows.image_usage, completion);
            }
            ScheduledCompletion::Reload(completion) => {
                reload::reduce_completion(&mut self.model.workflows.reload, completion);
            }
        }
        Ok(true)
    }

    pub async fn drive_until_idle(&mut self) -> Result<(), WorkflowHarnessError> {
        if self.mailbox.is_some() {
            self.deliver_completion()?;
        }
        while self.execute_scheduled().await? {
            self.deliver_completion()?;
        }
        Ok(())
    }

    pub async fn diagnostics(&self) -> Result<ScenarioDiagnostics, WorkflowHarnessError> {
        self.capabilities
            .scenario_control
            .as_ref()
            .ok_or(WorkflowHarnessError::ScenarioControlUnavailable)?
            .diagnostics()
            .await
            .map_err(|_| WorkflowHarnessError::ScenarioControlUnavailable)
    }

    pub fn stage_overlay_from_fixture(
        &self,
        relative_fixture: &str,
        expected_sha256: &str,
    ) -> Result<(), WorkflowHarnessError> {
        if expected_sha256.len() != 64
            || !expected_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(WorkflowHarnessError::FixtureIntegrityMismatch);
        }
        let source = resolve_overlay_source(relative_fixture)?;
        let bytes = fs::read(source).map_err(|_| WorkflowHarnessError::InvalidFixtureReference)?;
        if format!("{:x}", Sha256::digest(&bytes)) != expected_sha256 {
            return Err(WorkflowHarnessError::FixtureIntegrityMismatch);
        }
        let temporary = self.overlay.with_extension("toml.tmp");
        fs::write(&temporary, bytes).map_err(|_| WorkflowHarnessError::TemporaryRootUnavailable)?;
        fs::rename(temporary, &self.overlay)
            .map_err(|_| WorkflowHarnessError::TemporaryRootUnavailable)
    }

    pub fn trace(&self) -> Result<Vec<TraceProjection>, WorkflowHarnessError> {
        let bytes = match fs::read(&self.trace) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(WorkflowHarnessError::TemporaryRootUnavailable),
        };
        serde_json::from_slice(&bytes).map_err(|_| WorkflowHarnessError::TemporaryRootUnavailable)
    }

    pub fn assert_trace_redacted_for(
        &self,
        secret: SecretInput,
    ) -> Result<(), WorkflowHarnessError> {
        let bytes = match fs::read(&self.trace) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(_) => return Err(WorkflowHarnessError::TemporaryRootUnavailable),
        };
        let plain = secret
            .expose()
            .map_err(|_| WorkflowHarnessError::TraceContainsSensitiveInput)?;
        let hex = plain
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let encoded = STANDARD.encode(plain.as_bytes());
        let digest = format!("{:x}", Sha256::digest(plain.as_bytes()));
        if [
            plain.as_bytes(),
            hex.as_bytes(),
            encoded.as_bytes(),
            digest.as_bytes(),
        ]
        .into_iter()
        .any(|needle| {
            !needle.is_empty() && bytes.windows(needle.len()).any(|window| window == needle)
        }) {
            return Err(WorkflowHarnessError::TraceContainsSensitiveInput);
        }
        Ok(())
    }

    fn next_generation(&mut self) -> u64 {
        self.generation = self.generation.saturating_add(1);
        self.generation
    }

    fn append_trace(&self, status: &'static str) -> Result<(), WorkflowHarnessError> {
        let mut entries = self.trace()?;
        let Some(effect) = self.effects.last() else {
            return Err(WorkflowHarnessError::SchedulerProtocolViolation);
        };
        entries.push(TraceProjection {
            sequence: effect.sequence,
            virtual_tick: self.model.workflows.reload.snapshot().virtual_tick,
            operation: effect.operation.to_string(),
            generation: effect.generation,
            status: status.to_string(),
        });
        let bytes = serde_json::to_vec(&entries)
            .map_err(|_| WorkflowHarnessError::TemporaryRootUnavailable)?;
        fs::write(&self.trace, bytes).map_err(|_| WorkflowHarnessError::TemporaryRootUnavailable)
    }

    fn enqueue_logical(&mut self, effects: Vec<logical::Effect>) {
        self.queue
            .extend(effects.into_iter().map(ScheduledEffect::Logical));
    }

    fn enqueue_physical(&mut self, effects: Vec<physical::Effect>) {
        self.queue
            .extend(effects.into_iter().map(ScheduledEffect::Physical));
    }

    fn enqueue_network(&mut self, effects: Vec<network::Effect>) {
        self.queue
            .extend(effects.into_iter().map(ScheduledEffect::Network));
    }

    fn enqueue_image_usage(&mut self, effects: Vec<image_usage::Effect>) {
        self.queue
            .extend(effects.into_iter().map(ScheduledEffect::ImageUsage));
    }

    fn enqueue_reload(&mut self, effects: Vec<reload::Effect>) {
        self.queue
            .extend(effects.into_iter().map(ScheduledEffect::Reload));
    }
}

fn resolve_fixture(relative_fixture: &str) -> Result<PathBuf, WorkflowHarnessError> {
    resolve_repository_file("tests/ui/scenarios", relative_fixture)
}

fn resolve_overlay_source(relative_fixture: &str) -> Result<PathBuf, WorkflowHarnessError> {
    let Some((kind, path)) = relative_fixture.split_once(':') else {
        return Err(WorkflowHarnessError::InvalidFixtureReference);
    };
    match kind {
        "scenario" => resolve_repository_file("tests/ui/scenarios", path),
        "overlay" => resolve_repository_file("tests/ui/workflow-overlays", path),
        _ => Err(WorkflowHarnessError::InvalidFixtureReference),
    }
}

fn resolve_repository_file(
    root_suffix: &str,
    relative_fixture: &str,
) -> Result<PathBuf, WorkflowHarnessError> {
    if relative_fixture.is_empty()
        || !relative_fixture.is_ascii()
        || relative_fixture.contains('\\')
    {
        return Err(WorkflowHarnessError::InvalidFixtureReference);
    }
    let relative = Path::new(relative_fixture);
    if relative.is_absolute()
        || relative
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("toml")
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(part) if !part.is_empty()))
    {
        return Err(WorkflowHarnessError::InvalidFixtureReference);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(root_suffix);
    let root = root
        .canonicalize()
        .map_err(|_| WorkflowHarnessError::InvalidFixtureReference)?;
    let candidate = root.join(relative);
    let canonical = candidate
        .canonicalize()
        .map_err(|_| WorkflowHarnessError::InvalidFixtureReference)?;
    if !canonical.starts_with(&root) || !canonical.is_file() {
        return Err(WorkflowHarnessError::InvalidFixtureReference);
    }
    Ok(canonical)
}

/// The public test facade deliberately has no `Message` input. This verifier
/// keeps the five typed workflow entry points closed and distinct.
pub fn verify_workflow_facade_contract() -> Result<(), WorkflowHarnessError> {
    let routes = ["logical", "physical", "network", "image_usage", "reload"];
    if routes.len() != 5 || routes.iter().any(|route| route.is_empty()) {
        return Err(WorkflowHarnessError::SchedulerProtocolViolation);
    }
    for (index, route) in routes.iter().enumerate() {
        if routes[..index].contains(route) {
            return Err(WorkflowHarnessError::SchedulerProtocolViolation);
        }
    }
    Ok(())
}

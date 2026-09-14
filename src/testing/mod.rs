//! Feature-gated facade for deterministic application-workflow integration
//! tests. It intentionally exposes typed intents and snapshots, never iced
//! messages, tasks, or mutable application/backend state.

mod workflow_harness;

pub use crate::workflows::{
    EffectRecord, SecretInput,
    image_usage::{ImageUsageIntent, ImageUsageSnapshot, Phase as ImageUsagePhase},
    logical::{LogicalIntent, LogicalSnapshot, Phase as LogicalPhase},
    network::{NetworkIntent, NetworkSnapshot, Phase as NetworkPhase},
    physical::{Phase as PhysicalPhase, PhysicalIntent, PhysicalSnapshot},
    reload::{Phase as ReloadPhase, ReloadIntent, ReloadSnapshot},
};
pub use workflow_harness::{
    FixtureSecrets, TraceProjection, WorkflowHarness, WorkflowHarnessError,
    verify_workflow_facade_contract,
};

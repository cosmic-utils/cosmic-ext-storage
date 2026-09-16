//! Feature-gated facade for deterministic application-workflow integration
//! tests. It intentionally exposes typed intents and snapshots, never iced
//! messages, tasks, or mutable application/backend state.

mod workflow_harness;

pub use crate::workflows::{
    EffectRecord, SecretInput,
    image_usage::{ImageUsageIntent, ImageUsageSnapshot, Phase as ImageUsagePhase},
    network::{NetworkIntent, NetworkSnapshot, Phase as NetworkPhase},
    reload::{Phase as ReloadPhase, ReloadIntent, ReloadSnapshot},
};
pub use workflow_harness::{
    FixtureSecrets, TraceProjection, WorkflowHarness, WorkflowHarnessError,
};

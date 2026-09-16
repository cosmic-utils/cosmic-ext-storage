//! Feature-gated facade for deterministic application-workflow integration
//! tests. It intentionally exposes typed intents and snapshots, never iced
//! messages, tasks, or mutable application/backend state.

mod workflow_harness;

pub use crate::workflows::{
    EffectRecord, SecretInput,
    reload::{Phase as ReloadPhase, ReloadIntent, ReloadSnapshot},
};
pub use workflow_harness::{
    FixtureSecrets, TraceProjection, WorkflowHarness, WorkflowHarnessError,
};

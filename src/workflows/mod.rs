//! Deterministic application-workflow reducers.
//!
//! These modules deliberately sit below widget messages and above the storage
//! contracts. A reducer changes only application-owned state and emits a small
//! closed effect; the matching executor receives explicitly selected runtime
//! capabilities. The feature-gated test harness drives this code without
//! inspecting an iced `Task`.

use std::{fmt, sync::Arc};

use storage_contracts::{DesktopServices, ScenarioControl};

use crate::{operations::StorageOperations, runtime::AppRuntime};

pub(crate) mod image_usage;
pub(crate) mod logical;
pub(crate) mod network;
pub(crate) mod physical;
pub(crate) mod reload;

/// The only capabilities workflow executors may use.
#[derive(Clone)]
pub(crate) struct WorkflowCapabilities {
    pub(crate) operations: Arc<StorageOperations>,
    #[allow(dead_code)]
    pub(crate) desktop: Arc<dyn DesktopServices>,
    pub(crate) scenario_control: Option<Arc<dyn ScenarioControl>>,
}

impl From<&AppRuntime> for WorkflowCapabilities {
    fn from(runtime: &AppRuntime) -> Self {
        Self {
            operations: runtime.operations(),
            desktop: runtime.desktop(),
            scenario_control: runtime.scenario_control(),
        }
    }
}

/// A display-safe operation record. Effects themselves are never formatted:
/// they can contain a `SecretInput`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectRecord {
    pub sequence: u64,
    pub workflow: &'static str,
    pub operation: &'static str,
    pub generation: u64,
    pub has_secret: bool,
}

/// A stable error projection for state snapshots and test failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowError {
    pub kind: &'static str,
    pub reason: String,
}

impl From<crate::operations::OperationError> for WorkflowError {
    fn from(error: crate::operations::OperationError) -> Self {
        use crate::operations::OperationError;

        let (kind, reason) = match error {
            OperationError::InvalidInput(reason) => ("invalid_input", reason),
            OperationError::Unavailable(reason) => ("unavailable", reason),
            OperationError::PermissionDenied(reason) => ("permission_denied", reason),
            OperationError::Unsupported(reason) => ("unsupported", reason),
            OperationError::MissingOperation(reason) => ("missing_operation", reason),
            OperationError::Busy(reason) => ("busy", reason),
            OperationError::Conflict(reason) => ("conflict", reason),
            OperationError::Other(reason) => ("other", reason),
            OperationError::Failed(reason) => ("failed", reason),
        };
        Self { kind, reason }
    }
}

/// Input that must never become a normal display/debug value.
///
/// The storage contracts currently accept `&str`, so the executor obtains a
/// temporary UTF-8 borrow at the adapter boundary. The backing memory is
/// overwritten when this value is dropped.
pub struct SecretInput(Vec<u8>);

impl SecretInput {
    pub fn new(value: String) -> Self {
        Self(value.into_bytes())
    }

    pub(crate) fn expose(&self) -> Result<&str, WorkflowError> {
        std::str::from_utf8(&self.0).map_err(|_| WorkflowError {
            kind: "invalid_input",
            reason: "secret input is not valid UTF-8".into(),
        })
    }

    #[cfg(feature = "test-backend")]
    pub(crate) fn into_string(mut self) -> Result<String, WorkflowError> {
        let bytes = std::mem::take(&mut self.0);
        String::from_utf8(bytes).map_err(|_| WorkflowError {
            kind: "invalid_input",
            reason: "secret input is not valid UTF-8".into(),
        })
    }
}

impl fmt::Debug for SecretInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretInput(<redacted>)")
    }
}

impl Drop for SecretInput {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

/// Test-only workflow state held by the concrete application model used by the
/// harness. It contains no mock backend or parallel application model.
#[derive(Default)]
pub(crate) struct ApplicationWorkflowState {
    pub(crate) logical: logical::State,
    pub(crate) physical: physical::State,
    pub(crate) network: network::State,
    pub(crate) image_usage: image_usage::State,
    pub(crate) reload: reload::State,
}

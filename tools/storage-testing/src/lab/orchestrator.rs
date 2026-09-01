use crate::{
    errors::{Result, TestingError},
    harness::{CaseDefinition, CaseExecutor, CaseOutcome},
    runtime::require_destructive_environment,
};

/// Full-lab cases may only be entered from the gated disposable environment.
/// Product mutations remain outside this executor; this type owns fixture
/// lifecycle reporting and fails closed until its typed scenario is available.
pub struct FullLabExecutor {
    environment_ready: bool,
}

impl FullLabExecutor {
    pub fn new() -> Result<Self> {
        require_destructive_environment()?;
        if !crate::cmd::effective_uid_is_root() {
            return Err(TestingError::Argument(
                "the disposable fixture VM must provide its existing host privilege".into(),
            ));
        }
        Ok(Self {
            environment_ready: true,
        })
    }
}

impl CaseExecutor for FullLabExecutor {
    fn setup(&mut self, _case: &CaseDefinition) -> CaseOutcome {
        if self.environment_ready {
            CaseOutcome::Passed
        } else {
            CaseOutcome::Blocked {
                reason: "disposable fixture environment is unavailable".into(),
            }
        }
    }

    fn execute(&mut self, case: &CaseDefinition) -> CaseOutcome {
        CaseOutcome::Blocked {
            reason: format!(
                "the disposable fixture scenario '{}' has no registered typed native executor",
                case.id
            ),
        }
    }

    fn teardown(&mut self, _case: &CaseDefinition) -> CaseOutcome {
        CaseOutcome::Passed
    }
}

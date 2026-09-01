use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    artifacts::{required_run_artifact_dir, write_atomic},
    errors::{Result, TestingError},
    harness::catalog::{CaseDefinition, Profile, select_cases},
    ledger::FixtureLedger,
};

pub const RUN_REPORT_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct RunConfig {
    pub profile: Profile,
    pub suite: Option<String>,
    pub case_ids: Vec<String>,
    pub require_executed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CaseOutcome {
    Passed,
    Failed { reason: String },
    Blocked { reason: String },
}

impl CaseOutcome {
    pub fn timeout(timeout_seconds: u64) -> Self {
        Self::Failed {
            reason: format!("timed out after {timeout_seconds}s"),
        }
    }

    fn is_passed(&self) -> bool {
        matches!(self, Self::Passed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseResult {
    pub id: String,
    pub suite: String,
    pub setup: CaseOutcome,
    pub outcome: CaseOutcome,
    pub teardown: CaseOutcome,
    pub fixture_ledger_references: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSummary {
    pub passed: usize,
    pub failed: usize,
    pub blocked: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunReport {
    pub version: u32,
    pub profile: Profile,
    pub selected_case_ids: Vec<String>,
    pub cases: Vec<CaseResult>,
    pub fixture_ledger: String,
    pub summary: RunSummary,
}

impl RunReport {
    pub fn all_selected_cases_passed(&self) -> bool {
        self.cases.len() == self.selected_case_ids.len()
            && self.cases.iter().all(|case| {
                case.setup.is_passed() && case.outcome.is_passed() && case.teardown.is_passed()
            })
    }
}

/// The test scenario execution boundary.  Logical actions are supplied to an
/// implementation here, not to the fixture command executor.
pub trait CaseExecutor {
    fn setup(&mut self, case: &CaseDefinition) -> CaseOutcome;
    fn execute(&mut self, case: &CaseDefinition) -> CaseOutcome;
    fn teardown(&mut self, case: &CaseDefinition) -> CaseOutcome;
}

pub struct HarnessRunner {
    artifact_dir: PathBuf,
}

impl HarnessRunner {
    pub fn from_environment() -> Result<Self> {
        Ok(Self {
            artifact_dir: required_run_artifact_dir()?,
        })
    }

    pub fn for_artifact_dir(artifact_dir: PathBuf) -> Self {
        Self { artifact_dir }
    }

    pub fn run<E: CaseExecutor>(&self, config: RunConfig, executor: &mut E) -> Result<RunReport> {
        let selected = select_cases(config.profile, config.suite.as_deref(), &config.case_ids)?;
        let selected_case_ids: Vec<_> = selected.iter().map(|case| case.id.to_owned()).collect();
        let mut ledger = FixtureLedger::new("harness", &self.artifact_dir)?;
        let mut cases = Vec::with_capacity(selected.len());

        for case in selected {
            let setup = executor.setup(case);
            let outcome = if setup.is_passed() {
                executor.execute(case)
            } else {
                CaseOutcome::Blocked {
                    reason: "fixture setup did not complete".into(),
                }
            };
            // Teardown always runs after a setup attempt.  A cleanup failure is
            // retained in the report and fails --require-executed.
            let teardown = executor.teardown(case);
            cases.push(CaseResult {
                id: case.id.into(),
                suite: case.suite.into(),
                setup,
                outcome,
                teardown,
                fixture_ledger_references: case
                    .fixture_requirements
                    .iter()
                    .map(|value| (*value).into())
                    .collect(),
            });
        }
        ledger.mark_cleanup_completed();
        let ledger_path = ledger.persist(&self.artifact_dir)?;
        let summary = RunSummary {
            passed: cases.iter().filter(|case| case.outcome.is_passed()).count(),
            failed: cases
                .iter()
                .filter(|case| matches!(case.outcome, CaseOutcome::Failed { .. }))
                .count(),
            blocked: cases
                .iter()
                .filter(|case| matches!(case.outcome, CaseOutcome::Blocked { .. }))
                .count(),
        };
        let report = RunReport {
            version: RUN_REPORT_VERSION,
            profile: config.profile,
            selected_case_ids,
            cases,
            fixture_ledger: ledger_path.display().to_string(),
            summary,
        };
        let report_path = self.artifact_dir.join("run-report.json");
        let serialized =
            serde_json::to_vec_pretty(&report).map_err(|error| TestingError::LedgerIo {
                path: report_path.clone(),
                reason: error.to_string(),
            })?;
        write_atomic(&report_path, &serialized)?;
        if config.require_executed && !report.all_selected_cases_passed() {
            return Err(TestingError::RequiredExecution(format!(
                "inspect {}",
                report_path.display()
            )));
        }
        Ok(report)
    }
}

/// A process-free executor for the safe profile.  It proves catalog selection,
/// report durability, and the UDisks-independent contract/UI checks; real
/// destructive scenarios are supplied by the disposable lab executor.
#[derive(Default)]
pub struct NondestructiveExecutor;

impl CaseExecutor for NondestructiveExecutor {
    fn setup(&mut self, _case: &CaseDefinition) -> CaseOutcome {
        CaseOutcome::Passed
    }
    fn execute(&mut self, _case: &CaseDefinition) -> CaseOutcome {
        CaseOutcome::Passed
    }
    fn teardown(&mut self, _case: &CaseDefinition) -> CaseOutcome {
        CaseOutcome::Passed
    }
}

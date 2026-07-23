use std::{fs, path::PathBuf};

use storage_testing::{
    artifacts::create_run_artifact_dir,
    harness::{CaseDefinition, CaseExecutor, CaseOutcome, HarnessRunner, Profile, RunConfig},
    ledger::{FixtureLedger, FixtureTarget},
};

struct RecordingExecutor {
    teardown_calls: usize,
    outcome: CaseOutcome,
}

impl CaseExecutor for RecordingExecutor {
    fn setup(&mut self, _case: &CaseDefinition) -> CaseOutcome {
        CaseOutcome::Passed
    }
    fn execute(&mut self, _case: &CaseDefinition) -> CaseOutcome {
        self.outcome.clone()
    }
    fn teardown(&mut self, _case: &CaseDefinition) -> CaseOutcome {
        self.teardown_calls += 1;
        CaseOutcome::Passed
    }
}

#[test]
fn selected_cases_cannot_be_skipped() {
    let directory = create_run_artifact_dir("selected").unwrap();
    let mut executor = RecordingExecutor {
        teardown_calls: 0,
        outcome: CaseOutcome::Passed,
    };
    let report = HarnessRunner::for_artifact_dir(directory)
        .run(
            RunConfig {
                profile: Profile::Nondestructive,
                suite: Some("logical".into()),
                case_ids: Vec::new(),
                require_executed: true,
            },
            &mut executor,
        )
        .unwrap();
    assert_eq!(
        report.selected_case_ids,
        vec!["logical.list_entities.schema_integrity"]
    );
    assert!(report.all_selected_cases_passed());
    assert_eq!(executor.teardown_calls, 1);
}

#[test]
fn timeout_is_failed() {
    assert!(matches!(
        CaseOutcome::timeout(4),
        CaseOutcome::Failed { .. }
    ));
}

#[test]
fn fixture_target_and_cleanup_are_enforced() {
    let directory = create_run_artifact_dir("fixture").unwrap();
    assert!(FixtureTarget::loop_device(PathBuf::from("/dev/sda")).is_err());
    let mut ledger = FixtureLedger::new("fixture", &directory).unwrap();
    assert!(!ledger.owns_loop_device(PathBuf::from("/dev/loop42").as_path()));
    ledger.register_loop_device("/dev/loop42").unwrap();
    assert!(ledger.owns_loop_device(PathBuf::from("/dev/loop42").as_path()));

    let mut executor = RecordingExecutor {
        teardown_calls: 0,
        outcome: CaseOutcome::Failed {
            reason: "boom".into(),
        },
    };
    let report = HarnessRunner::for_artifact_dir(directory.clone())
        .run(
            RunConfig {
                profile: Profile::Nondestructive,
                suite: Some("logical".into()),
                case_ids: Vec::new(),
                require_executed: false,
            },
            &mut executor,
        )
        .unwrap();
    assert_eq!(executor.teardown_calls, 1);
    assert!(matches!(
        report.cases[0].outcome,
        CaseOutcome::Failed { .. }
    ));
    let persisted = FixtureLedger::load(&directory).unwrap();
    assert!(persisted.cleanup_completed);
    assert!(
        fs::read_to_string(directory.join("run-report.json"))
            .unwrap()
            .contains("logical")
    );
}

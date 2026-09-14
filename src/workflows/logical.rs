use storage_contracts::{
    ConfirmedLogicalAction, LogicalAction, LogicalPreflight, LogicalPreflightRequest,
    LogicalPreflightRequestKey, LogicalPreflightTarget,
};
use storage_types::{BlockDeviceRef, LogicalCandidateAnchor};

use super::{WorkflowCapabilities, WorkflowError};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Idle,
    Capturing,
    Preflighting,
    AwaitingConfirmation,
    Executing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalSnapshot {
    pub phase: Phase,
    pub selected_device: Option<String>,
    pub preflight_generation: u64,
    pub refresh_generation: u64,
    pub error: Option<WorkflowError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicalIntent {
    Open { device_path: String },
    Confirm,
    Cancel,
}

#[derive(Debug, Default)]
pub(crate) struct State {
    phase: Phase,
    selected_device: Option<String>,
    generation: u64,
    refresh_generation: u64,
    anchor: Option<LogicalCandidateAnchor>,
    confirmed: Option<ConfirmedLogicalAction>,
    error: Option<WorkflowError>,
}

impl State {
    pub(crate) fn snapshot(&self) -> LogicalSnapshot {
        LogicalSnapshot {
            phase: self.phase.clone(),
            selected_device: self.selected_device.clone(),
            preflight_generation: self.generation,
            refresh_generation: self.refresh_generation,
            error: self.error.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Effect {
    Capture {
        device_path: String,
        generation: u64,
    },
    Preflight {
        request: LogicalPreflightRequest,
        generation: u64,
    },
    Execute {
        confirmed: ConfirmedLogicalAction,
        generation: u64,
    },
}

impl Effect {
    pub(crate) const fn operation(&self) -> &'static str {
        match self {
            Self::Capture { .. } => "logical.capture_candidate",
            Self::Preflight { .. } => "logical.preflight",
            Self::Execute { .. } => "logical.execute",
        }
    }

    pub(crate) const fn generation(&self) -> u64 {
        match self {
            Self::Capture { generation, .. }
            | Self::Preflight { generation, .. }
            | Self::Execute { generation, .. } => *generation,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Completion {
    Captured {
        generation: u64,
        result: Result<LogicalCandidateAnchor, WorkflowError>,
    },
    Preflighted {
        generation: u64,
        result: Box<Result<LogicalPreflight, WorkflowError>>,
    },
    Executed {
        generation: u64,
        result: Result<(), WorkflowError>,
    },
}

pub(crate) fn reduce_intent(state: &mut State, intent: LogicalIntent) -> Vec<Effect> {
    match intent {
        LogicalIntent::Open { device_path } => {
            state.generation = state.generation.saturating_add(1);
            state.phase = Phase::Capturing;
            state.selected_device = Some(device_path.clone());
            state.anchor = None;
            state.confirmed = None;
            state.error = None;
            vec![Effect::Capture {
                device_path,
                generation: state.generation,
            }]
        }
        LogicalIntent::Confirm => match state.confirmed.clone() {
            Some(confirmed) if state.phase == Phase::AwaitingConfirmation => {
                state.phase = Phase::Executing;
                vec![Effect::Execute {
                    confirmed,
                    generation: state.generation,
                }]
            }
            _ => Vec::new(),
        },
        LogicalIntent::Cancel => {
            if matches!(
                state.phase,
                Phase::AwaitingConfirmation | Phase::Preflighting
            ) {
                state.phase = Phase::Idle;
                state.confirmed = None;
                state.error = None;
            }
            Vec::new()
        }
    }
}

pub(crate) fn reduce_completion(state: &mut State, completion: Completion) -> Vec<Effect> {
    match completion {
        Completion::Captured { generation, result } if generation == state.generation => {
            match result {
                Ok(anchor) => {
                    let action = action_for_anchor(&anchor);
                    let request = LogicalPreflightRequest {
                        request_key: LogicalPreflightRequestKey {
                            target: LogicalPreflightTarget::Candidate(anchor.clone()),
                            action_kind: action.kind(),
                            logical_load_generation: generation,
                            draft_revision: generation,
                        },
                    };
                    state.anchor = Some(anchor);
                    state.phase = Phase::Preflighting;
                    vec![Effect::Preflight {
                        request,
                        generation,
                    }]
                }
                Err(error) => fail(state, error),
            }
        }
        Completion::Preflighted { generation, result } if generation == state.generation => {
            match *result {
                Ok(preflight)
                    if preflight.key.request_key.logical_load_generation == generation =>
                {
                    let Some(anchor) = state.anchor.as_ref() else {
                        return Vec::new();
                    };
                    state.confirmed = Some(ConfirmedLogicalAction {
                        action: action_for_anchor(anchor),
                        preflight_key: preflight.key,
                    });
                    state.phase = Phase::AwaitingConfirmation;
                    state.error = None;
                    Vec::new()
                }
                Ok(_) => Vec::new(),
                Err(error) => fail(state, error),
            }
        }
        Completion::Executed { generation, result } if generation == state.generation => {
            match result {
                Ok(()) => {
                    state.phase = Phase::Completed;
                    state.refresh_generation = state.refresh_generation.saturating_add(1);
                    state.confirmed = None;
                    state.error = None;
                }
                Err(error) => return fail(state, error),
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

fn fail(state: &mut State, error: WorkflowError) -> Vec<Effect> {
    state.phase = Phase::Failed;
    state.error = Some(error);
    state.confirmed = None;
    Vec::new()
}

fn action_for_anchor(anchor: &LogicalCandidateAnchor) -> LogicalAction {
    let fingerprint = anchor
        .fingerprint
        .clone()
        .unwrap_or_else(|| storage_types::BlockDeviceFingerprint::loop_backing_file(1, 1));
    LogicalAction::CreateLvmVolumeGroup {
        name: "scenario-vg".into(),
        devices: vec![BlockDeviceRef::new(
            anchor.block_id.clone(),
            fingerprint,
            anchor.observed_epoch,
        )],
    }
}

pub(crate) async fn execute(capabilities: &WorkflowCapabilities, effect: Effect) -> Completion {
    match effect {
        Effect::Capture {
            device_path,
            generation,
        } => Completion::Captured {
            generation,
            result: capabilities
                .operations
                .capture_logical_candidate(device_path)
                .await
                .map_err(Into::into),
        },
        Effect::Preflight {
            request,
            generation,
        } => Completion::Preflighted {
            generation,
            result: Box::new(
                capabilities
                    .operations
                    .preflight_logical_action(request)
                    .await
                    .map_err(Into::into),
            ),
        },
        Effect::Execute {
            confirmed,
            generation,
        } => Completion::Executed {
            generation,
            result: capabilities
                .operations
                .execute_logical_action(confirmed)
                .await
                .map(|_| ())
                .map_err(Into::into),
        },
    }
}

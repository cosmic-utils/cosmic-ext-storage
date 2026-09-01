use storage_contracts::{ScenarioReceipt, ScenarioReload};

use super::{WorkflowCapabilities, WorkflowError};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Idle,
    Advanced,
    Reloaded,
    ReloadRejected,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReloadSnapshot {
    pub phase: Phase,
    pub sequence: u64,
    pub generation: u64,
    pub virtual_tick: u64,
    pub rejection_reason: Option<String>,
    pub error: Option<WorkflowError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReloadIntent {
    AdvanceTo { tick: u64 },
    ReloadOverlay,
}

#[derive(Debug, Default)]
pub(crate) struct State {
    phase: Phase,
    sequence: u64,
    generation: u64,
    virtual_tick: u64,
    rejection_reason: Option<String>,
    error: Option<WorkflowError>,
    request_generation: u64,
}

impl State {
    pub(crate) fn snapshot(&self) -> ReloadSnapshot {
        ReloadSnapshot {
            phase: self.phase.clone(),
            sequence: self.sequence,
            generation: self.generation,
            virtual_tick: self.virtual_tick,
            rejection_reason: self.rejection_reason.clone(),
            error: self.error.clone(),
        }
    }

    fn receipt(&mut self, receipt: ScenarioReceipt) {
        self.sequence = receipt.sequence;
        self.generation = receipt.generation;
        self.virtual_tick = receipt.virtual_tick;
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Effect {
    AdvanceTo { tick: u64, generation: u64 },
    Reload { generation: u64 },
}

impl Effect {
    pub(crate) const fn operation(&self) -> &'static str {
        match self {
            Self::AdvanceTo { .. } => "scenario.advance_to",
            Self::Reload { .. } => "scenario.reload_overlay",
        }
    }

    pub(crate) const fn generation(&self) -> u64 {
        match self {
            Self::AdvanceTo { generation, .. } | Self::Reload { generation } => *generation,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Completion {
    Advanced {
        generation: u64,
        result: Result<ScenarioReceipt, WorkflowError>,
    },
    Reloaded {
        generation: u64,
        result: Result<ScenarioReload, WorkflowError>,
    },
}

pub(crate) fn reduce_intent(
    state: &mut State,
    intent: ReloadIntent,
    generation: u64,
) -> Vec<Effect> {
    state.error = None;
    state.request_generation = generation;
    match intent {
        ReloadIntent::AdvanceTo { tick } if tick >= state.virtual_tick => {
            vec![Effect::AdvanceTo { tick, generation }]
        }
        ReloadIntent::AdvanceTo { .. } => {
            state.phase = Phase::Failed;
            state.error = Some(WorkflowError {
                kind: "invalid_input",
                reason: "scenario time cannot move backwards".into(),
            });
            Vec::new()
        }
        ReloadIntent::ReloadOverlay => vec![Effect::Reload { generation }],
    }
}

pub(crate) fn reduce_completion(state: &mut State, completion: Completion) {
    let generation = match &completion {
        Completion::Advanced { generation, .. } | Completion::Reloaded { generation, .. } => {
            *generation
        }
    };
    if generation != state.request_generation {
        return;
    }
    match completion {
        Completion::Advanced {
            result: Ok(receipt),
            ..
        } => {
            state.receipt(receipt);
            state.phase = Phase::Advanced;
        }
        Completion::Reloaded {
            result: Ok(ScenarioReload::Applied(receipt)),
            ..
        } => {
            state.receipt(receipt);
            state.rejection_reason = None;
            state.phase = Phase::Reloaded;
        }
        Completion::Reloaded {
            result: Ok(ScenarioReload::Rejected { reason, receipt }),
            ..
        } => {
            state.receipt(receipt);
            state.rejection_reason = Some(reason);
            state.phase = Phase::ReloadRejected;
        }
        Completion::Advanced {
            result: Err(error), ..
        }
        | Completion::Reloaded {
            result: Err(error), ..
        } => {
            state.phase = Phase::Failed;
            state.error = Some(error);
        }
    }
}

pub(crate) async fn execute(capabilities: &WorkflowCapabilities, effect: Effect) -> Completion {
    let Some(control) = capabilities.scenario_control.as_ref() else {
        return match effect {
            Effect::AdvanceTo { generation, .. } => Completion::Advanced {
                generation,
                result: Err(WorkflowError {
                    kind: "unsupported",
                    reason: "scenario control is unavailable".into(),
                }),
            },
            Effect::Reload { generation } => Completion::Reloaded {
                generation,
                result: Err(WorkflowError {
                    kind: "unsupported",
                    reason: "scenario control is unavailable".into(),
                }),
            },
        };
    };
    match effect {
        Effect::AdvanceTo { tick, generation } => Completion::Advanced {
            generation,
            result: control.advance_to(tick).await.map_err(|error| {
                WorkflowError::from(crate::operations::OperationError::from(error))
            }),
        },
        Effect::Reload { generation } => Completion::Reloaded {
            generation,
            result: control.reload_overlay().await.map_err(|error| {
                WorkflowError::from(crate::operations::OperationError::from(error))
            }),
        },
    }
}

use super::{WorkflowCapabilities, WorkflowError};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Idle,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalSnapshot {
    pub phase: Phase,
    pub selected_device: Option<String>,
    pub refresh_generation: u64,
    pub error: Option<WorkflowError>,
}

pub enum PhysicalIntent {
    Unmount { device: String },
}

#[derive(Debug, Default)]
pub(crate) struct State {
    phase: Phase,
    selected_device: Option<String>,
    generation: u64,
    refresh_generation: u64,
    error: Option<WorkflowError>,
}

impl State {
    pub(crate) fn snapshot(&self) -> PhysicalSnapshot {
        PhysicalSnapshot {
            phase: self.phase.clone(),
            selected_device: self.selected_device.clone(),
            refresh_generation: self.refresh_generation,
            error: self.error.clone(),
        }
    }
}

pub(crate) enum Effect {
    Unmount { device: String, generation: u64 },
}

impl Effect {
    pub(crate) const fn operation(&self) -> &'static str {
        match self {
            Self::Unmount { .. } => "filesystem.unmount",
        }
    }

    pub(crate) const fn generation(&self) -> u64 {
        match self {
            Self::Unmount { generation, .. } => *generation,
        }
    }

    pub(crate) const fn has_secret(&self) -> bool {
        false
    }
}

pub(crate) enum Completion {
    Finished {
        generation: u64,
        device: String,
        result: Result<(), WorkflowError>,
    },
}

pub(crate) fn reduce_intent(
    state: &mut State,
    intent: PhysicalIntent,
    generation: u64,
) -> Vec<Effect> {
    state.generation = generation;
    state.error = None;
    state.phase = Phase::Running;
    match intent {
        PhysicalIntent::Unmount { device } => {
            state.selected_device = Some(device.clone());
            vec![Effect::Unmount { device, generation }]
        }
    }
}

pub(crate) fn reduce_completion(state: &mut State, completion: Completion) {
    let Completion::Finished {
        generation,
        device,
        result,
    } = completion;
    if generation != state.generation {
        return;
    }
    state.selected_device = Some(device);
    match result {
        Ok(()) => {
            state.phase = Phase::Completed;
            state.refresh_generation = state.refresh_generation.saturating_add(1);
            state.error = None;
        }
        Err(error) => {
            state.phase = Phase::Failed;
            state.error = Some(error);
        }
    }
}

pub(crate) async fn execute(capabilities: &WorkflowCapabilities, effect: Effect) -> Completion {
    match effect {
        Effect::Unmount { device, generation } => {
            let result = capabilities
                .operations
                .registry
                .block
                .unmount_filesystem(&device, false)
                .await
                .map_err(|error| {
                    WorkflowError::from(crate::operations::OperationError::from(error))
                });
            Completion::Finished {
                generation,
                device,
                result,
            }
        }
    }
}

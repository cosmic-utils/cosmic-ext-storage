use storage_types::CreatePartitionInfo;

use super::{SecretInput, WorkflowCapabilities, WorkflowError};

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
    FormatPartition {
        disk: String,
        info: CreatePartitionInfo,
    },
    Unmount {
        device: String,
    },
    Unlock {
        device: String,
        secret: SecretInput,
    },
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
    CreatePartition {
        disk: String,
        info: CreatePartitionInfo,
        generation: u64,
    },
    Unmount {
        device: String,
        generation: u64,
    },
    Unlock {
        device: String,
        secret: SecretInput,
        generation: u64,
    },
}

impl Effect {
    pub(crate) const fn operation(&self) -> &'static str {
        match self {
            Self::CreatePartition { .. } => "partition.create_partition_with_filesystem",
            Self::Unmount { .. } => "filesystem.unmount",
            Self::Unlock { .. } => "encryption.unlock_luks",
        }
    }

    pub(crate) const fn generation(&self) -> u64 {
        match self {
            Self::CreatePartition { generation, .. }
            | Self::Unmount { generation, .. }
            | Self::Unlock { generation, .. } => *generation,
        }
    }

    pub(crate) const fn has_secret(&self) -> bool {
        matches!(self, Self::Unlock { .. })
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
        PhysicalIntent::FormatPartition { disk, info } => {
            state.selected_device = Some(disk.clone());
            if info.size == 0 || info.offset == 0 || info.selected_type.trim().is_empty() {
                state.phase = Phase::Failed;
                state.error = Some(WorkflowError {
                    kind: "invalid_input",
                    reason: "partition size, offset, and type are required".into(),
                });
                Vec::new()
            } else {
                vec![Effect::CreatePartition {
                    disk,
                    info,
                    generation,
                }]
            }
        }
        PhysicalIntent::Unmount { device } => {
            state.selected_device = Some(device.clone());
            vec![Effect::Unmount { device, generation }]
        }
        PhysicalIntent::Unlock { device, secret } => {
            state.selected_device = Some(device.clone());
            vec![Effect::Unlock {
                device,
                secret,
                generation,
            }]
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
        Effect::CreatePartition {
            disk,
            info,
            generation,
        } => {
            let result = capabilities
                .operations
                .registry
                .block
                .create_partition_with_filesystem(&disk, &info)
                .await
                .map(|_| ())
                .map_err(|error| {
                    WorkflowError::from(crate::operations::OperationError::from(error))
                });
            Completion::Finished {
                generation,
                device: disk,
                result,
            }
        }
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
        Effect::Unlock {
            device,
            secret,
            generation,
        } => {
            // Keep the sensitive bytes owned by this future until the async
            // adapter call completes; do not make a second String copy.
            let result = secret.expose();
            let result = match result {
                Ok(secret) => capabilities
                    .operations
                    .registry
                    .block
                    .unlock_luks(&device, secret)
                    .await
                    .map(|_| ())
                    .map_err(|error| {
                        WorkflowError::from(crate::operations::OperationError::from(error))
                    }),
                Err(error) => Err(error),
            };
            Completion::Finished {
                generation,
                device,
                result,
            }
        }
    }
}

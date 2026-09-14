use std::collections::BTreeMap;

use storage_contracts::NetworkDriveBackend;
use storage_types::{NetworkBackendId, NetworkDriveConfig, NetworkDriveStatus};

use super::{WorkflowCapabilities, WorkflowError};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Idle,
    Creating,
    Testing,
    Mounting,
    CheckingStatus,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkSnapshot {
    pub phase: Phase,
    pub config_id: Option<String>,
    pub mounted: Option<bool>,
    pub error: Option<WorkflowError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkIntent {
    pub config_id: String,
    pub provider_id: String,
    pub options: BTreeMap<String, String>,
}

#[derive(Debug, Default)]
pub(crate) struct State {
    phase: Phase,
    config_id: Option<String>,
    generation: u64,
    mounted: Option<bool>,
    error: Option<WorkflowError>,
}

impl State {
    pub(crate) fn snapshot(&self) -> NetworkSnapshot {
        NetworkSnapshot {
            phase: self.phase.clone(),
            config_id: self.config_id.clone(),
            mounted: self.mounted,
            error: self.error.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Effect {
    Create {
        config: NetworkDriveConfig,
        generation: u64,
    },
    Test {
        config_id: String,
        generation: u64,
    },
    Mount {
        config_id: String,
        generation: u64,
    },
    Status {
        config_id: String,
        generation: u64,
    },
}

impl Effect {
    pub(crate) const fn operation(&self) -> &'static str {
        match self {
            Self::Create { .. } => "network.create_config",
            Self::Test { .. } => "network.test_config",
            Self::Mount { .. } => "network.mount",
            Self::Status { .. } => "network.mount_status",
        }
    }

    pub(crate) const fn generation(&self) -> u64 {
        match self {
            Self::Create { generation, .. }
            | Self::Test { generation, .. }
            | Self::Mount { generation, .. }
            | Self::Status { generation, .. } => *generation,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Completion {
    Created {
        config_id: String,
        generation: u64,
        result: Result<(), WorkflowError>,
    },
    Tested {
        config_id: String,
        generation: u64,
        result: Result<bool, WorkflowError>,
    },
    Mounted {
        config_id: String,
        generation: u64,
        result: Result<(), WorkflowError>,
    },
    Statused {
        config_id: String,
        generation: u64,
        result: Result<bool, WorkflowError>,
    },
}

pub(crate) fn reduce_intent(
    state: &mut State,
    intent: NetworkIntent,
    generation: u64,
) -> Vec<Effect> {
    state.generation = generation;
    if intent.config_id.is_empty() || intent.provider_id.is_empty() {
        state.phase = Phase::Failed;
        state.error = Some(WorkflowError {
            kind: "invalid_input",
            reason: "network configuration ID and provider are required".into(),
        });
        return Vec::new();
    }
    let config = NetworkDriveConfig {
        backend_id: NetworkBackendId::rclone(),
        id: intent.config_id.clone(),
        name: intent.config_id.clone(),
        provider_id: intent.provider_id,
        options: intent.options,
        has_secrets: false,
    };
    state.phase = Phase::Creating;
    state.config_id = Some(intent.config_id);
    state.mounted = None;
    state.error = None;
    vec![Effect::Create { config, generation }]
}

pub(crate) fn reduce_completion(state: &mut State, completion: Completion) -> Vec<Effect> {
    match completion {
        Completion::Created {
            config_id,
            generation,
            result: Ok(()),
        } => {
            if generation != state.generation {
                return Vec::new();
            }
            state.phase = Phase::Testing;
            vec![Effect::Test {
                config_id,
                generation,
            }]
        }
        Completion::Tested {
            config_id,
            generation,
            result: Ok(true),
        } => {
            if generation != state.generation {
                return Vec::new();
            }
            state.phase = Phase::Mounting;
            vec![Effect::Mount {
                config_id,
                generation,
            }]
        }
        Completion::Mounted {
            config_id,
            generation,
            result: Ok(()),
        } => {
            if generation != state.generation {
                return Vec::new();
            }
            state.phase = Phase::CheckingStatus;
            vec![Effect::Status {
                config_id,
                generation,
            }]
        }
        Completion::Statused {
            config_id,
            generation,
            result: Ok(mounted),
        } => {
            if generation != state.generation || state.config_id.as_deref() != Some(&config_id) {
                return Vec::new();
            }
            state.phase = Phase::Completed;
            state.mounted = Some(mounted);
            state.error = None;
            Vec::new()
        }
        Completion::Tested {
            result: Ok(false), ..
        } => fail(
            state,
            WorkflowError {
                kind: "failed",
                reason: "network connection test failed".into(),
            },
        ),
        Completion::Created {
            generation,
            result: Err(error),
            ..
        }
        | Completion::Tested {
            generation,
            result: Err(error),
            ..
        }
        | Completion::Mounted {
            generation,
            result: Err(error),
            ..
        }
        | Completion::Statused {
            generation,
            result: Err(error),
            ..
        } if generation == state.generation => fail(state, error),
        _ => Vec::new(),
    }
}

fn fail(state: &mut State, error: WorkflowError) -> Vec<Effect> {
    state.phase = Phase::Failed;
    state.error = Some(error);
    Vec::new()
}

fn backend(
    capabilities: &WorkflowCapabilities,
) -> Result<std::sync::Arc<dyn NetworkDriveBackend>, WorkflowError> {
    capabilities
        .operations
        .registry
        .network_backend(&NetworkBackendId::rclone())
        .map_err(Into::into)
}

pub(crate) async fn execute(capabilities: &WorkflowCapabilities, effect: Effect) -> Completion {
    match effect {
        Effect::Create { config, generation } => {
            let config_id = config.id.clone();
            Completion::Created {
                config_id,
                generation,
                result: match backend(capabilities) {
                    Ok(backend) => backend.create_config(&config).await.map_err(|error| {
                        WorkflowError::from(crate::operations::OperationError::from(error))
                    }),
                    Err(error) => Err(error),
                },
            }
        }
        Effect::Test {
            config_id,
            generation,
        } => Completion::Tested {
            config_id: config_id.clone(),
            generation,
            result: match backend(capabilities) {
                Ok(backend) => backend
                    .test_config(&config_id)
                    .await
                    .map(|result| result.success)
                    .map_err(|error| {
                        WorkflowError::from(crate::operations::OperationError::from(error))
                    }),
                Err(error) => Err(error),
            },
        },
        Effect::Mount {
            config_id,
            generation,
        } => Completion::Mounted {
            config_id: config_id.clone(),
            generation,
            result: match backend(capabilities) {
                Ok(backend) => backend.mount(&config_id).await.map_err(|error| {
                    WorkflowError::from(crate::operations::OperationError::from(error))
                }),
                Err(error) => Err(error),
            },
        },
        Effect::Status {
            config_id,
            generation,
        } => Completion::Statused {
            config_id: config_id.clone(),
            generation,
            result: match backend(capabilities) {
                Ok(backend) => backend
                    .mount_status(&config_id)
                    .await
                    .map(|mount| matches!(mount.status, NetworkDriveStatus::Mounted))
                    .map_err(|error| {
                        WorkflowError::from(crate::operations::OperationError::from(error))
                    }),
                Err(error) => Err(error),
            },
        },
    }
}

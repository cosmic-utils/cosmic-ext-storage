use storage_types::{
    ImageAssetRef, ImageCopyKind, ImageCopyRequest, UsageDeleteRequest, UsageScanParallelismPreset,
    UsageWorkflowRequest, WorkflowState,
};

use super::{WorkflowCapabilities, WorkflowError};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Idle,
    ImageRunning,
    ImageCancelled,
    ImageCompleted,
    UsageRunning,
    UsageCompleted,
    DeleteCompleted,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageUsageSnapshot {
    pub phase: Phase,
    pub image_operation_id: Option<String>,
    pub image_progress: Option<(u64, u64)>,
    pub usage_scan_id: Option<String>,
    pub usage_progress: Option<(u64, u64)>,
    pub deleted_paths: Vec<String>,
    pub error: Option<WorkflowError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageUsageIntent {
    StartImage {
        asset: ImageAssetRef,
        device: String,
    },
    PollImage,
    CancelImage,
    StartUsage {
        scan_id: String,
        mounts: Vec<String>,
    },
    PollUsage,
    DeleteUsage {
        paths: Vec<String>,
    },
}

#[derive(Debug, Default)]
pub(crate) struct State {
    phase: Phase,
    image_operation_id: Option<String>,
    image_progress: Option<(u64, u64)>,
    usage_scan_id: Option<String>,
    usage_progress: Option<(u64, u64)>,
    deleted_paths: Vec<String>,
    error: Option<WorkflowError>,
    generation: u64,
}

impl State {
    pub(crate) fn snapshot(&self) -> ImageUsageSnapshot {
        ImageUsageSnapshot {
            phase: self.phase.clone(),
            image_operation_id: self.image_operation_id.clone(),
            image_progress: self.image_progress,
            usage_scan_id: self.usage_scan_id.clone(),
            usage_progress: self.usage_progress,
            deleted_paths: self.deleted_paths.clone(),
            error: self.error.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Effect {
    StartImage {
        request: ImageCopyRequest,
        generation: u64,
    },
    PollImage {
        operation_id: String,
        generation: u64,
    },
    CancelImage {
        operation_id: String,
        generation: u64,
    },
    StartUsage {
        request: UsageWorkflowRequest,
        generation: u64,
    },
    PollUsage {
        scan_id: String,
        generation: u64,
    },
    DeleteUsage {
        paths: Vec<String>,
        generation: u64,
    },
}

impl Effect {
    pub(crate) const fn operation(&self) -> &'static str {
        match self {
            Self::StartImage { .. } => "image.start_copy",
            Self::PollImage { .. } => "image.copy_status",
            Self::CancelImage { .. } => "image.cancel_copy",
            Self::StartUsage { .. } => "usage.start_scan",
            Self::PollUsage { .. } => "usage.scan_status",
            Self::DeleteUsage { .. } => "usage.delete_files",
        }
    }

    pub(crate) const fn generation(&self) -> u64 {
        match self {
            Self::StartImage { generation, .. }
            | Self::PollImage { generation, .. }
            | Self::CancelImage { generation, .. }
            | Self::StartUsage { generation, .. }
            | Self::PollUsage { generation, .. }
            | Self::DeleteUsage { generation, .. } => *generation,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Completion {
    ImageStarted {
        generation: u64,
        result: Result<String, WorkflowError>,
    },
    ImagePolled {
        generation: u64,
        result: Result<(WorkflowState, u64, u64), WorkflowError>,
    },
    ImageCancelled {
        generation: u64,
        result: Result<(), WorkflowError>,
    },
    UsageStarted {
        generation: u64,
        result: Result<String, WorkflowError>,
    },
    UsagePolled {
        generation: u64,
        result: Result<(WorkflowState, u64, u64), WorkflowError>,
    },
    UsageDeleted {
        generation: u64,
        result: Result<Vec<String>, WorkflowError>,
    },
}

pub(crate) fn reduce_intent(
    state: &mut State,
    intent: ImageUsageIntent,
    generation: u64,
) -> Vec<Effect> {
    state.error = None;
    state.generation = generation;
    match intent {
        ImageUsageIntent::StartImage { asset, device } => {
            state.phase = Phase::ImageRunning;
            state.image_operation_id = None;
            state.image_progress = Some((0, 0));
            vec![Effect::StartImage {
                request: ImageCopyRequest {
                    kind: ImageCopyKind::Backup,
                    device,
                    asset,
                },
                generation,
            }]
        }
        ImageUsageIntent::PollImage => state
            .image_operation_id
            .clone()
            .map(|operation_id| {
                vec![Effect::PollImage {
                    operation_id,
                    generation,
                }]
            })
            .unwrap_or_default(),
        ImageUsageIntent::CancelImage => state
            .image_operation_id
            .clone()
            .map(|operation_id| {
                vec![Effect::CancelImage {
                    operation_id,
                    generation,
                }]
            })
            .unwrap_or_default(),
        ImageUsageIntent::StartUsage { scan_id, mounts } => {
            if mounts.is_empty() {
                return fail(
                    state,
                    WorkflowError {
                        kind: "invalid_input",
                        reason: "at least one usage mount is required".into(),
                    },
                );
            }
            state.phase = Phase::UsageRunning;
            state.usage_scan_id = None;
            state.usage_progress = Some((0, 0));
            vec![Effect::StartUsage {
                request: UsageWorkflowRequest {
                    scan_id,
                    mounts,
                    top_files_per_category: 10,
                    show_all_files: false,
                    parallelism_preset: UsageScanParallelismPreset::Balanced,
                },
                generation,
            }]
        }
        ImageUsageIntent::PollUsage => state
            .usage_scan_id
            .clone()
            .map(|scan_id| {
                vec![Effect::PollUsage {
                    scan_id,
                    generation,
                }]
            })
            .unwrap_or_default(),
        ImageUsageIntent::DeleteUsage { paths } => {
            if paths.is_empty() {
                return fail(
                    state,
                    WorkflowError {
                        kind: "invalid_input",
                        reason: "at least one usage path is required".into(),
                    },
                );
            }
            vec![Effect::DeleteUsage { paths, generation }]
        }
    }
}

pub(crate) fn reduce_completion(state: &mut State, completion: Completion) {
    let generation = match &completion {
        Completion::ImageStarted { generation, .. }
        | Completion::ImagePolled { generation, .. }
        | Completion::ImageCancelled { generation, .. }
        | Completion::UsageStarted { generation, .. }
        | Completion::UsagePolled { generation, .. }
        | Completion::UsageDeleted { generation, .. } => *generation,
    };
    if generation != state.generation {
        return;
    }
    match completion {
        Completion::ImageStarted {
            result: Ok(operation_id),
            ..
        } => state.image_operation_id = Some(operation_id),
        Completion::ImagePolled {
            result: Ok((workflow, complete, total)),
            ..
        } => {
            state.image_progress = Some((complete, total));
            state.phase = match workflow {
                WorkflowState::Cancelled => Phase::ImageCancelled,
                WorkflowState::Completed => Phase::ImageCompleted,
                WorkflowState::Failed => Phase::Failed,
                WorkflowState::Pending | WorkflowState::Running => Phase::ImageRunning,
            };
        }
        Completion::ImageCancelled { result: Ok(()), .. } => state.phase = Phase::ImageCancelled,
        Completion::UsageStarted {
            result: Ok(scan_id),
            ..
        } => state.usage_scan_id = Some(scan_id),
        Completion::UsagePolled {
            result: Ok((workflow, processed, total)),
            ..
        } => {
            state.usage_progress = Some((processed, total));
            state.phase = match workflow {
                WorkflowState::Completed => Phase::UsageCompleted,
                WorkflowState::Failed => Phase::Failed,
                _ => Phase::UsageRunning,
            };
        }
        Completion::UsageDeleted {
            result: Ok(paths), ..
        } => {
            state.deleted_paths = paths;
            state.phase = Phase::DeleteCompleted;
        }
        Completion::ImageStarted {
            result: Err(error), ..
        }
        | Completion::ImagePolled {
            result: Err(error), ..
        }
        | Completion::ImageCancelled {
            result: Err(error), ..
        }
        | Completion::UsageStarted {
            result: Err(error), ..
        }
        | Completion::UsagePolled {
            result: Err(error), ..
        }
        | Completion::UsageDeleted {
            result: Err(error), ..
        } => {
            state.phase = Phase::Failed;
            state.error = Some(error);
        }
    }
}

fn fail(state: &mut State, error: WorkflowError) -> Vec<Effect> {
    state.phase = Phase::Failed;
    state.error = Some(error);
    Vec::new()
}

pub(crate) async fn execute(capabilities: &WorkflowCapabilities, effect: Effect) -> Completion {
    match effect {
        Effect::StartImage {
            request,
            generation,
        } => Completion::ImageStarted {
            generation,
            result: capabilities
                .operations
                .image_workflows
                .start_image_copy(request)
                .await
                .map_err(|error| {
                    WorkflowError::from(crate::operations::OperationError::from(error))
                }),
        },
        Effect::PollImage {
            operation_id,
            generation,
        } => Completion::ImagePolled {
            generation,
            result: capabilities
                .operations
                .image_workflows
                .image_copy_status(&operation_id)
                .await
                .map(|status| (status.state, status.bytes_completed, status.bytes_total))
                .map_err(|error| {
                    WorkflowError::from(crate::operations::OperationError::from(error))
                }),
        },
        Effect::CancelImage {
            operation_id,
            generation,
        } => Completion::ImageCancelled {
            generation,
            result: capabilities
                .operations
                .image_workflows
                .cancel_image_copy(&operation_id)
                .await
                .map_err(|error| {
                    WorkflowError::from(crate::operations::OperationError::from(error))
                }),
        },
        Effect::StartUsage {
            request,
            generation,
        } => Completion::UsageStarted {
            generation,
            result: capabilities
                .operations
                .usage_operations
                .start_usage_scan(request)
                .await
                .map_err(|error| {
                    WorkflowError::from(crate::operations::OperationError::from(error))
                }),
        },
        Effect::PollUsage {
            scan_id,
            generation,
        } => Completion::UsagePolled {
            generation,
            result: capabilities
                .operations
                .usage_operations
                .usage_scan_status(&scan_id)
                .await
                .map(|status| {
                    (
                        status.state,
                        status.processed_bytes,
                        status.estimated_total_bytes,
                    )
                })
                .map_err(|error| {
                    WorkflowError::from(crate::operations::OperationError::from(error))
                }),
        },
        Effect::DeleteUsage { paths, generation } => Completion::UsageDeleted {
            generation,
            result: capabilities
                .operations
                .usage_operations
                .delete_usage_files(UsageDeleteRequest { paths })
                .await
                .map(|response| response.result.deleted)
                .map_err(|error| {
                    WorkflowError::from(crate::operations::OperationError::from(error))
                }),
        },
    }
}

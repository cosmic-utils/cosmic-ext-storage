//! Stateful logical-topology selection, loading, and action generations.

use storage_contracts::LogicalAction;
use storage_types::{
    LogicalEntity, LogicalEntityId, LogicalEntityKind, LogicalSourceStatus, LogicalTopology,
    ProgressRatio,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogicalDetailTab {
    #[default]
    Overview,
    Members,
    Operations,
    Btrfs,
}

#[derive(Debug, Clone)]
pub struct PendingLogicalAction {
    pub action: LogicalAction,
    pub entity: Option<LogicalEntityId>,
    pub generation: u64,
}

#[derive(Debug, Default)]
pub struct LogicalState {
    /// The user has selected a non-privileged Logical sidebar candidate.
    /// Detailed topology is intentionally loaded only after this point.
    pub view_requested: bool,
    /// Device selected from the non-privileged sidebar tree. This anchors a
    /// subsequent topology load to the logical entity the user opened.
    pub selected_device: Option<String>,
    pub entities: Vec<LogicalEntity>,
    pub selected: Option<LogicalEntityId>,
    pub selected_tab: LogicalDetailTab,
    pub pending: Option<PendingLogicalAction>,
    pub logical_action_generation: u64,
    pub logical_load_generation: u64,
    pub loading: bool,
    pub last_refresh_error: Option<String>,
    pub action_status: Option<String>,
    pub action_progress: Option<ProgressRatio>,
    pub source_statuses: Vec<LogicalSourceStatus>,
}

impl LogicalState {
    pub fn request_view(&mut self, device_path: Option<String>) {
        self.view_requested = true;
        if let Some(device_path) = device_path {
            if self.selected_device.as_deref() != Some(&device_path) {
                self.selected = None;
            }
            self.selected_device = Some(device_path);
        }
    }

    pub fn leave_view(&mut self) {
        self.view_requested = false;
        self.selected_device = None;
        self.selected = None;
    }

    pub fn begin_load(&mut self) -> u64 {
        self.logical_load_generation = self.logical_load_generation.saturating_add(1);
        self.loading = true;
        self.last_refresh_error = None;
        self.logical_load_generation
    }

    pub fn finish_load(
        &mut self,
        generation: u64,
        result: Result<LogicalTopology, String>,
    ) -> bool {
        if generation != self.logical_load_generation {
            return false;
        }
        self.loading = false;
        match result {
            Ok(topology) => {
                let selected = self.selected.clone();
                self.entities = topology.entities;
                self.source_statuses = topology.sources;
                self.selected =
                    selected.filter(|id| self.entities.iter().any(|entity| &entity.id == id));
                if self.selected.is_none() {
                    self.selected = self.selected_device.as_deref().and_then(|device_path| {
                        self.entities
                            .iter()
                            .find(|entity| {
                                entity.kind == LogicalEntityKind::BtrfsFilesystem
                                    && entity.device_path.as_deref() == Some(device_path)
                            })
                            .or_else(|| {
                                self.entities.iter().find(|entity| {
                                    entity.parent_id.is_none()
                                        && entity.members.iter().any(|member| {
                                            member.device_path.as_deref() == Some(device_path)
                                        })
                                })
                            })
                            .map(|entity| entity.id.clone())
                    });
                }
                if self.selected.is_none() {
                    self.selected = self
                        .entities
                        .iter()
                        .find(|entity| entity.parent_id.is_none())
                        .or_else(|| self.entities.first())
                        .map(|entity| entity.id.clone());
                }
                self.last_refresh_error = None;
            }
            Err(error) => self.last_refresh_error = Some(error),
        }
        true
    }

    pub fn select(&mut self, entity: Option<LogicalEntityId>) {
        self.view_requested = true;
        self.selected = entity.filter(|id| self.entities.iter().any(|current| &current.id == id));
    }

    pub fn selected_entity(&self) -> Option<&LogicalEntity> {
        self.selected
            .as_ref()
            .and_then(|id| self.entities.iter().find(|entity| &entity.id == id))
    }

    pub fn begin_action(
        &mut self,
        action: LogicalAction,
        entity: Option<LogicalEntityId>,
    ) -> Result<u64, String> {
        if self.pending.is_some() {
            return Err("Another logical operation is in progress.".into());
        }
        self.logical_action_generation = self.logical_action_generation.saturating_add(1);
        let generation = self.logical_action_generation;
        self.pending = Some(PendingLogicalAction {
            action,
            entity,
            generation,
        });
        self.action_status = Some("Operation in progress".into());
        self.action_progress = None;
        Ok(generation)
    }

    pub fn progress(&mut self, generation: u64, progress: ProgressRatio) -> bool {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.generation == generation)
        {
            self.action_progress = Some(progress);
            true
        } else {
            false
        }
    }

    pub fn finish_action(&mut self, generation: u64, result: Result<(), String>) -> bool {
        if self
            .pending
            .as_ref()
            .is_none_or(|pending| pending.generation != generation)
        {
            return false;
        }
        self.pending = None;
        self.action_progress = None;
        self.action_status = Some(match result {
            Ok(()) => "Operation completed".into(),
            Err(error) => error,
        });
        true
    }
}

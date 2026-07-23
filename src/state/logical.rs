//! Stateful logical-topology selection, loading, and action generations.

use storage_contracts::LogicalAction;
use storage_types::{
    LogicalEntity, LogicalEntityId, LogicalSourceStatus, LogicalTopology, ProgressRatio,
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
                self.last_refresh_error = None;
            }
            Err(error) => self.last_refresh_error = Some(error),
        }
        true
    }

    pub fn select(&mut self, entity: Option<LogicalEntityId>) {
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

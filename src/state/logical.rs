//! Stateful logical-topology selection, loading, and action generations.

use std::collections::BTreeSet;

use storage_contracts::{
    ConfirmedLogicalAction, LogicalAction, LogicalActionKind, LogicalPreflight,
    LogicalPreflightRequestKey,
};
use storage_types::{
    BlockDeviceRef, LogicalCandidateAnchor, LogicalCandidateResolution, LogicalEntity,
    LogicalEntityId, LogicalEntityKind, LogicalLoadResult, LogicalSourceStatus, LogicalTopology,
    ProgressRatio,
};

#[derive(Debug, Clone)]
pub struct PendingLogicalAction {
    pub action: LogicalAction,
    pub entity: Option<LogicalEntityId>,
    pub generation: u64,
}

/// One editable logical operation.  Input changes increment `revision` and
/// invalidate the epoch-bound preflight/confirmation that came before it.
#[derive(Debug, Clone)]
pub struct LogicalActionDraft {
    pub action: LogicalAction,
    pub target: Option<LogicalEntityId>,
    pub revision: u64,
    pub validation_error: Option<String>,
}

/// The root operation represented by a preflighted device picker.  It lives
/// with logical state so state transition tests do not depend on the UI/dialog
/// module that happens to render it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicalDevicePickerAction {
    LvmPhysicalVolume { volume_group: LogicalEntityId },
    MdRaidMember { array: LogicalEntityId },
    BtrfsDevice { filesystem: LogicalEntityId },
}

impl LogicalDevicePickerAction {
    pub fn title(&self) -> &'static str {
        match self {
            Self::LvmPhysicalVolume { .. } => "Add LVM physical volume",
            Self::MdRaidMember { .. } => "Add MD RAID member",
            Self::BtrfsDevice { .. } => "Add Btrfs device",
        }
    }

    pub fn action(&self, device: BlockDeviceRef) -> LogicalAction {
        match self {
            Self::LvmPhysicalVolume { volume_group } => LogicalAction::AddLvmPhysicalVolume {
                volume_group: volume_group.clone(),
                device,
            },
            Self::MdRaidMember { array } => LogicalAction::AddMdRaidMember {
                array: array.clone(),
                device,
            },
            Self::BtrfsDevice { filesystem } => LogicalAction::AddBtrfsDevice {
                filesystem: filesystem.clone(),
                device,
            },
        }
    }
}

#[allow(dead_code)]
pub type RefreshClock = u64;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RefreshDomain {
    Logical,
    Physical,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RefreshCause {
    Manual,
    DeviceEvent,
    ActionSuccess { action_generation: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshRequest {
    pub requested_at: RefreshClock,
    pub must_start_after: Option<RefreshClock>,
    pub causes: BTreeSet<RefreshCause>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshRun {
    pub run_id: u64,
    pub started_at: RefreshClock,
    pub causes: BTreeSet<RefreshCause>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DomainRefreshState {
    pub running: Option<RefreshRun>,
    pub queued: Option<RefreshRequest>,
    pub next_run_id: u64,
}

/// The single authority for refresh run allocation.  A successful action
/// creates a post-success barrier in both domains, so an older in-flight load
/// cannot accidentally satisfy the refresh that observes the action.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RefreshCoordinator {
    pub clock: RefreshClock,
    pub logical: DomainRefreshState,
    pub physical: DomainRefreshState,
}

#[allow(dead_code)]
impl RefreshCoordinator {
    pub fn request(&mut self, domain: RefreshDomain, cause: RefreshCause) -> Option<RefreshRun> {
        let requested_at = self.tick();
        self.enqueue(
            domain,
            RefreshRequest {
                requested_at,
                must_start_after: None,
                causes: BTreeSet::from([cause]),
            },
        )
    }

    pub fn action_succeeded(&mut self, action_generation: u64) -> [Option<RefreshRun>; 2] {
        let barrier = self.tick();
        let mut request = |domain: RefreshDomain| {
            self.enqueue(
                domain,
                RefreshRequest {
                    requested_at: barrier,
                    must_start_after: Some(barrier),
                    causes: BTreeSet::from([RefreshCause::ActionSuccess { action_generation }]),
                },
            )
        };
        [
            request(RefreshDomain::Logical),
            request(RefreshDomain::Physical),
        ]
    }

    pub fn start_next(&mut self, domain: RefreshDomain) -> Option<RefreshRun> {
        if self.domain(domain).running.is_some() {
            return None;
        }
        let request = self.domain_mut(domain).queued.take()?;
        let started_at = self.tick();
        debug_assert!(
            request
                .must_start_after
                .is_none_or(|barrier| started_at > barrier)
        );
        let state = self.domain_mut(domain);
        state.next_run_id = state.next_run_id.saturating_add(1);
        let run = RefreshRun {
            run_id: state.next_run_id,
            started_at,
            causes: request.causes,
        };
        state.running = Some(run.clone());
        Some(run)
    }

    pub fn complete(&mut self, domain: RefreshDomain, run_id: u64) -> Option<RefreshRun> {
        if self.domain(domain).running.as_ref().map(|run| run.run_id) != Some(run_id) {
            return None;
        }
        self.domain_mut(domain).running = None;
        self.start_next(domain)
    }

    fn enqueue(&mut self, domain: RefreshDomain, request: RefreshRequest) -> Option<RefreshRun> {
        let can_join_running = {
            let state = self.domain(domain);
            state.queued.is_none()
                && state.running.as_ref().is_some_and(|run| {
                    request
                        .must_start_after
                        .is_none_or(|barrier| run.started_at > barrier)
                })
        };
        if can_join_running {
            self.domain_mut(domain)
                .running
                .as_mut()
                .expect("the checked running refresh still exists")
                .causes
                .extend(request.causes);
            return None;
        }

        let state = self.domain_mut(domain);
        match &mut state.queued {
            Some(queued) => {
                queued.requested_at = queued.requested_at.min(request.requested_at);
                queued.must_start_after = match (queued.must_start_after, request.must_start_after)
                {
                    (Some(left), Some(right)) => Some(left.max(right)),
                    (left @ Some(_), None) | (None, left @ Some(_)) => left,
                    (None, None) => None,
                };
                queued.causes.extend(request.causes);
            }
            None => state.queued = Some(request),
        }
        self.start_next(domain)
    }

    fn domain(&self, domain: RefreshDomain) -> &DomainRefreshState {
        match domain {
            RefreshDomain::Logical => &self.logical,
            RefreshDomain::Physical => &self.physical,
        }
    }

    fn domain_mut(&mut self, domain: RefreshDomain) -> &mut DomainRefreshState {
        match domain {
            RefreshDomain::Logical => &mut self.logical,
            RefreshDomain::Physical => &mut self.physical,
        }
    }

    fn tick(&mut self) -> RefreshClock {
        self.clock = self.clock.saturating_add(1);
        self.clock
    }
}

#[derive(Debug, Default)]
pub struct LogicalState {
    /// The user has selected a non-privileged Logical sidebar candidate.
    /// Detailed topology is intentionally loaded only after this point.
    pub view_requested: bool,
    /// Device selected from the non-privileged sidebar tree. This anchors a
    /// subsequent topology load to the logical entity the user opened.
    pub selected_device: Option<String>,
    /// Typed navigation identity.  The display path above remains only while
    /// old sidebar routing is migrated; it is never used to form an action.
    pub selected_candidate: Option<LogicalCandidateAnchor>,
    pub candidate_resolution: Option<LogicalCandidateResolution>,
    pub entities: Vec<LogicalEntity>,
    pub selected: Option<LogicalEntityId>,
    pub draft: Option<LogicalActionDraft>,
    pub device_picker: Option<LogicalDevicePickerAction>,
    pub draft_revision: u64,
    pub preflight_request: Option<LogicalPreflightRequestKey>,
    pub preflight: Option<LogicalPreflight>,
    pub confirmation: Option<ConfirmedLogicalAction>,
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

    #[allow(dead_code)]
    pub fn request_candidate(&mut self, anchor: Option<LogicalCandidateAnchor>) {
        self.view_requested = true;
        if self.selected_candidate != anchor {
            self.selected = None;
            self.invalidate_draft();
        }
        self.selected_device = anchor.as_ref().map(|anchor| anchor.display_path.clone());
        self.selected_candidate = anchor;
    }

    pub fn leave_view(&mut self) {
        self.view_requested = false;
        self.selected_device = None;
        self.selected_candidate = None;
        self.candidate_resolution = None;
        self.selected = None;
        self.invalidate_draft();
    }

    pub fn begin_load(&mut self) -> u64 {
        self.logical_load_generation = self.logical_load_generation.saturating_add(1);
        self.preflight_request = None;
        self.preflight = None;
        self.confirmation = None;
        self.loading = true;
        self.last_refresh_error = None;
        self.logical_load_generation
    }

    #[allow(dead_code)]
    pub fn finish_load_result(
        &mut self,
        generation: u64,
        result: Result<LogicalLoadResult, String>,
    ) -> bool {
        match result {
            Ok(result) => {
                self.candidate_resolution = Some(result.candidate_resolution.clone());
                let resolution = result.candidate_resolution;
                let accepted = self.finish_load(generation, Ok(result.topology));
                if accepted {
                    if let LogicalCandidateResolution::Resolved { root_id } = resolution {
                        self.selected = self
                            .entities
                            .iter()
                            .any(|entity| entity.id == root_id)
                            .then_some(root_id);
                    }
                    if !matches!(
                        self.candidate_resolution,
                        Some(LogicalCandidateResolution::Resolved { .. })
                    ) && self.selected_candidate.is_some()
                    {
                        self.selected = None;
                    }
                }
                accepted
            }
            Err(error) => self.finish_load(generation, Err(error)),
        }
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
                                    && entity.display_device_path() == Some(device_path)
                            })
                            .or_else(|| {
                                self.entities.iter().find(|entity| {
                                    entity.parent_id.is_none()
                                        && entity.display_device_path() == Some(device_path)
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
        self.invalidate_draft();
        self.selected = entity.filter(|id| self.entities.iter().any(|current| &current.id == id));
    }

    pub fn begin_draft(
        &mut self,
        action: LogicalAction,
        target: Option<LogicalEntityId>,
    ) -> LogicalPreflightRequestKey {
        let revision = self.next_draft_revision();
        let action_kind = action.kind();
        self.draft = Some(LogicalActionDraft {
            action,
            target: target.clone(),
            revision,
            validation_error: None,
        });
        self.device_picker = None;
        self.begin_preflight(target, action_kind, revision)
    }

    pub fn begin_device_picker(
        &mut self,
        picker: LogicalDevicePickerAction,
    ) -> LogicalPreflightRequestKey {
        self.invalidate_draft();
        self.device_picker = Some(picker.clone());
        let revision = self.next_draft_revision();
        let (target, action_kind) = match picker {
            LogicalDevicePickerAction::LvmPhysicalVolume { volume_group } => {
                (volume_group, LogicalActionKind::AddLvmPhysicalVolume)
            }
            LogicalDevicePickerAction::MdRaidMember { array } => {
                (array, LogicalActionKind::AddMdRaidMember)
            }
            LogicalDevicePickerAction::BtrfsDevice { filesystem } => {
                (filesystem, LogicalActionKind::AddBtrfsDevice)
            }
        };
        self.begin_preflight(Some(target), action_kind, revision)
    }

    pub fn accept_preflight(&mut self, preflight: LogicalPreflight) -> bool {
        if self.preflight_request.as_ref() != Some(&preflight.key.request_key) {
            return false;
        }
        self.preflight = Some(preflight);
        true
    }

    pub fn confirm_draft(&mut self) -> Result<ConfirmedLogicalAction, String> {
        let Some(draft) = &self.draft else {
            return Err("No logical action is being edited.".into());
        };
        if let Some(error) = &draft.validation_error {
            return Err(error.clone());
        }
        let Some(preflight) = &self.preflight else {
            return Err("Wait for the current logical action review.".into());
        };
        if preflight.key.request_key
            != self
                .preflight_request
                .clone()
                .ok_or_else(|| "The logical action review expired.".to_string())?
        {
            return Err("The logical action review is stale; retry it.".into());
        }
        if preflight.key.request_key.draft_revision != draft.revision {
            return Err("The logical action draft changed; review it again.".into());
        }
        let request_target = match &preflight.key.request_key.target {
            storage_contracts::LogicalPreflightTarget::Root(target) => Some(target),
            storage_contracts::LogicalPreflightTarget::Landing
            | storage_contracts::LogicalPreflightTarget::Candidate(_) => None,
        };
        if request_target != draft.target.as_ref() {
            return Err("The logical action target changed; review it again.".into());
        }
        if !matches!(
            preflight.availability,
            storage_contracts::LogicalPreflightAvailability::Ready
        ) {
            return Err("This logical action is currently unavailable.".into());
        }
        let confirmed = ConfirmedLogicalAction {
            action: draft.action.clone(),
            preflight_key: preflight.key.clone(),
        };
        self.confirmation = Some(confirmed.clone());
        Ok(confirmed)
    }

    pub fn invalidate_draft(&mut self) {
        self.draft = None;
        self.device_picker = None;
        self.preflight_request = None;
        self.preflight = None;
        self.confirmation = None;
    }

    fn next_draft_revision(&mut self) -> u64 {
        self.draft_revision = self.draft_revision.saturating_add(1);
        self.draft_revision
    }

    fn begin_preflight(
        &mut self,
        target: Option<LogicalEntityId>,
        action_kind: LogicalActionKind,
        revision: u64,
    ) -> LogicalPreflightRequestKey {
        self.preflight = None;
        self.confirmation = None;
        let request = LogicalPreflightRequestKey {
            target: target.map_or(
                storage_contracts::LogicalPreflightTarget::Landing,
                storage_contracts::LogicalPreflightTarget::Root,
            ),
            action_kind,
            logical_load_generation: self.logical_load_generation,
            draft_revision: revision,
        };
        self.preflight_request = Some(request.clone());
        request
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

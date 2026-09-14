//! Application-facing logical topology and action façade.

use std::collections::BTreeMap;

use storage_contracts::{
    ConfirmedLogicalAction, LogicalActionOutcome, LogicalPreflight, LogicalPreflightRequest,
};
use storage_types::{
    BlockDeviceRef, LogicalCandidateAnchor, LogicalCandidateResolution, LogicalCapabilities,
    LogicalEntity, LogicalEntityDetails, LogicalEntityId, LogicalLoadRequest, LogicalLoadResult,
    LogicalSource, LogicalSourceAvailability, LogicalSourceStatus, LogicalTopology,
};

use super::{OperationError, StorageOperations};

impl StorageOperations {
    pub async fn capture_logical_candidate(
        &self,
        display_path: String,
    ) -> Result<LogicalCandidateAnchor, OperationError> {
        self.registry
            .logical_operations
            .capture_logical_candidate(display_path)
            .await
            .map_err(Into::into)
    }

    /// Loads all registered sources in declared authority order and merges them
    /// without granting a local-tools entity mutation authority.
    pub async fn load_logical_topology(&self) -> Result<LogicalTopology, OperationError> {
        let expected = [LogicalSource::Udisks, LogicalSource::LocalTools];
        let actual: Vec<_> = self
            .registry
            .logical_topology_sources
            .iter()
            .map(|source| source.logical_source())
            .collect();
        if actual != expected {
            return Err(OperationError::Failed(
                "Logical topology sources were registered in an invalid order.".into(),
            ));
        }

        let mut results = Vec::new();
        let mut errors = Vec::new();
        for source in &self.registry.logical_topology_sources {
            let declared = source.logical_availability();
            match source.list_logical_entities().await {
                Ok(entities) => results.push((source.logical_source(), declared, entities)),
                Err(error) => {
                    errors.push(error.clone());
                    results.push((
                        source.logical_source(),
                        LogicalSourceAvailability::Failed {
                            reason: error.message,
                        },
                        Vec::new(),
                    ));
                }
            }
        }
        match merge_logical_sources(results) {
            Ok(topology) => Ok(topology),
            Err(_) => Err(errors
                .into_iter()
                .next()
                .map(OperationError::from)
                .unwrap_or_else(|| {
                    OperationError::Unavailable("No logical topology source is available.".into())
                })),
        }
    }

    /// Loads the shared topology and resolves an optional sidebar anchor from
    /// the same typed result.  The path stored for messaging is never used to
    /// select a root.
    pub async fn load_logical_topology_for(
        &self,
        request: LogicalLoadRequest,
    ) -> Result<LogicalLoadResult, OperationError> {
        let topology = self.load_logical_topology().await?;
        let candidate_resolution = request
            .anchor
            .as_ref()
            .map(|anchor| resolve_candidate(&topology, anchor))
            .unwrap_or(LogicalCandidateResolution::Missing);
        Ok(LogicalLoadResult {
            topology,
            candidate_resolution,
        })
    }

    pub async fn execute_logical_action(
        &self,
        confirmed: ConfirmedLogicalAction,
    ) -> Result<LogicalActionOutcome, OperationError> {
        self.registry
            .logical_operations
            .execute_logical_action(confirmed)
            .await
            .map_err(Into::into)
    }

    pub async fn preflight_logical_action(
        &self,
        request: LogicalPreflightRequest,
    ) -> Result<LogicalPreflight, OperationError> {
        self.registry
            .logical_operations
            .preflight_logical_action(request)
            .await
            .map_err(Into::into)
    }
}

fn resolve_candidate(
    topology: &LogicalTopology,
    anchor: &LogicalCandidateAnchor,
) -> LogicalCandidateResolution {
    let resolved = topology.entities.iter().find_map(|entity| {
        let matches = match &entity.details {
            LogicalEntityDetails::BtrfsFilesystem(details) => details
                .members
                .iter()
                .any(|member| anchor_matches(member.block.as_ref(), anchor)),
            LogicalEntityDetails::LvmVolumeGroup(details) => details
                .physical_volumes
                .iter()
                .any(|member| anchor_matches(member.member.block.as_ref(), anchor)),
            LogicalEntityDetails::MdRaidArray(details) => details
                .members
                .iter()
                .any(|member| anchor_matches(member.block.as_ref(), anchor)),
            _ => false,
        };
        matches.then(|| entity.id.clone())
    });
    if let Some(root_id) = resolved {
        return LogicalCandidateResolution::Resolved { root_id };
    }
    if anchor.fingerprint.is_none() {
        return LogicalCandidateResolution::Unavailable {
            source: "UDisks".into(),
            reason: "The selected device has no strong identity after topology refresh.".into(),
        };
    }
    if let Some(reason) = topology
        .sources
        .iter()
        .find(|status| status.source == LogicalSource::Udisks)
        .and_then(|status| status.availability.reason())
    {
        return LogicalCandidateResolution::Unavailable {
            source: "UDisks".into(),
            reason: reason.into(),
        };
    }
    LogicalCandidateResolution::Missing
}

fn anchor_matches(reference: Option<&BlockDeviceRef>, anchor: &LogicalCandidateAnchor) -> bool {
    let Some(reference) = reference else {
        return false;
    };
    reference.id == anchor.block_id
        && anchor
            .fingerprint
            .as_ref()
            .is_some_and(|fingerprint| fingerprint == &reference.fingerprint)
}

pub(crate) fn merge_logical_sources(
    results: Vec<(LogicalSource, LogicalSourceAvailability, Vec<LogicalEntity>)>,
) -> Result<LogicalTopology, OperationError> {
    let mut sources = Vec::new();
    let mut by_id: BTreeMap<LogicalEntityId, (LogicalSource, LogicalEntity)> = BTreeMap::new();
    let udisks_failure = results.iter().find_map(|(source, availability, _)| {
        (*source == LogicalSource::Udisks)
            .then_some(availability)
            .and_then(LogicalSourceAvailability::reason)
            .map(str::to_string)
    });
    for (source, availability, entities) in results {
        sources.push(LogicalSourceStatus {
            source,
            availability,
        });
        for entity in entities {
            match by_id.get_mut(&entity.id) {
                Some((existing_source, existing)) if *existing_source == LogicalSource::Udisks => {
                    merge_local_display_fields(existing, entity);
                }
                Some(_) if source == LogicalSource::Udisks => {
                    let id = entity.id.clone();
                    by_id.insert(id, (source, entity));
                }
                Some(_) => {}
                None => {
                    let id = entity.id.clone();
                    by_id.insert(id, (source, entity));
                }
            }
        }
    }
    if by_id.is_empty()
        && sources
            .iter()
            .all(|status| !matches!(status.availability, LogicalSourceAvailability::Available))
    {
        return Err(OperationError::Unavailable(
            "No logical topology source supplied entities.".into(),
        ));
    }
    let mut entities: Vec<_> = by_id
        .into_values()
        .map(|(source, mut entity)| {
            if source == LogicalSource::LocalTools {
                entity.capabilities = LogicalCapabilities::block_all("Not discovered by UDisks");
            } else if let Some(reason) = &udisks_failure {
                entity.capabilities = LogicalCapabilities::block_all(reason.clone());
            }
            entity
        })
        .collect();
    entities.sort_by(|left, right| {
        left.display_name()
            .cmp(right.display_name())
            .then(left.id.cmp(&right.id))
    });
    LogicalTopology::new(entities, sources)
        .map_err(|error| OperationError::Failed(error.to_string()))
}

fn merge_local_display_fields(udisks: &mut LogicalEntity, local: LogicalEntity) {
    match (&mut udisks.details, local.details) {
        (
            storage_types::LogicalEntityDetails::LvmVolumeGroup(udisks),
            storage_types::LogicalEntityDetails::LvmVolumeGroup(local),
        ) => {
            merge_display(&mut udisks.uuid, local.uuid);
            merge_display(&mut udisks.size, local.size);
            merge_display(&mut udisks.used, local.used);
            merge_display(&mut udisks.free, local.free);
        }
        (
            storage_types::LogicalEntityDetails::LvmLogicalVolume(udisks),
            storage_types::LogicalEntityDetails::LvmLogicalVolume(local),
        ) => {
            merge_display(&mut udisks.device_path, local.device_path);
            merge_display(&mut udisks.size, local.size);
            merge_display(&mut udisks.activation, local.activation);
        }
        _ => {}
    }
}

fn merge_display<T>(
    target: &mut storage_types::LogicalDisplay<T>,
    fallback: storage_types::LogicalDisplay<T>,
) {
    if matches!(target, storage_types::LogicalDisplay::Unknown { .. })
        && matches!(fallback, storage_types::LogicalDisplay::Known(_))
    {
        *target = fallback;
    }
}

//! Application-facing logical topology and action façade.

use std::collections::BTreeMap;

use storage_contracts::{LogicalAction, LogicalActionOutcome};
use storage_types::{
    LogicalCapabilities, LogicalEntity, LogicalEntityId, LogicalSource, LogicalSourceAvailability,
    LogicalSourceStatus, LogicalTopology,
};

use super::{OperationError, StorageOperations};

impl StorageOperations {
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

    pub async fn execute_logical_action(
        &self,
        action: LogicalAction,
    ) -> Result<LogicalActionOutcome, OperationError> {
        self.registry
            .logical_operations
            .execute_logical_action(action)
            .await
            .map_err(Into::into)
    }
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
    entities.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
    LogicalTopology::new(entities, sources)
        .map_err(|error| OperationError::Failed(error.to_string()))
}

fn merge_local_display_fields(udisks: &mut LogicalEntity, local: LogicalEntity) {
    if udisks.uuid.is_none() {
        udisks.uuid = local.uuid;
    }
    if udisks.device_path.is_none() {
        udisks.device_path = local.device_path;
    }
    if udisks.used_bytes.is_none() {
        udisks.used_bytes = local.used_bytes;
    }
    if udisks.free_bytes.is_none() {
        udisks.free_bytes = local.free_bytes;
    }
    if udisks.health_status.is_none() {
        udisks.health_status = local.health_status;
    }
    if udisks.progress_fraction.is_none() {
        udisks.progress_fraction = local.progress_fraction;
    }
    for (key, value) in local.metadata {
        udisks.metadata.entry(key).or_insert(value);
    }
    for member in local.members {
        if !udisks.members.iter().any(|current| current.id == member.id) {
            udisks.members.push(member);
        }
    }
}

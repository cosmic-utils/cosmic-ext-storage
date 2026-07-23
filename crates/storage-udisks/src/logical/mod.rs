//! UDisks-native logical storage discovery and mutations.

mod discover;
mod error;
mod operations;
mod proxy;
mod resolve;

use async_trait::async_trait;
use storage_contracts::{LogicalTopologySource, StorageError};
use storage_types::{LogicalEntity, LogicalSource, LogicalSourceAvailability};

use crate::UdisksBackend;

#[async_trait]
impl LogicalTopologySource for UdisksBackend {
    fn logical_source(&self) -> LogicalSource {
        LogicalSource::Udisks
    }

    fn logical_availability(&self) -> LogicalSourceAvailability {
        LogicalSourceAvailability::Available
    }

    async fn list_logical_entities(&self) -> Result<Vec<LogicalEntity>, StorageError> {
        discover::entities(self.manager(), self.logical_epoch()).await
    }
}

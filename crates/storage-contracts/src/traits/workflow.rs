// SPDX-License-Identifier: GPL-3.0-only

//! Contract seams for workflows that used to live directly in UI operations.

use std::sync::Arc;

use async_trait::async_trait;
use storage_types::{
    DesktopImageSelection, FilesystemToolInfo, ImageAssetRef, ImageAttachment,
    ImageAttachmentRequest, ImageCopyRequest, ImageWorkflowStatus, UsageDeleteRequest,
    UsageDeleteResponse, UsageWorkflowRequest, UsageWorkflowStatus,
};

use crate::{StorageError, StorageErrorKind};

#[async_trait]
pub trait FilesystemToolDiscovery: Send + Sync {
    async fn list_filesystem_tools(&self) -> Result<Vec<FilesystemToolInfo>, StorageError>;
}

#[async_trait]
pub trait UsageOperations: Send + Sync {
    async fn list_usage_mounts(&self) -> Result<Vec<String>, StorageError>;
    async fn authorize_show_all_files(&self) -> Result<bool, StorageError>;
    async fn start_usage_scan(&self, request: UsageWorkflowRequest)
    -> Result<String, StorageError>;
    async fn usage_scan_status(&self, scan_id: &str) -> Result<UsageWorkflowStatus, StorageError>;
    async fn wait_for_usage_scan(&self, scan_id: &str)
    -> Result<UsageWorkflowStatus, StorageError>;
    async fn delete_usage_files(
        &self,
        request: UsageDeleteRequest,
    ) -> Result<UsageDeleteResponse, StorageError>;
}

#[async_trait]
pub trait ImageWorkflowOperations: Send + Sync {
    async fn create_image_asset(
        &self,
        asset: ImageAssetRef,
        size_bytes: u64,
    ) -> Result<(), StorageError>;
    async fn attach_image(
        &self,
        request: ImageAttachmentRequest,
    ) -> Result<ImageAttachment, StorageError>;
    async fn start_image_copy(&self, request: ImageCopyRequest) -> Result<String, StorageError>;
    async fn image_copy_status(
        &self,
        operation_id: &str,
    ) -> Result<ImageWorkflowStatus, StorageError>;
    async fn wait_for_image_copy(
        &self,
        operation_id: &str,
    ) -> Result<ImageWorkflowStatus, StorageError>;
    async fn cancel_image_copy(&self, operation_id: &str) -> Result<(), StorageError>;
    async fn forget_image_copy(&self, operation_id: &str) -> Result<(), StorageError>;
}

#[async_trait]
pub trait DesktopServices: Send + Sync {
    async fn select_image(&self) -> Result<DesktopImageSelection, StorageError>;
    async fn reveal(&self, display_reference: &str) -> Result<(), StorageError>;
    async fn open_url(&self, url: &str) -> Result<(), StorageError>;
}

/// Scenario-only semantic controls.  There is deliberately no generic
/// command or arbitrary file-system API here.
#[async_trait]
pub trait ScenarioControl: Send + Sync {
    async fn advance_to(&self, tick: u64) -> Result<ScenarioReceipt, StorageError>;
    async fn reload_overlay(&self) -> Result<ScenarioReload, StorageError>;
    async fn diagnostics(&self) -> Result<ScenarioDiagnostics, StorageError>;
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScenarioReceipt {
    pub sequence: u64,
    pub generation: u64,
    pub virtual_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioReload {
    Applied(ScenarioReceipt),
    Rejected {
        reason: String,
        receipt: ScenarioReceipt,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScenarioDiagnostics {
    pub fixture_name: String,
    pub fixture_sha256: String,
    pub generation: u64,
    pub last_sequence: u64,
    pub virtual_tick: u64,
}

/// The selected adapter graph.  Keeping all composition data in one explicit
/// value prevents a scenario launch from accidentally constructing a second,
/// real backend for a side workflow.
pub struct RuntimeAdapters {
    pub block: Arc<dyn crate::BlockStorageBackend>,
    pub btrfs: Option<Arc<dyn crate::BtrfsBackend>>,
    pub network: Vec<Arc<dyn crate::NetworkDriveBackend>>,
    pub network_availability: Vec<(
        storage_types::NetworkBackendId,
        storage_types::NetworkBackendAvailability,
    )>,
    pub logical_topology_sources: Vec<Arc<dyn crate::LogicalTopologySource>>,
    pub logical_operations: Arc<dyn crate::LogicalOperations>,
    pub filesystem_tools: Arc<dyn FilesystemToolDiscovery>,
    pub usage: Arc<dyn UsageOperations>,
    pub image: Arc<dyn ImageWorkflowOperations>,
    pub desktop: Arc<dyn DesktopServices>,
    pub scenario_control: Option<Arc<dyn ScenarioControl>>,
}

impl RuntimeAdapters {
    pub fn validate(&self) -> Result<(), StorageError> {
        let actual_sources = self
            .logical_topology_sources
            .iter()
            .map(|source| source.logical_source())
            .collect::<Vec<_>>();
        Self::validate_registration(
            actual_sources,
            self.network.iter().map(|backend| backend.id()),
        )
    }

    pub fn validate_registration(
        sources: Vec<storage_types::LogicalSource>,
        network_ids: impl IntoIterator<Item = storage_types::NetworkBackendId>,
    ) -> Result<(), StorageError> {
        if sources
            != [
                storage_types::LogicalSource::Udisks,
                storage_types::LogicalSource::LocalTools,
            ]
        {
            return Err(StorageError::new(
                StorageErrorKind::InvalidInput,
                "runtime adapters must register logical sources in [Udisks, LocalTools] order",
            ));
        }
        let mut unique_ids = std::collections::BTreeSet::new();
        for id in network_ids {
            if !unique_ids.insert(id) {
                return Err(StorageError::new(
                    StorageErrorKind::InvalidInput,
                    "runtime adapters contain a duplicate network backend ID",
                ));
            }
        }
        Ok(())
    }
}

// SPDX-License-Identifier: GPL-3.0-only

//! Typed, in-process storage operations.
//!
//! Application code uses this module rather than the removed project D-Bus
//! service.  Concrete adapters are constructed here at the composition root;
//! the remaining operation modules consume only `storage-contracts` traits.

use std::{collections::BTreeMap, sync::Arc};

#[cfg(feature = "test-backend")]
use std::{cell::Cell, marker::PhantomData, rc::Rc};

use storage_contracts::{
    BlockStorageBackend, BtrfsBackend, FilesystemToolDiscovery, ImageWorkflowOperations,
    LogicalOperations, LogicalTopologySource, NetworkDriveBackend, RuntimeAdapters,
    UsageOperations,
};
use storage_types::{NetworkBackendAvailability, NetworkBackendId};
use tokio::sync::OnceCell;

pub mod btrfs;
pub mod disks;
pub mod error;
pub mod filesystems;
pub mod image;
pub mod logical;
pub mod luks;
pub mod partitions;
pub mod protected_paths;
pub mod rclone;

pub use btrfs::BtrfsClient;
pub use disks::DisksClient;
pub use error::OperationError;
pub use filesystems::FilesystemsClient;
pub use image::ImageClient;
pub use luks::LuksClient;
pub use partitions::PartitionsClient;
pub use rclone::RcloneClient;

/// Backend registrations available to the application.  The registry owns no
/// UI state, allowing contract-backed operations to be tested with mocks.
pub struct BackendRegistry {
    pub block: Arc<dyn BlockStorageBackend>,
    pub btrfs: Option<Arc<dyn BtrfsBackend>>,
    pub network: BTreeMap<NetworkBackendId, Arc<dyn NetworkDriveBackend>>,
    pub network_availability: BTreeMap<NetworkBackendId, NetworkBackendAvailability>,
    pub logical_topology_sources: Vec<Arc<dyn LogicalTopologySource>>,
    pub logical_operations: Arc<dyn LogicalOperations>,
}

impl BackendRegistry {
    pub fn network_backend(
        &self,
        id: &NetworkBackendId,
    ) -> Result<Arc<dyn NetworkDriveBackend>, OperationError> {
        self.network.get(id).cloned().ok_or_else(|| {
            let reason = match self.network_availability.get(id) {
                Some(NetworkBackendAvailability::Unavailable { reason }) => reason.clone(),
                _ => format!("Network backend '{id}' is not available"),
            };
            OperationError::Unavailable(reason)
        })
    }
}

/// Shared operation context constructed once by the application composition
/// root.  It deliberately exposes contracts, never UDisks2/rclone internals.
pub struct StorageOperations {
    pub registry: BackendRegistry,
    pub filesystem_tools: Vec<storage_types::FilesystemToolInfo>,
    pub filesystem_tool_discovery: Arc<dyn FilesystemToolDiscovery>,
    pub usage_operations: Arc<dyn UsageOperations>,
    pub image_workflows: Arc<dyn ImageWorkflowOperations>,
    pub image_manager: image::ImageOperationManager,
}

impl std::fmt::Debug for StorageOperations {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StorageOperations")
            .finish_non_exhaustive()
    }
}

impl StorageOperations {
    pub async fn new() -> Result<Arc<Self>, OperationError> {
        let udisks = Arc::new(storage_udisks::UdisksBackend::new().await?);
        if let Err(error) = udisks.enable_optional_modules().await {
            // Module availability is discovered per feature below.  Failing to
            // load an optional plugin must not prevent ordinary disks from
            // opening, nor trigger a direct-command fallback.
            tracing::warn!(%error, "UDisks optional modules are unavailable");
        }
        let block = udisks.clone() as Arc<dyn BlockStorageBackend>;
        let logical_topology_sources: Vec<Arc<dyn LogicalTopologySource>> = vec![
            udisks.clone() as Arc<dyn LogicalTopologySource>,
            Arc::new(storage_sys::LocalLogicalTopologySource::new())
                as Arc<dyn LogicalTopologySource>,
        ];
        let logical_operations = udisks as Arc<dyn LogicalOperations>;
        let btrfs = Some(Arc::new(disks_btrfs::BtrfsUtilBackend::new()) as Arc<dyn BtrfsBackend>);

        let mut network = BTreeMap::new();
        let mut network_availability = BTreeMap::new();
        let rclone_id = NetworkBackendId::rclone();
        match storage_sys::RcloneNetworkBackend::new() {
            Ok(adapter) => {
                network.insert(
                    rclone_id.clone(),
                    Arc::new(adapter) as Arc<dyn NetworkDriveBackend>,
                );
                network_availability.insert(rclone_id, NetworkBackendAvailability::Available);
            }
            Err(error) => {
                network_availability.insert(
                    rclone_id,
                    NetworkBackendAvailability::unavailable(error.to_string()),
                );
            }
        }

        Ok(Arc::new(Self {
            registry: BackendRegistry {
                block,
                btrfs,
                network,
                network_availability,
                logical_topology_sources,
                logical_operations,
            },
            filesystem_tools: filesystems::detect_filesystem_tools(),
            filesystem_tool_discovery: Arc::new(StaticFilesystemTools(
                filesystems::detect_filesystem_tools(),
            )),
            usage_operations: Arc::new(filesystems::ProductionUsageOperations::default()),
            image_workflows: Arc::new(crate::runtime::UnavailableWorkflowAdapter),
            image_manager: image::ImageOperationManager::default(),
        }))
    }

    pub fn from_adapters(adapters: RuntimeAdapters) -> Result<Arc<Self>, OperationError> {
        adapters.validate()?;
        let mut network = BTreeMap::new();
        for backend in adapters.network {
            network.insert(backend.id(), backend);
        }
        let network_availability = adapters.network_availability.into_iter().collect();
        Ok(Arc::new(Self {
            registry: BackendRegistry {
                block: adapters.block,
                btrfs: adapters.btrfs,
                network,
                network_availability,
                logical_topology_sources: adapters.logical_topology_sources,
                logical_operations: adapters.logical_operations,
            },
            filesystem_tools: Vec::new(),
            filesystem_tool_discovery: adapters.filesystem_tools,
            usage_operations: adapters.usage,
            image_workflows: adapters.image,
            image_manager: image::ImageOperationManager::default(),
        }))
    }
}

struct StaticFilesystemTools(Vec<storage_types::FilesystemToolInfo>);

#[async_trait::async_trait]
impl FilesystemToolDiscovery for StaticFilesystemTools {
    async fn list_filesystem_tools(
        &self,
    ) -> Result<Vec<storage_types::FilesystemToolInfo>, storage_contracts::StorageError> {
        Ok(self.0.clone())
    }
}

static SHARED_OPERATIONS: OnceCell<Arc<StorageOperations>> = OnceCell::const_new();

#[cfg(feature = "test-backend")]
thread_local! {
    static REJECT_SHARED_OPERATIONS: Cell<u32> = const { Cell::new(0) };
}

/// Test-only migration detector for application-workflow tests.
///
/// This does not replace or mutate the selected production context. It simply
/// makes an accidental compatibility lookup fail on the current test thread.
#[cfg(feature = "test-backend")]
pub(crate) fn reject_global_operations_for_workflow_tests() -> GlobalOperationsGuard {
    REJECT_SHARED_OPERATIONS.with(|depth| depth.set(depth.get().saturating_add(1)));
    GlobalOperationsGuard(PhantomData)
}

#[cfg(feature = "test-backend")]
pub(crate) struct GlobalOperationsGuard(PhantomData<Rc<()>>);

#[cfg(feature = "test-backend")]
impl Drop for GlobalOperationsGuard {
    fn drop(&mut self) {
        REJECT_SHARED_OPERATIONS.with(|depth| {
            let current = depth.get();
            assert!(current > 0, "workflow global-operations guard underflow");
            depth.set(current - 1);
        });
    }
}

pub fn install_selected(operations: Arc<StorageOperations>) -> Result<(), OperationError> {
    SHARED_OPERATIONS.set(operations).map_err(|_| {
        OperationError::Failed("a storage runtime is already installed for this process".into())
    })
}

/// Compatibility access for task code that has not yet been converted to carry
/// `Arc<StorageOperations>`. It intentionally never constructs adapters: the
/// composition root must install either the production or scenario graph
/// before any task runs. This prevents a scenario task from falling back to
/// host-backed operations.
pub async fn shared() -> Result<Arc<StorageOperations>, OperationError> {
    #[cfg(feature = "test-backend")]
    if REJECT_SHARED_OPERATIONS.with(|depth| depth.get() > 0) {
        return Err(OperationError::Failed(
            "workflow test attempted global operations context".into(),
        ));
    }
    SHARED_OPERATIONS.get().cloned().ok_or_else(|| {
        OperationError::Failed("storage runtime was not installed by the composition root".into())
    })
}

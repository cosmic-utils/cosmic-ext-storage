// SPDX-License-Identifier: GPL-3.0-only

//! Typed, in-process storage operations.
//!
//! Application code uses this module rather than the removed project D-Bus
//! service.  Concrete adapters are constructed here at the composition root;
//! the remaining operation modules consume only `storage-contracts` traits.

use std::{collections::BTreeMap, sync::Arc};

use storage_contracts::{BlockStorageBackend, BtrfsBackend, NetworkDriveBackend};
use storage_types::{NetworkBackendAvailability, NetworkBackendId};
use tokio::sync::OnceCell;

pub mod btrfs;
pub mod disks;
pub mod error;
pub mod filesystems;
pub mod image;
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
        let block =
            Arc::new(storage_udisks::UdisksBackend::new().await?) as Arc<dyn BlockStorageBackend>;
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
            },
            filesystem_tools: filesystems::detect_filesystem_tools(),
            image_manager: image::ImageOperationManager::default(),
        }))
    }
}

static SHARED_OPERATIONS: OnceCell<Arc<StorageOperations>> = OnceCell::const_new();

/// Compatibility access for existing task code while the task graph is being
/// converted to carry `Arc<StorageOperations>`.  The cell still guarantees one
/// UDisks adapter/connection for the process.
pub async fn shared() -> Result<Arc<StorageOperations>, OperationError> {
    SHARED_OPERATIONS
        .get_or_try_init(StorageOperations::new)
        .await
        .cloned()
}

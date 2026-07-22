// SPDX-License-Identifier: GPL-3.0-only

use async_trait::async_trait;
use storage_types::{
    NetworkBackendId, NetworkDriveCapabilities, NetworkDriveConfig,
    NetworkDriveConfigurationSchema, NetworkDriveList, NetworkDriveMount, NetworkDriveTestResult,
};

use crate::StorageError;

#[async_trait]
pub trait NetworkDriveBackend: Send + Sync {
    fn id(&self) -> NetworkBackendId;
    fn capabilities(&self) -> NetworkDriveCapabilities;
    fn configuration_schema(&self) -> NetworkDriveConfigurationSchema;
    async fn list_configs(&self) -> Result<NetworkDriveList, StorageError>;
    async fn create_config(&self, config: &NetworkDriveConfig) -> Result<(), StorageError>;
    async fn update_config(&self, config: &NetworkDriveConfig) -> Result<(), StorageError>;
    async fn delete_config(&self, config_id: &str) -> Result<(), StorageError>;
    async fn test_config(&self, config_id: &str) -> Result<NetworkDriveTestResult, StorageError>;
    async fn mount(&self, config_id: &str) -> Result<(), StorageError>;
    async fn unmount(&self, config_id: &str) -> Result<(), StorageError>;
    async fn mount_status(&self, config_id: &str) -> Result<NetworkDriveMount, StorageError>;
    async fn mount_on_login(&self, config_id: &str) -> Result<bool, StorageError>;
    async fn set_mount_on_login(&self, config_id: &str, enabled: bool) -> Result<(), StorageError>;
}

// SPDX-License-Identifier: GPL-3.0-only

//! Compatibility façade for the current network view.
//!
//! All calls route through the generic `NetworkDriveBackend`; only the legacy
//! view-model conversion remains until the view renders generic schemas.

use std::sync::Arc;

use storage_contracts::NetworkDriveBackend;
use storage_types::{
    NetworkBackendId, NetworkDriveConfig, NetworkDriveStatus,
    rclone::{
        ConfigScope, MountStatus, MountStatusResult, RemoteConfig, RemoteConfigList, TestResult,
    },
};

use super::{OperationError, StorageOperations, shared};

#[derive(Clone, Debug)]
pub struct RcloneClient(Arc<StorageOperations>);

impl RcloneClient {
    pub async fn new() -> Result<Self, OperationError> {
        Ok(Self(shared().await?))
    }
    fn backend(&self) -> Result<Arc<dyn NetworkDriveBackend>, OperationError> {
        self.0.registry.network_backend(&NetworkBackendId::rclone())
    }
    fn user_scope(scope: &str) -> Result<(), OperationError> {
        if scope.eq_ignore_ascii_case("user") {
            Ok(())
        } else {
            Err(OperationError::Unsupported(
                "Only per-user network-drive configurations are supported".into(),
            ))
        }
    }
    fn remote(config: NetworkDriveConfig) -> RemoteConfig {
        RemoteConfig {
            name: config.name,
            remote_type: config.provider_id,
            scope: ConfigScope::User,
            options: config.options.into_iter().collect(),
            has_secrets: config.has_secrets,
        }
    }
    fn config(config: &RemoteConfig) -> Result<NetworkDriveConfig, OperationError> {
        if config.scope != ConfigScope::User {
            return Err(OperationError::Unsupported(
                "Only per-user network-drive configurations are supported".into(),
            ));
        }
        config
            .validate_name()
            .map_err(OperationError::InvalidInput)?;
        Ok(NetworkDriveConfig {
            backend_id: NetworkBackendId::rclone(),
            id: config.name.clone(),
            name: config.name.clone(),
            provider_id: config.remote_type.clone(),
            options: config.options.clone().into_iter().collect(),
            has_secrets: config.has_secrets,
        })
    }
    pub async fn list_remotes(&self) -> Result<RemoteConfigList, OperationError> {
        let list = self.backend()?.list_configs().await?;
        Ok(RemoteConfigList {
            remotes: list.configs.into_iter().map(Self::remote).collect(),
            user_config_path: None,
            system_config_path: None,
        })
    }
    pub async fn test_remote(&self, name: &str, scope: &str) -> Result<TestResult, OperationError> {
        Self::user_scope(scope)?;
        let result = self.backend()?.test_config(name).await?;
        Ok(TestResult {
            success: result.success,
            message: result.message,
            latency_ms: result.latency_ms,
        })
    }
    pub async fn mount(&self, name: &str, scope: &str) -> Result<(), OperationError> {
        Self::user_scope(scope)?;
        self.backend()?.mount(name).await.map_err(Into::into)
    }
    pub async fn unmount(&self, name: &str, scope: &str) -> Result<(), OperationError> {
        Self::user_scope(scope)?;
        self.backend()?.unmount(name).await.map_err(Into::into)
    }
    pub async fn get_mount_status(
        &self,
        name: &str,
        scope: &str,
    ) -> Result<MountStatusResult, OperationError> {
        Self::user_scope(scope)?;
        let mount = self.backend()?.mount_status(name).await?;
        let status = match mount.status {
            NetworkDriveStatus::Unmounted => MountStatus::Unmounted,
            NetworkDriveStatus::Mounting => MountStatus::Mounting,
            NetworkDriveStatus::Mounted => MountStatus::Mounted,
            NetworkDriveStatus::Unmounting => MountStatus::Unmounting,
            NetworkDriveStatus::Error { message } => MountStatus::Error(message),
        };
        Ok(MountStatusResult::new(status, mount.mount_point))
    }
    pub async fn get_mount_on_boot(&self, name: &str, scope: &str) -> Result<bool, OperationError> {
        Self::user_scope(scope)?;
        self.backend()?
            .mount_on_login(name)
            .await
            .map_err(Into::into)
    }
    pub async fn set_mount_on_boot(
        &self,
        name: &str,
        scope: &str,
        enabled: bool,
    ) -> Result<(), OperationError> {
        Self::user_scope(scope)?;
        self.backend()?
            .set_mount_on_login(name, enabled)
            .await
            .map_err(Into::into)
    }
    pub async fn create_remote(&self, config: &RemoteConfig) -> Result<(), OperationError> {
        let config = Self::config(config)?;
        self.backend()?
            .create_config(&config)
            .await
            .map_err(Into::into)
    }
    pub async fn update_remote(
        &self,
        _name: &str,
        config: &RemoteConfig,
    ) -> Result<(), OperationError> {
        let config = Self::config(config)?;
        self.backend()?
            .update_config(&config)
            .await
            .map_err(Into::into)
    }
    pub async fn delete_remote(&self, name: &str, scope: &str) -> Result<(), OperationError> {
        Self::user_scope(scope)?;
        self.backend()?
            .delete_config(name)
            .await
            .map_err(Into::into)
    }
}

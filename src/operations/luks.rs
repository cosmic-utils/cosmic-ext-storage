// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;

use storage_types::EncryptionOptionsSettings;

use super::{OperationError, StorageOperations, shared};

#[derive(Clone, Debug)]
pub struct LuksClient(Arc<StorageOperations>);

#[allow(dead_code)]
impl LuksClient {
    pub async fn new() -> Result<Self, OperationError> {
        Ok(Self(shared().await?))
    }
    pub fn with_operations(operations: Arc<StorageOperations>) -> Self {
        Self(operations)
    }
    pub async fn unlock(&self, device: &str, passphrase: &str) -> Result<String, OperationError> {
        self.0
            .registry
            .block
            .unlock_luks(device, passphrase)
            .await
            .map_err(Into::into)
    }
    pub async fn lock(&self, device: &str) -> Result<(), OperationError> {
        self.0
            .registry
            .block
            .lock_luks(device)
            .await
            .map_err(Into::into)
    }
    pub async fn change_passphrase(
        &self,
        device: &str,
        old: &str,
        new: &str,
    ) -> Result<(), OperationError> {
        self.0
            .registry
            .block
            .change_luks_passphrase(device, old, new)
            .await
            .map_err(Into::into)
    }
    pub async fn get_encryption_options(
        &self,
        device: &str,
    ) -> Result<Option<EncryptionOptionsSettings>, OperationError> {
        self.0
            .registry
            .block
            .encryption_options(device)
            .await
            .map_err(Into::into)
    }
    pub async fn set_encryption_options(
        &self,
        device: &str,
        options: &EncryptionOptionsSettings,
    ) -> Result<(), OperationError> {
        self.0
            .registry
            .block
            .set_encryption_options(device, options)
            .await
            .map_err(Into::into)
    }
    pub async fn default_encryption_options(&self, device: &str) -> Result<(), OperationError> {
        self.0
            .registry
            .block
            .clear_encryption_options(device)
            .await
            .map_err(Into::into)
    }
}

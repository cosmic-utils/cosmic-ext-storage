// SPDX-License-Identifier: GPL-3.0-only

use async_trait::async_trait;
use storage_types::{SmartInfo, SmartSelfTestKind};

use crate::StorageError;

/// Operations that affect a whole drive.
#[async_trait]
pub trait DriveOperations: Send + Sync {
    async fn smart_info(&self, device: &str) -> Result<SmartInfo, StorageError>;
    async fn start_smart_selftest(
        &self,
        device: &str,
        kind: SmartSelfTestKind,
    ) -> Result<(), StorageError>;
    async fn eject(&self, device: &str, ejectable: bool) -> Result<(), StorageError>;
    async fn power_off(&self, device: &str, can_power_off: bool) -> Result<(), StorageError>;
    async fn standby(&self, device: &str) -> Result<(), StorageError>;
    async fn wakeup(&self, device: &str) -> Result<(), StorageError>;
    async fn safe_remove(
        &self,
        device: &str,
        is_loop: bool,
        removable: bool,
        can_power_off: bool,
    ) -> Result<(), StorageError>;
}

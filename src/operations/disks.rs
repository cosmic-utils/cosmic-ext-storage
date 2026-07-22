// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;

use storage_types::{DiskInfo, SmartAttribute, SmartSelfTestKind, SmartStatus, VolumeInfo};

use super::{OperationError, StorageOperations, shared};

#[derive(Clone, Debug)]
pub struct DisksClient(Arc<StorageOperations>);

#[allow(dead_code)]
impl DisksClient {
    pub async fn new() -> Result<Self, OperationError> {
        Ok(Self(shared().await?))
    }

    pub fn with_operations(operations: Arc<StorageOperations>) -> Self {
        Self(operations)
    }

    pub async fn list_disks(&self) -> Result<Vec<DiskInfo>, OperationError> {
        self.0.registry.block.list_disks().await.map_err(Into::into)
    }

    pub async fn get_disk_info(&self, device: &str) -> Result<DiskInfo, OperationError> {
        let needle = device.strip_prefix("/dev/").unwrap_or(device);
        self.list_disks()
            .await?
            .into_iter()
            .find(|disk| {
                disk.device == device
                    || disk.device.rsplit('/').next() == Some(needle)
                    || disk.id == device
                    || disk.id == needle
            })
            .ok_or_else(|| OperationError::MissingOperation(format!("Disk not found: {device}")))
    }

    pub async fn list_volumes(&self) -> Result<Vec<VolumeInfo>, OperationError> {
        self.0
            .registry
            .block
            .list_volumes()
            .await
            .map_err(Into::into)
    }

    pub async fn get_smart_status(&self, device: &str) -> Result<SmartStatus, OperationError> {
        let device_path = if device.starts_with("/dev/") {
            device.into()
        } else {
            format!("/dev/{device}")
        };
        let info = self.0.registry.block.smart_info(&device_path).await?;
        Ok(SmartStatus {
            device: device.into(),
            healthy: !info
                .selftest_status
                .as_deref()
                .is_some_and(|status| status.to_ascii_lowercase().contains("fail")),
            temperature_celsius: info.temperature_c.map(|temperature| temperature as i16),
            power_on_hours: info.power_on_hours,
            power_cycle_count: info
                .attributes
                .get("Power_Cycle_Count")
                .and_then(|value| value.parse().ok()),
            test_running: info.selftest_status.as_deref().is_some_and(|status| {
                let status = status.to_ascii_lowercase();
                status.contains("progress") || status.contains("running")
            }),
            test_percent_remaining: None,
        })
    }

    pub async fn get_smart_attributes(
        &self,
        device: &str,
    ) -> Result<Vec<SmartAttribute>, OperationError> {
        let device_path = if device.starts_with("/dev/") {
            device.into()
        } else {
            format!("/dev/{device}")
        };
        let info = self.0.registry.block.smart_info(&device_path).await?;
        Ok(info
            .attributes
            .into_iter()
            .filter_map(|(name, value)| {
                value.parse().ok().map(|raw_value| SmartAttribute {
                    id: 0,
                    name,
                    current: 100,
                    worst: 100,
                    threshold: 0,
                    raw_value,
                    failing: false,
                })
            })
            .collect())
    }

    pub async fn start_smart_test(
        &self,
        device: &str,
        test_type: &str,
    ) -> Result<(), OperationError> {
        let kind = SmartSelfTestKind::parse(test_type).ok_or_else(|| {
            OperationError::InvalidInput("SMART test must be short or extended".into())
        })?;
        self.0
            .registry
            .block
            .start_smart_selftest(device, kind)
            .await
            .map_err(Into::into)
    }

    pub async fn eject(&self, device: &str) -> Result<(), OperationError> {
        let disk = self.get_disk_info(device).await?;
        self.0
            .registry
            .block
            .eject(&disk.device, disk.ejectable)
            .await
            .map_err(Into::into)
    }

    pub async fn power_off(&self, device: &str) -> Result<(), OperationError> {
        let disk = self.get_disk_info(device).await?;
        self.0
            .registry
            .block
            .power_off(&disk.device, disk.can_power_off)
            .await
            .map_err(Into::into)
    }

    pub async fn standby_now(&self, device: &str) -> Result<(), OperationError> {
        self.0
            .registry
            .block
            .standby(device)
            .await
            .map_err(Into::into)
    }

    pub async fn wakeup(&self, device: &str) -> Result<(), OperationError> {
        self.0
            .registry
            .block
            .wakeup(device)
            .await
            .map_err(Into::into)
    }

    pub async fn remove(&self, device: &str) -> Result<(), OperationError> {
        let disk = self.get_disk_info(device).await?;
        self.0
            .registry
            .block
            .safe_remove(
                &disk.device,
                disk.is_loop,
                disk.removable,
                disk.can_power_off,
            )
            .await
            .map_err(Into::into)
    }

    pub fn operations(&self) -> &Arc<StorageOperations> {
        &self.0
    }
}

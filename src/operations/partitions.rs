// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;

use storage_types::{CreatePartitionInfo, PartitionInfo};

use super::{OperationError, StorageOperations, shared};

#[derive(Clone, Debug)]
pub struct PartitionsClient(Arc<StorageOperations>);

#[allow(dead_code)]
impl PartitionsClient {
    pub async fn new() -> Result<Self, OperationError> {
        Ok(Self(shared().await?))
    }
    pub fn with_operations(operations: Arc<StorageOperations>) -> Self {
        Self(operations)
    }
    pub async fn list_partitions(&self, disk: &str) -> Result<Vec<PartitionInfo>, OperationError> {
        self.0
            .registry
            .block
            .list_partitions(disk)
            .await
            .map_err(Into::into)
    }
    pub async fn create_partition_table(
        &self,
        disk: &str,
        table_type: &str,
    ) -> Result<(), OperationError> {
        let table_type = match table_type.to_ascii_lowercase().as_str() {
            "gpt" => "gpt",
            "dos" | "mbr" | "msdos" => "dos",
            _ => {
                return Err(OperationError::InvalidInput(format!(
                    "Unsupported partition table: {table_type}"
                )));
            }
        };
        self.0
            .registry
            .block
            .create_partition_table(disk, table_type)
            .await
            .map_err(Into::into)
    }
    pub async fn create_partition_with_filesystem(
        &self,
        disk: &str,
        info: &CreatePartitionInfo,
    ) -> Result<String, OperationError> {
        self.0
            .registry
            .block
            .create_partition_with_filesystem(disk, info)
            .await
            .map_err(Into::into)
    }
    pub async fn delete_partition(&self, partition: &str) -> Result<(), OperationError> {
        self.0
            .registry
            .block
            .delete_partition(partition)
            .await
            .map_err(Into::into)
    }
    pub async fn resize_partition(
        &self,
        partition: &str,
        new_size: u64,
    ) -> Result<(), OperationError> {
        self.0
            .registry
            .block
            .resize_partition(partition, new_size)
            .await
            .map_err(Into::into)
    }
    pub async fn set_partition_type(
        &self,
        partition: &str,
        type_id: &str,
    ) -> Result<(), OperationError> {
        self.0
            .registry
            .block
            .set_partition_type(partition, type_id)
            .await
            .map_err(Into::into)
    }
    pub async fn set_partition_flags(
        &self,
        partition: &str,
        flags: u64,
    ) -> Result<(), OperationError> {
        self.0
            .registry
            .block
            .set_partition_flags(partition, flags)
            .await
            .map_err(Into::into)
    }
    pub async fn set_partition_name(
        &self,
        partition: &str,
        name: &str,
    ) -> Result<(), OperationError> {
        self.0
            .registry
            .block
            .set_partition_name(partition, name)
            .await
            .map_err(Into::into)
    }
}

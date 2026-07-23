// SPDX-License-Identifier: GPL-3.0-only

use async_trait::async_trait;
use storage_types::{CreatePartitionInfo, PartitionInfo};

use crate::StorageError;

#[async_trait]
pub trait PartitionOperations: Send + Sync {
    async fn list_partitions(&self, disk: &str) -> Result<Vec<PartitionInfo>, StorageError>;
    async fn create_partition_table(
        &self,
        disk: &str,
        table_type: &str,
    ) -> Result<(), StorageError>;
    async fn create_partition(
        &self,
        disk: &str,
        offset: u64,
        size: u64,
        type_id: &str,
    ) -> Result<String, StorageError>;
    async fn create_partition_with_filesystem(
        &self,
        disk: &str,
        info: &CreatePartitionInfo,
    ) -> Result<String, StorageError>;
    async fn delete_partition(&self, partition: &str) -> Result<(), StorageError>;
    async fn resize_partition(&self, partition: &str, new_size: u64) -> Result<(), StorageError>;
    async fn set_partition_type(&self, partition: &str, type_id: &str) -> Result<(), StorageError>;
    async fn set_partition_flags(&self, partition: &str, flags: u64) -> Result<(), StorageError>;
    async fn set_partition_name(&self, partition: &str, name: &str) -> Result<(), StorageError>;
}

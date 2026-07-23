use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use storage_contracts::{
    BtrfsResizeRequest, LogicalAction, LogicalOperations, LogicalTopologySource, MdRaidName,
    StorageError, StorageErrorKind,
};
use storage_types::{
    BlockDeviceFingerprint, BlockDeviceId, BlockDeviceRef, LogicalEntity, LogicalEntityId,
    LogicalEntityKind, LogicalSource, LogicalSourceAvailability,
};

fn device(number: u64) -> BlockDeviceRef {
    BlockDeviceRef::new(
        BlockDeviceId::new(8, number),
        BlockDeviceFingerprint::partition_uuid(format!("uuid-{number}")).unwrap(),
        3,
    )
}

#[test]
fn all_actions_have_typed_validation() {
    let invalid = LogicalAction::CreateLvmVolumeGroup {
        name: "vg/unsafe".into(),
        devices: vec![device(1)],
    };
    assert_eq!(
        invalid.validate().unwrap_err().kind,
        StorageErrorKind::InvalidInput
    );

    let name = MdRaidName::from_source("/dev/md0").unwrap();
    assert_eq!(name.0, "md0");
    assert!(MdRaidName::from_source("/dev/md0/").is_err());

    let resize = LogicalAction::ResizeBtrfsFilesystem {
        filesystem: LogicalEntityId::new("btrfs:fsid").unwrap(),
        request: BtrfsResizeRequest::AbsoluteBytes(0),
    };
    assert_eq!(
        resize.validate().unwrap_err().kind,
        StorageErrorKind::InvalidInput
    );
}

#[test]
fn registry_separates_sources_from_executor() {
    let source: Arc<dyn LogicalTopologySource> = Arc::new(ReadOnlySource);
    let executor: Arc<dyn LogicalOperations> = Arc::new(Executor);
    assert_eq!(source.logical_source(), LogicalSource::LocalTools);
    assert!(matches!(
        source.logical_availability(),
        LogicalSourceAvailability::Available
    ));
    let action = LogicalAction::CreateLvmVolumeGroup {
        name: "vg0".into(),
        devices: vec![device(1)],
    };
    assert!(futures::executor::block_on(executor.execute_logical_action(action)).is_ok());
}

#[test]
fn error_kind_reaches_operation_error() {
    let error = StorageError::new(StorageErrorKind::Other, "native job failed");
    assert_eq!(error.kind, StorageErrorKind::Other);
    assert_eq!(error.message, "native job failed");
}

struct ReadOnlySource;

#[async_trait]
impl LogicalTopologySource for ReadOnlySource {
    fn logical_source(&self) -> LogicalSource {
        LogicalSource::LocalTools
    }

    fn logical_availability(&self) -> LogicalSourceAvailability {
        LogicalSourceAvailability::Available
    }

    async fn list_logical_entities(&self) -> Result<Vec<LogicalEntity>, StorageError> {
        Ok(vec![LogicalEntity {
            id: LogicalEntityId::new("btrfs:read-only").unwrap(),
            kind: LogicalEntityKind::BtrfsFilesystem,
            name: "read-only".into(),
            uuid: None,
            parent_id: None,
            device_path: None,
            size_bytes: 0,
            used_bytes: None,
            free_bytes: None,
            health_status: None,
            progress_fraction: None,
            members: Vec::new(),
            capabilities: Default::default(),
            metadata: BTreeMap::new(),
        }])
    }
}

struct Executor;

#[async_trait]
impl LogicalOperations for Executor {
    async fn execute_logical_action(
        &self,
        action: LogicalAction,
    ) -> Result<storage_contracts::LogicalActionOutcome, StorageError> {
        action.validate()?;
        Ok(storage_contracts::LogicalActionOutcome {
            action,
            affected_entity_ids: Vec::new(),
            native_job_id: None,
            progress: None,
        })
    }
}

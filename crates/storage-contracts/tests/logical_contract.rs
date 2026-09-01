use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use storage_contracts::{
    BtrfsResizeRequest, ConfirmedLogicalAction, LogicalAction, LogicalOperations, LogicalPreflight,
    LogicalPreflightAvailability, LogicalPreflightKey, LogicalPreflightRequest,
    LogicalTopologySource, MdRaidName, StorageError, StorageErrorKind,
};
use storage_types::{
    BlockDeviceFingerprint, BlockDeviceId, BlockDeviceRef, BtrfsFilesystemDetails,
    BtrfsPrimaryMember, BtrfsRelativePath, BtrfsSubvolumeRef, LogicalDisplay, LogicalEntity,
    LogicalEntityDetails, LogicalEntityId, LogicalEntityKind, LogicalSource,
    LogicalSourceAvailability,
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

    let traversal = LogicalAction::CreateBtrfsSubvolume {
        filesystem: LogicalEntityId::new("btrfs:fsid").unwrap(),
        name: "../unsafe".into(),
    };
    assert_eq!(
        traversal.validate().unwrap_err().kind,
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
    let request_key = storage_contracts::LogicalPreflightRequestKey {
        target: storage_contracts::LogicalPreflightTarget::Landing,
        action_kind: action.kind(),
        logical_load_generation: 0,
        draft_revision: 1,
    };
    assert!(
        futures::executor::block_on(executor.execute_logical_action(ConfirmedLogicalAction {
            action,
            preflight_key: LogicalPreflightKey {
                request_key,
                udisks_epoch: 0
            },
        }))
        .is_ok()
    );
}

#[test]
fn error_kind_reaches_operation_error() {
    let error = StorageError::new(StorageErrorKind::Other, "native job failed");
    assert_eq!(error.kind, StorageErrorKind::Other);
    assert_eq!(error.message, "native job failed");
}

#[test]
fn btrfs_selected_actions_require_a_fresh_typed_reference() {
    let filesystem = LogicalEntityId::new("btrfs:fsid").unwrap();
    let reference = BtrfsSubvolumeRef {
        filesystem: filesystem.clone(),
        id: std::num::NonZeroU64::new(256).unwrap(),
        expected_relative_path: BtrfsRelativePath::new("home").unwrap(),
        expected_parent_id: None,
        observed_topology_epoch: 4,
    };
    assert!(
        LogicalAction::DeleteBtrfsSubvolume {
            filesystem: filesystem.clone(),
            subvolume: reference.clone(),
        }
        .validate()
        .is_ok()
    );
    assert_eq!(
        LogicalAction::DeleteBtrfsSubvolume {
            filesystem: LogicalEntityId::new("btrfs:other").unwrap(),
            subvolume: reference,
        }
        .validate()
        .unwrap_err()
        .kind,
        StorageErrorKind::InvalidInput
    );
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
            details: LogicalEntityDetails::BtrfsFilesystem(BtrfsFilesystemDetails {
                filesystem_uuid: uuid::Uuid::nil(),
                label: LogicalDisplay::known("read-only".into()),
                allocation: LogicalDisplay::unknown("fixture"),
                mount_usage: None,
                default_subvolume: LogicalDisplay::known(None),
                primary_member: BtrfsPrimaryMember::Unavailable {
                    reason: "fixture".into(),
                },
                members: Vec::new(),
                subvolumes: Vec::new(),
                diagnostics: Vec::new(),
            }),
            parent_id: None,
            capabilities: Default::default(),
            metadata: BTreeMap::new(),
            name: "read-only".into(),
            uuid: None,
            device_path: None,
            size_bytes: 0,
            used_bytes: None,
            free_bytes: None,
            health_status: None,
            progress_fraction: None,
            members: Vec::new(),
        }])
    }
}

struct Executor;

#[async_trait]
impl LogicalOperations for Executor {
    async fn capture_logical_candidate(
        &self,
        display_path: String,
    ) -> Result<storage_types::LogicalCandidateAnchor, StorageError> {
        Ok(storage_types::LogicalCandidateAnchor {
            kind: storage_types::LogicalCandidateKind::Btrfs,
            block_id: BlockDeviceId::new(8, 1),
            fingerprint: None,
            observed_epoch: 0,
            display_path,
        })
    }

    async fn preflight_logical_action(
        &self,
        request: LogicalPreflightRequest,
    ) -> Result<LogicalPreflight, StorageError> {
        Ok(LogicalPreflight {
            key: LogicalPreflightKey {
                request_key: request.request_key,
                udisks_epoch: 0,
            },
            availability: LogicalPreflightAvailability::Ready,
            device_candidates: Vec::new(),
            constraints: Default::default(),
            review: storage_contracts::LogicalReviewData::None,
        })
    }

    async fn execute_logical_action(
        &self,
        confirmed: ConfirmedLogicalAction,
    ) -> Result<storage_contracts::LogicalActionOutcome, StorageError> {
        let action = confirmed.action;
        action.validate()?;
        Ok(storage_contracts::LogicalActionOutcome {
            action,
            affected_entity_ids: Vec::new(),
            native_job_id: None,
            progress: None,
        })
    }
}

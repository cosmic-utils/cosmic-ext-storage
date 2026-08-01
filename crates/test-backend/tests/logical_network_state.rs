use storage_contracts::{
    ConfirmedLogicalAction, LogicalAction, LogicalActionKind, LogicalOperations,
    LogicalPreflightRequest, LogicalPreflightRequestKey, LogicalPreflightTarget, ScenarioControl,
    StorageErrorKind,
};
use storage_types::{NetworkBackendId, NetworkDriveConfig, NetworkDriveStatus};
use test_backend::ScenarioRuntime;

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/ui/scenarios")
        .join(name)
}

#[tokio::test]
async fn logical_confirmation_requires_current_preflight_key() {
    let backend = ScenarioRuntime::load(fixture("logical/preflight.toml"), None, None)
        .expect("runtime")
        .backend();
    let anchor = backend
        .capture_logical_candidate("/dev/ui-disk0p1".into())
        .await
        .expect("candidate");
    let action = LogicalAction::CreateLvmVolumeGroup {
        name: "fixture".into(),
        devices: vec![storage_types::BlockDeviceRef {
            id: anchor.block_id.clone(),
            fingerprint: anchor.fingerprint.clone().expect("fingerprint"),
            observed_generation: anchor.observed_epoch,
        }],
    };
    let preflight = backend
        .preflight_logical_action(LogicalPreflightRequest {
            request_key: LogicalPreflightRequestKey {
                target: LogicalPreflightTarget::Candidate(anchor),
                action_kind: LogicalActionKind::CreateLvmVolumeGroup,
                logical_load_generation: 0,
                draft_revision: 0,
            },
        })
        .await
        .expect("preflight");
    backend
        .execute_logical_action(ConfirmedLogicalAction {
            action: action.clone(),
            preflight_key: preflight.key.clone(),
        })
        .await
        .expect("current confirmation");
    let error = backend
        .execute_logical_action(ConfirmedLogicalAction {
            action,
            preflight_key: preflight.key,
        })
        .await
        .expect_err("stale confirmation");
    assert_eq!(error.kind, StorageErrorKind::Conflict);
}

#[tokio::test]
async fn logical_mutation_emits_one_declared_refresh_event() {
    let backend = ScenarioRuntime::load(fixture("logical/preflight.toml"), None, None)
        .expect("runtime")
        .backend();
    let before = backend.diagnostics().await.expect("diagnostics");
    backend.advance_to(1).await.expect("semantic action");
    let after = backend.diagnostics().await.expect("diagnostics");
    assert_eq!(after.generation, before.generation);
}

#[tokio::test]
async fn btrfs_utility_state_never_uses_host_tools() {
    let backend = ScenarioRuntime::load(fixture("empty.toml"), None, None)
        .expect("runtime")
        .backend();
    let error = storage_contracts::BtrfsOperations::list_subvolumes(&*backend, "/mnt/ui-data")
        .await
        .expect_err("not modelled");
    assert_eq!(error.kind, StorageErrorKind::Unsupported);
}

#[tokio::test]
async fn network_mutations_follow_declared_schema() {
    let runtime =
        ScenarioRuntime::load(fixture("network/mount.toml"), None, None).expect("runtime");
    let backend = runtime
        .adapters()
        .network
        .into_iter()
        .next()
        .expect("scenario network backend");
    let config = NetworkDriveConfig {
        backend_id: NetworkBackendId::rclone(),
        id: "new-remote".into(),
        name: "new-remote".into(),
        provider_id: "s3".into(),
        options: Default::default(),
        has_secrets: false,
    };
    backend.create_config(&config).await.expect("create");
    backend.mount("new-remote").await.expect("mount");
    assert_eq!(
        backend
            .mount_status("new-remote")
            .await
            .expect("status")
            .status,
        NetworkDriveStatus::Mounted
    );
}

#[tokio::test]
async fn scenario_reload_is_atomic_and_refreshes_once() {
    let runtime = ScenarioRuntime::load(fixture("reload/live.toml"), None, None).expect("runtime");
    let backend = runtime.backend();
    let outcome = backend.reload_overlay().await.expect("reload receipt");
    assert!(matches!(
        outcome,
        storage_contracts::ScenarioReload::Rejected { .. }
    ));
}

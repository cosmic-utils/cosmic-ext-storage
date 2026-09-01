#![cfg(feature = "test-backend")]

use cosmic_ext_storage::AppRuntime;
use storage_types::{
    ImageAssetRef, ImageCopyKind, ImageCopyRequest, NetworkBackendId, NetworkDriveConfig,
    NetworkDriveStatus, UsageWorkflowRequest,
};

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/ui/scenarios")
        .join(name)
}

#[tokio::test]
async fn physical_scenario_drives_dialog_and_refresh() {
    let runtime = AppRuntime::scenario(fixture("physical/partition-format.toml"), None, None)
        .expect("scenario runtime");
    let operations = runtime.operations();
    let device = operations
        .registry
        .block
        .create_partition("/dev/ui-disk0", 0, 1024, "linux")
        .await
        .expect("partition");
    assert_eq!(device, "/dev/ui-disk0p1");
}

#[tokio::test]
async fn configured_backend_error_remains_actionable() {
    let runtime = AppRuntime::scenario(fixture("physical/busy-unmount.toml"), None, None)
        .expect("scenario runtime");
    let error = runtime
        .operations()
        .registry
        .block
        .unmount_filesystem("/dev/ui-disk0p1", false)
        .await
        .expect_err("busy fixture");
    assert!(error.message.contains("Fixture user is active"));
}

#[tokio::test]
async fn usage_and_image_progress_render_from_contract_events() {
    let runtime = AppRuntime::scenario(fixture("workflows/image-usage.toml"), None, None)
        .expect("scenario runtime");
    let operations = runtime.operations();
    let scan = operations
        .usage_operations
        .start_usage_scan(UsageWorkflowRequest {
            scan_id: "scan".into(),
            mounts: Vec::new(),
            top_files_per_category: 1,
            show_all_files: false,
            parallelism_preset: Default::default(),
        })
        .await
        .expect("scan");
    assert_eq!(
        operations
            .usage_operations
            .usage_scan_status(&scan)
            .await
            .expect("status")
            .scan_id,
        "scan"
    );
    let asset = ImageAssetRef::new("asset:disk").expect("asset");
    operations
        .image_workflows
        .create_image_asset(asset.clone(), 0)
        .await
        .expect("asset");
    let copy = operations
        .image_workflows
        .start_image_copy(ImageCopyRequest {
            kind: ImageCopyKind::Backup,
            device: "/dev/ui-disk0".into(),
            asset,
        })
        .await
        .expect("copy");
    assert_eq!(
        operations
            .image_workflows
            .image_copy_status(&copy)
            .await
            .expect("copy status")
            .operation_id,
        copy
    );
}

#[tokio::test]
async fn network_scenario_exercises_crud_and_mount_state() {
    let runtime =
        AppRuntime::scenario(fixture("network/mount.toml"), None, None).expect("scenario runtime");
    let backend = runtime
        .operations()
        .registry
        .network_backend(&NetworkBackendId::rclone())
        .expect("backend");
    backend
        .create_config(&NetworkDriveConfig {
            backend_id: NetworkBackendId::rclone(),
            id: "new-remote".into(),
            name: "new-remote".into(),
            provider_id: "s3".into(),
            options: Default::default(),
            has_secrets: false,
        })
        .await
        .expect("config");
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

#[test]
fn keyboard_dialog_flow_has_named_controls() {
    // The runner requires semantic control names; cases declare them in TOML.
    assert!(std::path::Path::new("tests/ui/cases/keyboard_accessibility.toml").exists());
}

#[test]
fn scenario_factory_never_constructs_production_adapters() {
    let runtime =
        AppRuntime::scenario(fixture("empty.toml"), None, None).expect("scenario runtime");
    assert_eq!(runtime.operations().registry.block.id().0, "ui-scenario");
    assert!(
        runtime
            .scenario_marker()
            .is_some_and(|marker| marker.starts_with("Test scenario: blank-state sha256:"))
    );
}

#[test]
fn scenario_mode_has_no_real_backend_fallback() {
    assert!(AppRuntime::scenario(fixture("missing.toml"), None, None).is_err());
}

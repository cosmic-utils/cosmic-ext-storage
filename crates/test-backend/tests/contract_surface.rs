use std::sync::Arc;

use storage_contracts::{
    BlockStorageBackend, DriveOperations, ImageDeviceOperations, ScenarioOperation,
    StorageErrorKind,
};
use test_backend::{ScenarioBackend, ScenarioStore};

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/ui/scenarios")
        .join(name)
}

#[test]
fn scenario_operation_inventory_matches_contract_surface() {
    assert_eq!(ScenarioOperation::all().count(), 91);
    assert!(ScenarioOperation::parse("filesystem.unmount").is_some());
    assert!(ScenarioOperation::parse("not.a.real.operation").is_none());
}

#[test]
fn bootstrap_inventory_matches_generated_contract_surface() {
    let inventory = include_str!("../../../docs/plans/4-testing/contract-surface-v1.toml");
    for operation in ScenarioOperation::all() {
        assert!(
            inventory.contains(operation.as_str()),
            "{} missing from bootstrap inventory",
            operation
        );
    }
}

#[test]
fn scenario_crate_has_no_production_adapter_dependencies() {
    let manifest = include_str!("../Cargo.toml");
    for forbidden in [
        "storage-udisks",
        "storage-sys",
        "storage-btrfs",
        "zbus",
        "which",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "forbidden dependency: {forbidden}"
        );
    }
}

#[test]
fn scenario_store_is_the_only_host_io_boundary() {
    for source in [
        include_str!("../src/lib.rs"),
        include_str!("../src/schema.rs"),
    ] {
        assert!(!source.contains("std::fs"));
        assert!(!source.contains("std::process"));
    }
    assert!(include_str!("../src/store.rs").contains("std::fs"));
}

#[tokio::test]
async fn unsupported_methods_never_succeed_or_touch_host() {
    let backend = ScenarioBackend::load(ScenarioStore::new(fixture("empty.toml"), None, None))
        .expect("fixture loads");
    let error = backend
        .standby("/dev/ui-disk0")
        .await
        .expect_err("unsupported operation");
    assert_eq!(error.kind, StorageErrorKind::Unsupported);
    let _: Arc<dyn BlockStorageBackend> = backend;
}

#[tokio::test]
async fn native_only_legacy_image_methods_return_unsupported() {
    let backend = ScenarioBackend::load(ScenarioStore::new(fixture("empty.toml"), None, None))
        .expect("fixture loads");
    let error = backend
        .loop_setup("/tmp/not-used.img")
        .await
        .expect_err("legacy image bridge must fail closed");
    assert_eq!(error.kind, StorageErrorKind::Unsupported);
}

use futures::StreamExt;
use std::collections::BTreeMap;
use storage_contracts::{
    DeviceEventSource, EncryptionOperations, FilesystemOperations, PartitionOperations,
    StorageErrorKind,
};
use test_backend::{ScenarioBackend, ScenarioRuntime, ScenarioStore};

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/ui/scenarios")
        .join(name)
}

#[tokio::test]
async fn partition_transition_emits_ordered_device_event() {
    let runtime = ScenarioRuntime::load(fixture("physical/partition-format.toml"), None, None)
        .expect("runtime");
    let backend = runtime.backend();
    backend
        .create_partition("/dev/ui-disk0", 0, 1024, "linux")
        .await
        .expect("partition");
    let mut events = backend.device_events().await.expect("events");
    assert_eq!(
        events.next().await.expect("event").expect("valid event"),
        storage_types::DeviceEvent::Added("/dev/ui-disk0p1".into())
    );
}

#[tokio::test]
async fn busy_unmount_preserves_state() {
    let backend = ScenarioBackend::load(ScenarioStore::new(
        fixture("physical/busy-unmount.toml"),
        None,
        None,
    ))
    .expect("backend");
    let error = backend
        .unmount_filesystem("/dev/ui-disk0p1", false)
        .await
        .expect_err("busy");
    assert_eq!(error.kind, StorageErrorKind::Busy);
    assert_eq!(
        backend
            .get_mount_point("/dev/ui-disk0p1")
            .await
            .expect("still mounted"),
        "/mnt/ui-data"
    );
}

#[tokio::test]
async fn luks_fixture_requires_an_out_of_band_secret_and_transitions_once() {
    let backend = ScenarioBackend::load_with_secrets(
        ScenarioStore::new(fixture("physical/luks.toml"), None, None),
        BTreeMap::from([("luks0".into(), "fixture-passphrase".into())]),
    )
    .expect("backend");
    let error = backend
        .unlock_luks("/dev/ui-disk0p1", "incorrect")
        .await
        .expect_err("wrong secret");
    assert_eq!(error.kind, StorageErrorKind::PermissionDenied);
    let mapper = backend
        .unlock_luks("/dev/ui-disk0p1", "fixture-passphrase")
        .await
        .expect("correct secret");
    assert_eq!(mapper, "/dev/mapper/ui-crypt-data");
    let devices = backend.list_luks_devices().await.expect("luks list");
    assert_eq!(devices.len(), 1);
    assert!(devices[0].unlocked);
}

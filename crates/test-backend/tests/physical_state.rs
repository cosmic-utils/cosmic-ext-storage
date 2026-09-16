mod common;
use common::fixture;
use futures::StreamExt;
use std::collections::BTreeMap;
use storage_contracts::{
    DeviceEventSource, DiskDiscovery, EncryptionOperations, FilesystemOperations,
    PartitionOperations, StorageErrorKind,
};
use test_backend::{ScenarioBackend, ScenarioRuntime, ScenarioStore};

#[rstest::rstest]
#[tokio::test]
async fn volume_discovery_projects_current_partition_and_filesystem_state(
    #[from(common::scenario)]
    #[with("physical/partition-format.toml")]
    #[future(awt)]
    runtime: ScenarioRuntime,
) {
    let backend = runtime.backend();
    assert!(backend.list_volumes().await.unwrap().is_empty());
    backend
        .create_partition("/dev/ui-disk0", 1048576, 536870912, "linux")
        .await
        .unwrap();
    let volume = backend.list_volumes().await.unwrap().remove(0);
    assert_eq!(volume.device_path.as_deref(), Some("/dev/ui-disk0p1"));
    assert_eq!(volume.parent_path.as_deref(), Some("/dev/ui-disk0"));
    assert_eq!(volume.offset, 1048576);
    assert!(!volume.has_filesystem);
    backend
        .format_filesystem(
            "/dev/ui-disk0p1",
            "ext4",
            "Current label",
            Default::default(),
        )
        .await
        .unwrap();
    let volumes = backend.list_volumes().await.unwrap();
    assert_eq!(volumes.len(), 1);
    assert!(volumes[0].has_filesystem);
    assert_eq!(volumes[0].label, "Current label");
    assert_eq!(volumes[0].id_type, "ext4");
    backend
        .mount_filesystem("/dev/ui-disk0p1", "/mnt/fixture", Default::default())
        .await
        .unwrap();
    assert_eq!(
        backend.list_volumes().await.unwrap()[0].mount_points,
        ["/mnt/fixture"]
    );
    backend
        .unmount_filesystem("/dev/ui-disk0p1", false)
        .await
        .unwrap();
    assert!(
        backend.list_volumes().await.unwrap()[0]
            .mount_points
            .is_empty()
    );
}

#[rstest::rstest]
#[tokio::test]
async fn partition_transition_emits_ordered_device_event(
    #[from(common::scenario)]
    #[with("physical/partition-format.toml")]
    #[future(awt)]
    runtime: ScenarioRuntime,
) {
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

#[rstest::rstest]
#[tokio::test]
async fn device_subscriptions_remain_live_and_do_not_steal_each_others_events(
    #[from(common::scenario)]
    #[with("physical/partition-format.toml")]
    #[future(awt)]
    runtime: ScenarioRuntime,
) {
    let backend = runtime.backend();
    let mut first = backend.device_events().await.expect("first subscription");
    let mut second = backend.device_events().await.expect("second subscription");
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(10), first.next())
            .await
            .is_err()
    );
    backend
        .create_partition("/dev/ui-disk0", 1048576, 536870912, "linux")
        .await
        .expect("partition");
    for stream in [&mut first, &mut second] {
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(1), stream.next())
                .await
                .expect("live notification")
                .expect("open stream")
                .expect("event"),
            storage_types::DeviceEvent::Added("/dev/ui-disk0p1".into())
        );
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), stream.next())
                .await
                .is_err()
        );
    }
}

#[rstest::rstest]
#[tokio::test]
async fn busy_unmount_preserves_state(
    #[from(common::backend)]
    #[with("physical/busy-unmount.toml")]
    backend: std::sync::Arc<ScenarioBackend>,
) {
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
    let volumes = backend.list_volumes().await.unwrap();
    assert_eq!(volumes.len(), 1);
    assert!(volumes[0].locked);
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
    let volumes = backend.list_volumes().await.unwrap();
    assert_eq!(volumes.len(), 2);
    let container = volumes
        .iter()
        .find(|volume| volume.device_path.as_deref() == Some("/dev/ui-disk0p1"))
        .unwrap();
    assert!(!container.locked);
    let child = volumes
        .iter()
        .find(|volume| volume.device_path.as_deref() == Some(mapper.as_str()))
        .unwrap();
    assert_eq!(child.parent_path.as_deref(), Some("/dev/ui-disk0p1"));
    backend.lock_luks(&mapper).await.unwrap();
    let volumes = backend.list_volumes().await.unwrap();
    assert_eq!(volumes.len(), 1);
    assert!(volumes[0].locked);
}

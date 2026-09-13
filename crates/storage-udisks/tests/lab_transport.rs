//! Transport isolation tests. These use a private peer, not a disk simulator:
//! the peer always denies discovery, so no storage mutation can occur.
use std::{
    collections::HashMap,
    os::unix::net::UnixStream,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use storage_contracts::{
    DiskDiscovery, EncryptionOperations, FilesystemOperations, ImageDeviceOperations,
    PartitionOperations, StorageErrorKind,
};
use storage_udisks::{
    UdisksBackend,
    storage_types::{FormatOptions, MountOptions},
};
use zbus::{
    Connection,
    connection::Builder,
    zvariant::{OwnedObjectPath, Value},
};

struct DeniedManager {
    calls: Arc<AtomicUsize>,
    marker: String,
}

#[zbus::interface(name = "org.freedesktop.UDisks2.Manager")]
impl DeniedManager {
    fn get_block_devices(
        &self,
        _options: HashMap<String, Value<'_>>,
    ) -> zbus::fdo::Result<Vec<OwnedObjectPath>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(zbus::fdo::Error::AccessDenied(self.marker.clone()))
    }
}

async fn denied_peer(marker: &str) -> (Connection, Arc<Connection>, Arc<AtomicUsize>) {
    let (server, client) = UnixStream::pair().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let server = Builder::unix_stream(server)
        .server(zbus::Guid::generate())
        .unwrap()
        .p2p()
        .serve_at(
            "/org/freedesktop/UDisks2/Manager",
            DeniedManager {
                calls: calls.clone(),
                marker: marker.into(),
            },
        )
        .unwrap();
    let client = Builder::unix_stream(client)
        .p2p()
        .method_timeout(Duration::from_secs(2));
    let (server, client) = futures::try_join!(server.build(), client.build()).unwrap();
    (server, Arc::new(client), calls)
}

#[tokio::test]
async fn authorization_failure_maps_to_typed_storage_error() {
    let (_server, connection, calls) = denied_peer("transport-permission-sentinel").await;
    let backend = UdisksBackend::from_connection(connection);
    let error = backend.list_disks().await.unwrap_err();
    assert_eq!(error.kind, StorageErrorKind::PermissionDenied);
    assert!(error.to_string().contains("transport-permission-sentinel"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn device_operations_across_storage_families_use_selected_transport() {
    let (_server, connection, calls) = denied_peer("selected-transport").await;
    let backend = UdisksBackend::from_connection(connection);
    // A nonexistent device is intentional: this test cannot mutate storage,
    // even if a regression accidentally reaches the host's UDisks daemon.
    let device = "/dev/storage-lab-nonexistent-transport-test";
    let results = [
        backend.create_partition_table(device, "gpt").await,
        backend
            .create_partition(device, 0, 4096, "")
            .await
            .map(|_| ()),
        backend
            .format_filesystem(device, "ext4", "test", FormatOptions::default())
            .await,
        backend
            .mount_filesystem(device, "/tmp/never-mounted", MountOptions::default())
            .await
            .map(|_| ()),
        backend.unmount_filesystem(device, false).await,
        backend.filesystem_label(device).await.map(|_| ()),
        backend.set_filesystem_label(device, "test").await,
        backend.check_filesystem(device, false).await.map(|_| ()),
        backend.format_luks(device, "test-secret", "luks2").await,
        backend.unlock_luks(device, "test-secret").await.map(|_| ()),
        backend.lock_luks(device).await,
        backend
            .change_luks_passphrase(device, "before", "after")
            .await,
        backend.list_luks_devices().await.map(|_| ()),
        backend.open_for_backup(device).await.map(|_| ()),
        backend.open_for_restore(device).await.map(|_| ()),
    ];
    for (index, result) in results.into_iter().enumerate() {
        let error = result.expect_err("private peer denies every operation");
        assert!(
            error.to_string().contains("selected-transport"),
            "operation {index} escaped its transport: {error}"
        );
        assert_eq!(
            error.kind,
            StorageErrorKind::PermissionDenied,
            "operation {index}"
        );
    }
    assert!(calls.load(Ordering::SeqCst) >= 15);
}

#[tokio::test]
async fn explicit_connections_are_isolated_and_preserved_by_clones() {
    let (_server_a, a, calls_a) = denied_peer("peer-a").await;
    let (_server_b, b, calls_b) = denied_peer("peer-b").await;
    let first = UdisksBackend::from_connection(a.clone());
    let second = UdisksBackend::from_connection(b.clone());
    let clone = first.clone();
    assert!(Arc::ptr_eq(clone.manager().connection(), &a));
    assert!(Arc::ptr_eq(second.manager().connection(), &b));
    let (left, right) = tokio::join!(clone.list_disks(), second.list_disks());
    assert!(left.unwrap_err().to_string().contains("peer-a"));
    assert!(right.unwrap_err().to_string().contains("peer-b"));
    assert_eq!(calls_a.load(Ordering::SeqCst), 1);
    assert_eq!(calls_b.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn explicit_lab_transport_does_not_change_production_constructor() {
    // A machine without a system bus is also a useful negative control. Its
    // normal constructor must not suddenly start using the supplied peer.
    let before = UdisksBackend::new().await;
    let (_server, connection, calls) = denied_peer("not-the-system-bus").await;
    let _explicit = UdisksBackend::from_connection(connection);
    let after = UdisksBackend::new().await;
    match (before, after) {
        (Ok(before), Ok(after)) => assert!(Arc::ptr_eq(
            before.manager().connection(),
            after.manager().connection()
        )),
        (Err(before), Err(after)) => assert_eq!(before.to_string(), after.to_string()),
        _ => panic!("normal constructor changed availability after explicit construction"),
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

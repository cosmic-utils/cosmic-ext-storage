#![cfg(feature = "test-backend")]

mod common;

use common::scenario;
use cosmic_ext_storage::{
    AppRuntime,
    models::{UiVolume, build_volume_tree},
    operations::FilesystemsClient,
};
use rstest::{fixture, rstest};
use std::sync::Arc;
use storage_types::{VolumeInfo, VolumeKind};

#[fixture]
fn fs_client(scenario: AppRuntime) -> Arc<FilesystemsClient> {
    Arc::new(FilesystemsClient::with_operations(scenario.operations()))
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn test_build_simple_tree(fs_client: Arc<FilesystemsClient>) {
    let volumes = vec![
        VolumeInfo {
            kind: VolumeKind::Partition,
            label: "Root".to_string(),
            size: 100_000_000,
            offset: 1048576,
            partition_number: 1,
            id_type: "ext4".to_string(),
            device_path: Some("/dev/sda1".to_string()),
            parent_path: Some("/dev/sda".to_string()),
            has_filesystem: true,
            mount_points: vec!["/".to_string()],
            usage: None,
            locked: false,
            children: Vec::new(),
        },
        VolumeInfo {
            kind: VolumeKind::Partition,
            label: "Home".to_string(),
            size: 200_000_000,
            offset: 101_000_000,
            partition_number: 2,
            id_type: "ext4".to_string(),
            device_path: Some("/dev/sda2".to_string()),
            parent_path: Some("/dev/sda".to_string()),
            has_filesystem: true,
            mount_points: vec!["/home".to_string()],
            usage: None,
            locked: false,
            children: Vec::new(),
        },
    ];

    let tree = build_volume_tree("/dev/sda", volumes, fs_client).unwrap();
    assert_eq!(tree.len(), 2);
    assert_eq!(tree[0].volume.label, "Root".to_string());
    assert_eq!(tree[1].volume.label, "Home".to_string());
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn test_build_nested_tree(fs_client: Arc<FilesystemsClient>) {
    let volumes = vec![
        VolumeInfo {
            kind: VolumeKind::CryptoContainer,
            label: String::new(),
            size: 100_000_000,
            offset: 1048576,
            partition_number: 1,
            id_type: "crypto_LUKS".to_string(),
            device_path: Some("/dev/sda1".to_string()),
            parent_path: Some("/dev/sda".to_string()),
            has_filesystem: false,
            mount_points: Vec::new(),
            usage: None,
            locked: false,
            children: Vec::new(),
        },
        VolumeInfo {
            kind: VolumeKind::Block,
            label: "Secure".to_string(),
            size: 100_000_000,
            offset: 0,
            partition_number: 0,
            id_type: "ext4".to_string(),
            device_path: Some("/dev/mapper/luks-123".to_string()),
            parent_path: Some("/dev/sda1".to_string()),
            has_filesystem: true,
            mount_points: vec!["/mnt/secure".to_string()],
            usage: None,
            locked: false,
            children: Vec::new(),
        },
    ];

    let tree = build_volume_tree("/dev/sda", volumes, fs_client).unwrap();
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].volume.device_path, Some("/dev/sda1".to_string()));
    assert_eq!(tree[0].children.len(), 1);
    assert_eq!(
        tree[0].children[0].volume.device_path,
        Some("/dev/mapper/luks-123".to_string())
    );
}

fn volume(device: Option<&str>, parent: Option<&str>) -> VolumeInfo {
    VolumeInfo {
        kind: VolumeKind::Partition,
        label: "Fixture".into(),
        size: 4096,
        offset: 0,
        partition_number: 1,
        id_type: "ext4".into(),
        device_path: device.map(str::to_owned),
        parent_path: parent.map(str::to_owned),
        has_filesystem: true,
        mount_points: Vec::new(),
        usage: None,
        locked: false,
        children: Vec::new(),
    }
}

#[rstest]
#[case::empty(Vec::new(), 0)]
#[case::other_disk(vec![volume(Some("other"), Some("other-disk"))], 0)]
#[case::orphan(vec![volume(Some("orphan"), None)], 0)]
#[case::no_device(vec![volume(None, Some("disk"))], 1)]
#[tokio::test(flavor = "current_thread")]
async fn tree_filters_roots(
    fs_client: Arc<FilesystemsClient>,
    #[case] volumes: Vec<VolumeInfo>,
    #[case] expected: usize,
) {
    let tree = build_volume_tree("disk", volumes, fs_client).unwrap();
    assert_eq!(tree.len(), expected);
    for node in tree {
        assert!(node.children.is_empty());
        assert!(node.device().is_none());
        assert!(node.find_by_device("missing").is_none());
    }
}

#[rstest]
#[tokio::test(flavor = "current_thread")]
async fn nested_mutation_search_mount_and_clone_preserve_ownership(
    fs_client: Arc<FilesystemsClient>,
) {
    let mut child_info = volume(Some("child"), Some("root"));
    child_info.mount_points.push("/fixture/mount".into());
    let mut roots = build_volume_tree(
        "disk",
        vec![
            volume(Some("root"), Some("disk")),
            child_info,
            volume(Some("leaf"), Some("child")),
        ],
        fs_client.clone(),
    )
    .unwrap();
    let root = &mut roots[0];
    assert_eq!(root.device(), Some("root"));
    assert_eq!(root.device_path().as_deref(), Some("root"));
    assert!(std::ptr::eq(root.filesystem_client(), fs_client.as_ref()));
    assert!(root.can_mount());
    assert!(!root.is_mounted());
    assert_eq!(root.collect_mounted_descendants(), ["child"]);
    assert_eq!(root.find_by_device("root").unwrap().device(), Some("root"));
    assert_eq!(root.find_by_device("leaf").unwrap().device(), Some("leaf"));
    assert!(root.find_by_device("missing").is_none());
    assert!(root.find_by_device_mut("missing").is_none());

    root.find_by_device_mut("root").unwrap().volume.label = "Root".into();
    root.find_by_device_mut("child").unwrap().volume.label = "Child".into();
    let clone = root.clone();
    assert!(std::ptr::eq(
        root.filesystem_client(),
        clone.filesystem_client()
    ));
    assert_eq!(clone.children[0].label, "Child");
    let mut replacement = volume(Some("child"), Some("root"));
    replacement.label = "Updated".into();
    assert!(root.update_volume("child", &replacement));
    assert_eq!(root.children[0].label, "Updated");
    assert_eq!(root.children[0].children[0].device(), Some("leaf"));
    assert_eq!(clone.children[0].label, "Child");
    assert!(!root.update_volume("missing", &replacement));
    assert!(root.collect_mounted_descendants().is_empty());
    assert!(root.remove_child("leaf"));
    assert!(!root.remove_child("leaf"));
    assert!(root.remove_child("child"));
    root.add_child(UiVolume::with_children(volume(None, None), Vec::new(), fs_client).unwrap());
    assert!(!root.remove_child("missing"));
    assert_eq!(root.children.len(), 1);
}

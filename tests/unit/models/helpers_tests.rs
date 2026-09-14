use super::*;
use storage_types::VolumeKind;

fn test_fs_client() -> Option<Arc<FilesystemsClient>> {
    let rt = tokio::runtime::Runtime::new().ok()?;
    let client = rt.block_on(FilesystemsClient::new()).ok()?;
    Some(Arc::new(client))
}

#[test]
fn test_build_simple_tree() {
    let Some(fs_client) = test_fs_client() else {
        return; // Skip if no D-Bus (e.g. in CI)
    };
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

#[test]
fn test_build_nested_tree() {
    let Some(fs_client) = test_fs_client() else {
        return;
    };
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

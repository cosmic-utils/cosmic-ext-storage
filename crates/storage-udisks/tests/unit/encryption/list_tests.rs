use super::*;

fn make_volume(kind: VolumeKind, device: Option<&str>, locked: bool) -> VolumeInfo {
    VolumeInfo {
        kind,
        label: String::new(),
        size: 0,
        offset: 0,
        partition_number: 0,
        id_type: "crypto_LUKS_luks2".to_string(),
        device_path: device.map(ToString::to_string),
        parent_path: None,
        has_filesystem: false,
        mount_points: Vec::new(),
        usage: None,
        locked,
        children: Vec::new(),
    }
}

#[test]
fn collects_luks_devices_from_nested_tree() {
    let mut root = make_volume(VolumeKind::Partition, Some("/dev/sda1"), false);
    let mut crypto = make_volume(VolumeKind::CryptoContainer, Some("/dev/sda2"), false);
    let cleartext = make_volume(VolumeKind::Filesystem, Some("/dev/mapper/luks-abc"), false);
    crypto.children.push(cleartext);
    root.children.push(crypto);

    let mut out = Vec::new();
    collect_luks_devices(&root, &mut out);

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].device, "/dev/sda2");
    assert!(out[0].unlocked);
    assert_eq!(
        out[0].cleartext_device.as_deref(),
        Some("/dev/mapper/luks-abc")
    );
    assert_eq!(out[0].version, LuksVersion::Luks2);
}

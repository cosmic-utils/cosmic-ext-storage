use super::flatten_volumes_to_partitions;
use storage_types::{VolumeInfo, VolumeKind};

fn volume(
    kind: VolumeKind,
    partition_number: u32,
    offset: u64,
    device_path: Option<&str>,
    children: Vec<VolumeInfo>,
) -> VolumeInfo {
    VolumeInfo {
        kind,
        label: String::new(),
        size: 1024,
        offset,
        partition_number,
        id_type: "ext4".to_string(),
        device_path: device_path.map(ToOwned::to_owned),
        parent_path: None,
        has_filesystem: true,
        mount_points: vec![],
        usage: None,
        locked: false,
        children,
    }
}

#[test]
fn flatten_partitions_sorts_by_offset() {
    let volumes = vec![
        volume(VolumeKind::Partition, 2, 4096, Some("/dev/sda2"), vec![]),
        volume(VolumeKind::Partition, 1, 2048, Some("/dev/sda1"), vec![]),
    ];

    let partitions = flatten_volumes_to_partitions(&volumes, "/dev/sda");

    let devices: Vec<&str> = partitions.iter().map(|p| p.device.as_str()).collect();
    assert_eq!(devices, vec!["/dev/sda1", "/dev/sda2"]);
}

#[test]
fn flatten_partitions_ignores_non_partition_and_nested_children() {
    let nested_child = volume(
        VolumeKind::Partition,
        99,
        8192,
        Some("/dev/mapper/inner"),
        vec![],
    );
    let volumes = vec![
        volume(VolumeKind::Filesystem, 0, 1024, Some("/dev/sda"), vec![]),
        volume(
            VolumeKind::Partition,
            1,
            2048,
            Some("/dev/sda1"),
            vec![nested_child],
        ),
    ];

    let partitions = flatten_volumes_to_partitions(&volumes, "/dev/sda");

    assert_eq!(partitions.len(), 1);
    assert_eq!(partitions[0].device, "/dev/sda1");
}

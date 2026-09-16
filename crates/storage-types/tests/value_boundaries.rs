use rstest::{fixture, rstest};
use storage_types::*;

#[rstest]
#[case("ext4", FilesystemType::Ext4, "mkfs.ext4")]
#[case("XFS", FilesystemType::Xfs, "mkfs.xfs")]
#[case("btrfs", FilesystemType::Btrfs, "mkfs.btrfs")]
#[case("vfat", FilesystemType::Fat32, "mkfs.vfat")]
#[case("fat32", FilesystemType::Fat32, "mkfs.vfat")]
#[case("ntfs", FilesystemType::Ntfs, "mkfs.ntfs")]
#[case("exfat", FilesystemType::Exfat, "mkfs.exfat")]
#[case("unknown", FilesystemType::Other, "")]
fn filesystem_names_map_to_the_intended_tool(
    #[case] input: &str,
    #[case] expected: FilesystemType,
    #[case] command: &str,
) {
    assert_eq!(FilesystemType::parse(input), expected);
    assert_eq!(expected.mkfs_command(), command);
}

#[rstest]
#[case("gpt", Some(PartitionTableType::Gpt), "gpt")]
#[case("mbr", Some(PartitionTableType::Mbr), "dos")]
#[case("dos", Some(PartitionTableType::Mbr), "dos")]
#[case("unknown", None, "")]
fn partition_table_aliases_are_explicit(
    #[case] input: &str,
    #[case] expected: Option<PartitionTableType>,
    #[case] output: &str,
) {
    assert_eq!(PartitionTableType::parse(input), expected);
    if let Some(value) = expected {
        assert_eq!(value.as_udisks_str(), output);
    }
}

#[rstest]
#[case("LUKS1", Some(LuksVersion::Luks1), "luks1")]
#[case("1", Some(LuksVersion::Luks1), "luks1")]
#[case("LUKS2", Some(LuksVersion::Luks2), "luks2")]
#[case("2", Some(LuksVersion::Luks2), "luks2")]
#[case("3", None, "")]
fn luks_versions_are_closed(
    #[case] input: &str,
    #[case] expected: Option<LuksVersion>,
    #[case] output: &str,
) {
    assert_eq!(LuksVersion::parse(input), expected);
    if let Some(value) = expected {
        assert_eq!(value.as_str(), output);
    }
}

#[rstest]
#[case("SHORT", Some(SmartSelfTestKind::Short), "short")]
#[case("extended", Some(SmartSelfTestKind::Extended), "extended")]
#[case("long", Some(SmartSelfTestKind::Extended), "extended")]
#[case("invalid", None, "")]
fn smart_test_names_do_not_allow_arbitrary_commands(
    #[case] input: &str,
    #[case] expected: Option<SmartSelfTestKind>,
    #[case] output: &str,
) {
    assert_eq!(SmartSelfTestKind::parse(input), expected);
    if let Some(value) = expected {
        assert_eq!(value.as_udisks_str(), output);
    }
}

#[rstest]
#[case(0, 0, 0, false, 0, 0)]
#[case(0, 10, 10, true, 0, 10)]
#[case(4, 12, 10, false, 4, 10)]
#[case(12, 4, 10, false, 10, 4)]
#[case(u64::MAX, u64::MAX, 1, false, 1, 1)]
fn byte_ranges_are_bounded_without_underflow(
    #[case] start: u64,
    #[case] end: u64,
    #[case] disk: u64,
    #[case] valid: bool,
    #[case] clamped_start: u64,
    #[case] clamped_end: u64,
) {
    let range = ByteRange { start, end };
    assert_eq!(range.is_valid_for_disk(disk), valid);
    assert_eq!(range.size(), end.saturating_sub(start));
    assert_eq!(
        range.clamp_to_disk(disk),
        ByteRange {
            start: clamped_start,
            end: clamped_end
        }
    );
}

#[rstest]
#[case("0 B", 0)]
#[case("1.5 KB", 1536)]
#[case("2 MB", 2 * 1024 * 1024)]
#[case("1 GB", 1024_u64.pow(3))]
#[case("1 TB", 1024_u64.pow(4))]
#[case("1 PB", 1024_u64.pow(5))]
#[case("1 EB", 1024_u64.pow(6))]
fn byte_parser_handles_units(#[case] input: &str, #[case] expected: u64) {
    assert_eq!(pretty_to_bytes(input).unwrap(), expected);
}

#[rstest]
#[case("")]
#[case("bytes")]
#[case("1 XX")]
#[case("-1 GB")]
#[case("NaN B")]
#[case("inf MB")]
#[case("1 ZB")]
#[case("1 YB")]
#[case("1 garbage GB")]
fn byte_parser_rejects_malformed_or_unrepresentable_sizes(#[case] input: &str) {
    assert!(pretty_to_bytes(input).is_err(), "{input}");
}

#[rstest]
#[case(0, "0.00 B", 0.0, 1.0)]
#[case(1024, "1024.00 B", 1024.0, 1.0)]
#[case(1536, "1.50 KB", 1.5, 1024.0)]
#[case(1_572_864, "1.50 MB", 1.5, 1_048_576.0)]
fn byte_display_numeric_and_spinner_step_agree(
    #[case] bytes: u64,
    #[case] display: &str,
    #[case] numeric: f64,
    #[case] step: f64,
) {
    assert_eq!(bytes_to_pretty(&bytes, false), display);
    assert_eq!(get_numeric(&bytes), numeric);
    assert_eq!(get_step(&bytes), step);
    assert!(bytes_to_pretty(&bytes, true).starts_with(display));
    assert!(bytes_to_pretty(&bytes, true).ends_with(" bytes)"));
}

#[fixture]
fn volume() -> VolumeInfo {
    VolumeInfo {
        kind: VolumeKind::Partition,
        label: String::new(),
        size: 1024,
        offset: 0,
        partition_number: 1,
        id_type: "ext4".into(),
        device_path: Some("/dev/fixture".into()),
        parent_path: None,
        has_filesystem: true,
        mount_points: vec![],
        usage: None,
        locked: false,
        children: vec![],
    }
}

#[rstest]
fn nested_volumes_preserve_identity_and_mount_capability(mut volume: VolumeInfo) {
    assert!(volume.can_mount());
    assert!(!volume.is_mounted());
    assert!(!volume.can_lock() && !volume.can_unlock());
    volume.mount_points.push("/mnt/fixture".into());
    assert!(volume.is_mounted() && !volume.can_mount());
    volume.has_filesystem = false;
    assert!(!volume.is_mounted() && !volume.can_mount());
    volume.kind = VolumeKind::CryptoContainer;
    assert!(volume.can_lock() && !volume.can_unlock());
    volume.locked = true;
    assert!(volume.can_unlock() && !volume.can_lock());
    let mut child = volume.clone();
    child.device_path = Some("/dev/child".into());
    let mut grandchild = child.clone();
    grandchild.device_path = Some("/dev/grandchild".into());
    child.children.push(grandchild);
    volume.children.push(child);
    assert_eq!(volume.volume_count(), 3);
    assert_eq!(
        volume
            .find_by_device("/dev/grandchild")
            .unwrap()
            .device_path
            .as_deref(),
        Some("/dev/grandchild")
    );
    assert!(volume.find_by_device("missing").is_none());
    assert_eq!(volume.name(), "/dev/fixture");
    volume.label = "Archive".into();
    assert_eq!(volume.name(), "Archive");
    volume.label.clear();
    volume.device_path = None;
    assert_eq!(volume.name(), "Unknown");
}

#[rstest]
#[case(false, false, false, 0)]
#[case(true, false, false, 4)]
#[case(false, true, false, 1)]
#[case(false, false, true, 1 << 62)]
#[case(true, true, true, (1 << 62) | 5)]
fn partition_flags_preserve_udisks_bit_positions(
    #[case] legacy: bool,
    #[case] system: bool,
    #[case] hidden: bool,
    #[case] bits: u64,
) {
    assert_eq!(make_partition_flags_bits(legacy, system, hidden), bits);
}

#[rstest]
#[case(rclone::MountStatus::Unmounted, true, false)]
#[case(rclone::MountStatus::Mounting, false, false)]
#[case(rclone::MountStatus::Mounted, true, true)]
#[case(rclone::MountStatus::Unmounting, false, false)]
#[case(rclone::MountStatus::Error("failed".into()), true, false)]
fn mount_progress_and_terminal_states_are_distinct(
    #[case] status: rclone::MountStatus,
    #[case] terminal: bool,
    #[case] mounted: bool,
) {
    assert_eq!(status.is_terminal(), terminal);
    assert_eq!(status.is_mounted(), mounted);
}

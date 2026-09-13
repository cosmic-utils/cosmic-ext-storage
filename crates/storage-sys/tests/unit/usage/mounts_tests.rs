use super::{parse_local_mounts, used_bytes_from_fields};

#[test]
fn parses_mountinfo_and_filters_non_local_types() {
    let sample = "36 25 8:2 / / rw,relatime - ext4 /dev/nvme0n1p2 rw\n37 25 0:5 / /proc rw,nosuid,nodev,noexec,relatime - proc proc rw\n38 25 0:57 / /mnt/nfs rw,relatime - nfs server:/x rw\n";

    let mounts = parse_local_mounts(sample).expect("parse should succeed");
    assert_eq!(mounts, vec![std::path::PathBuf::from("/")]);
}

#[test]
fn sums_used_bytes_for_included_mounts() {
    assert_eq!(used_bytes_from_fields(1_000, 250, 4096), 3_072_000);
}

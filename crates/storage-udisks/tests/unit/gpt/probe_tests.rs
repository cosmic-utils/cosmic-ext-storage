use super::*;

#[test]
fn parses_first_last_usable_lba() {
    let mut sector = vec![0u8; 512];
    sector[0..8].copy_from_slice(b"EFI PART");
    sector[12..16].copy_from_slice(&(92u32.to_le_bytes()));
    sector[40..48].copy_from_slice(&(34u64.to_le_bytes()));
    sector[48..56].copy_from_slice(&(1000u64.to_le_bytes()));

    let res = gpt_parse_first_last_usable_lba(&sector).unwrap();
    assert_eq!(res, (34, 1000));
}

#[test]
fn rejects_non_gpt_signature() {
    let sector = vec![0u8; 512];
    assert!(gpt_parse_first_last_usable_lba(&sector).is_none());
}

#[test]
fn fallback_returns_none_for_small_disks() {
    // Disk exactly at threshold (2 MiB)
    assert!(fallback_gpt_usable_range_bytes(2 * GPT_ALIGNMENT_BYTES).is_none());
    // Disk smaller than threshold
    assert!(fallback_gpt_usable_range_bytes(1024 * 1024).is_none());
    assert!(fallback_gpt_usable_range_bytes(0).is_none());
}

#[test]
fn fallback_returns_range_for_larger_disks() {
    // 10 MiB disk should return range with 1 MiB reserved at each end
    let disk_size = 10 * 1024 * 1024;
    let range = fallback_gpt_usable_range_bytes(disk_size).unwrap();
    assert_eq!(range.start, GPT_ALIGNMENT_BYTES);
    assert_eq!(range.end, disk_size - GPT_ALIGNMENT_BYTES);

    // 100 GiB disk
    let disk_size = 100 * 1024 * 1024 * 1024;
    let range = fallback_gpt_usable_range_bytes(disk_size).unwrap();
    assert_eq!(range.start, GPT_ALIGNMENT_BYTES);
    assert_eq!(range.end, disk_size - GPT_ALIGNMENT_BYTES);
}

#[test]
fn rejects_gpt_header_size_larger_than_sector() {
    let mut sector = vec![0u8; 512];
    sector[0..8].copy_from_slice(b"EFI PART");
    // Set header_size to 1024 (larger than our 512-byte sector)
    sector[12..16].copy_from_slice(&(1024u32.to_le_bytes()));
    sector[40..48].copy_from_slice(&(34u64.to_le_bytes()));
    sector[48..56].copy_from_slice(&(1000u64.to_le_bytes()));

    assert!(gpt_parse_first_last_usable_lba(&sector).is_none());
}

#[test]
fn rejects_first_usable_equal_to_last_usable() {
    let mut sector = vec![0u8; 512];
    sector[0..8].copy_from_slice(b"EFI PART");
    sector[12..16].copy_from_slice(&(92u32.to_le_bytes()));
    sector[40..48].copy_from_slice(&(100u64.to_le_bytes()));
    sector[48..56].copy_from_slice(&(100u64.to_le_bytes()));

    assert!(gpt_parse_first_last_usable_lba(&sector).is_none());
}

#[test]
fn rejects_first_usable_greater_than_last_usable() {
    let mut sector = vec![0u8; 512];
    sector[0..8].copy_from_slice(b"EFI PART");
    sector[12..16].copy_from_slice(&(92u32.to_le_bytes()));
    sector[40..48].copy_from_slice(&(1000u64.to_le_bytes()));
    sector[48..56].copy_from_slice(&(34u64.to_le_bytes()));

    assert!(gpt_parse_first_last_usable_lba(&sector).is_none());
}

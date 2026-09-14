use super::*;

#[test]
fn decode_c_string_bytes_truncates_nul() {
    let bytes = b"/run/media/user/DISK\0garbage";
    assert_eq!(decode_c_string_bytes(bytes), "/run/media/user/DISK");
}

#[test]
fn decode_mount_points_filters_empty_entries() {
    let decoded = decode_mount_points(vec![
        b"/mnt/a\0".to_vec(),
        b"\0".to_vec(),
        Vec::new(),
        b"/mnt/b".to_vec(),
    ]);

    assert_eq!(decoded, vec!["/mnt/a".to_string(), "/mnt/b".to_string()]);
}

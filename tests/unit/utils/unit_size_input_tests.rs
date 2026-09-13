use super::*;

#[test]
fn test_bytes_identity() {
    let bytes = 12345u64;
    assert_eq!(SizeUnit::Bytes.to_bytes(bytes as f64), bytes);
    assert_eq!(SizeUnit::Bytes.from_bytes(bytes), bytes as f64);
}

#[test]
fn test_megabytes_to_bytes() {
    let mb = 100.0;
    let expected = 104857600u64; // 100 * 1024 * 1024
    assert_eq!(SizeUnit::Megabytes.to_bytes(mb), expected);
}

#[test]
fn test_bytes_to_megabytes() {
    let bytes = 104857600u64; // 100 MB
    let mb = SizeUnit::Megabytes.from_bytes(bytes);
    assert!((mb - 100.0).abs() < 0.001);
}

#[test]
fn test_gigabytes_to_bytes() {
    let gb = 5.0;
    let expected = 5368709120u64; // 5 * 1024^3
    assert_eq!(SizeUnit::Gigabytes.to_bytes(gb), expected);
}

#[test]
fn test_bytes_to_gigabytes() {
    let bytes = 1073741824u64; // 1 GB
    let gb = SizeUnit::Gigabytes.from_bytes(bytes);
    assert!((gb - 1.0).abs() < 0.001);
}

#[test]
fn test_terabytes_conversion() {
    let tb = 2.5;
    let bytes = SizeUnit::Terabytes.to_bytes(tb);
    let back_to_tb = SizeUnit::Terabytes.from_bytes(bytes);
    assert!((back_to_tb - tb).abs() < 0.001);
}

#[test]
fn test_unit_index_roundtrip() {
    for unit in [
        SizeUnit::Bytes,
        SizeUnit::Kilobytes,
        SizeUnit::Megabytes,
        SizeUnit::Gigabytes,
        SizeUnit::Terabytes,
    ] {
        let idx = unit.to_index();
        let recovered = SizeUnit::from_index(idx);
        assert_eq!(unit, recovered);
    }
}

#[test]
fn test_auto_select() {
    assert_eq!(SizeUnit::auto_select(512), SizeUnit::Bytes);
    assert_eq!(SizeUnit::auto_select(2048), SizeUnit::Kilobytes);
    assert_eq!(SizeUnit::auto_select(5 * 1024 * 1024), SizeUnit::Megabytes);
    assert_eq!(
        SizeUnit::auto_select(3 * 1024 * 1024 * 1024),
        SizeUnit::Gigabytes
    );
    assert_eq!(
        SizeUnit::auto_select(2 * 1024u64 * 1024 * 1024 * 1024),
        SizeUnit::Terabytes
    );
}

#[test]
fn test_labels() {
    let labels = SizeUnit::labels();
    assert_eq!(labels.len(), 5);
    assert_eq!(labels[0], "B");
    assert_eq!(labels[1], "KB");
    assert_eq!(labels[2], "MB");
    assert_eq!(labels[3], "GB");
    assert_eq!(labels[4], "TB");
}

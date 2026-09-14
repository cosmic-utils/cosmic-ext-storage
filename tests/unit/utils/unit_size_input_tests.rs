use super::*;
use rstest::rstest;

#[test]
fn test_bytes_identity() {
    let bytes = 12345u64;
    assert_eq!(SizeUnit::Bytes.to_bytes(bytes as f64), bytes);
    assert_eq!(SizeUnit::Bytes.from_bytes(bytes), bytes as f64);
}

#[rstest]
#[case::megabytes(SizeUnit::Megabytes, 100.0, 104857600)]
#[case::gigabytes(SizeUnit::Gigabytes, 5.0, 5368709120)]
fn unit_to_bytes(#[case] unit: SizeUnit, #[case] value: f64, #[case] expected: u64) {
    assert_eq!(unit.to_bytes(value), expected);
}

#[rstest]
#[case::megabytes(SizeUnit::Megabytes, 104857600, 100.0)]
#[case::gigabytes(SizeUnit::Gigabytes, 1073741824, 1.0)]
fn bytes_to_unit(#[case] unit: SizeUnit, #[case] bytes: u64, #[case] expected: f64) {
    assert!((unit.from_bytes(bytes) - expected).abs() < 0.001);
}

#[test]
fn test_terabytes_conversion() {
    let tb = 2.5;
    let bytes = SizeUnit::Terabytes.to_bytes(tb);
    let back_to_tb = SizeUnit::Terabytes.from_bytes(bytes);
    assert!((back_to_tb - tb).abs() < 0.001);
}

#[rstest]
#[case::bytes(SizeUnit::Bytes)]
#[case::kilobytes(SizeUnit::Kilobytes)]
#[case::megabytes(SizeUnit::Megabytes)]
#[case::gigabytes(SizeUnit::Gigabytes)]
#[case::terabytes(SizeUnit::Terabytes)]
fn test_unit_index_roundtrip(#[case] unit: SizeUnit) {
    let idx = unit.to_index();
    let recovered = SizeUnit::from_index(idx);
    assert_eq!(unit, recovered);
}

#[rstest]
#[case::bytes(512, SizeUnit::Bytes)]
#[case::kilobytes(2048, SizeUnit::Kilobytes)]
#[case::megabytes(5 * 1024 * 1024, SizeUnit::Megabytes)]
#[case::gigabytes(3 * 1024 * 1024 * 1024, SizeUnit::Gigabytes)]
#[case::terabytes(2 * 1024u64 * 1024 * 1024 * 1024, SizeUnit::Terabytes)]
fn test_auto_select(#[case] bytes: u64, #[case] expected: SizeUnit) {
    assert_eq!(SizeUnit::auto_select(bytes), expected);
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

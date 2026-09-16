use super::*;
use rstest::rstest;

#[test]
fn startup_secret_map_accepts_unicode_and_empty_inventory() {
    let source = r#"[["luks0","test-é🦀"]]"#;
    assert_eq!(read(source.as_bytes()).unwrap()["luks0"], "test-é🦀");
    assert!(read(b"[]".as_slice()).unwrap().is_empty());
}

#[rstest]
#[case::malformed("fixture-secret")]
#[case::duplicate(r#"[["luks0","fixture-secret"],["luks0","another"]]"#)]
#[case::empty_id(r#"[["","fixture-secret"]]"#)]
#[case::unsafe_id(r#"[["../luks","fixture-secret"]]"#)]
#[case::empty_value(r#"[["luks0",""]]"#)]
#[case::control(r#"[["luks0","fixture-secret\n"]]"#)]
fn invalid_startup_secrets_fail_without_echoing_input(#[case] source: &str) {
    let error = read(source.as_bytes()).unwrap_err();
    assert_eq!(error.to_string(), "invalid scenario secret input");
    assert!(!format!("{error:?}").contains("fixture-secret"));
}

#[test]
fn startup_secret_input_is_bounded() {
    assert!(read(vec![b' '; 16_385].as_slice()).is_err());
    let entries = vec![("id", "value"); 17];
    assert!(read(serde_json::to_vec(&entries).unwrap().as_slice()).is_err());
    let entries = vec![("id", "x".repeat(4097))];
    assert!(read(serde_json::to_vec(&entries).unwrap().as_slice()).is_err());
}

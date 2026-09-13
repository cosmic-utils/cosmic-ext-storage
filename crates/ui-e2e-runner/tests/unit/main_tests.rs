use super::*;

#[test]
fn png_signature_requires_all_eight_bytes() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("image.png");
    fs::write(&path, b"\x89PNG\r\n\x1a\nbytes").expect("write PNG");
    assert!(is_png(&path).expect("read PNG"));
    fs::write(&path, b"PNG").expect("write non-PNG");
    assert!(!is_png(&path).expect("read non-PNG"));
}

#[test]
fn legacy_case_inventory_is_sorted_and_rejects_duplicate_ids() {
    let directory = tempfile::tempdir().expect("temporary directory");
    fs::write(
        directory.path().join("z.toml"),
        "schema_version = 1\nid = \"z_case\"\n",
    )
    .expect("write case");
    fs::write(
        directory.path().join("a.toml"),
        "schema_version = 1\nid = \"a_case\"\n",
    )
    .expect("write case");
    assert_eq!(
        list_case_ids(directory.path()).expect("list cases"),
        ["a_case", "z_case"]
    );

    fs::write(
        directory.path().join("duplicate.toml"),
        "schema_version = 1\nid = \"a_case\"\n",
    )
    .expect("write duplicate case");
    assert!(list_case_ids(directory.path()).is_err());
}

#[test]
fn interactive_role_gate_rejects_window_only_trees() {
    assert!(is_interactive_role("button"));
    assert!(is_interactive_role("scroll bar"));
    assert!(!is_interactive_role("application"));
    assert!(!is_interactive_role("frame"));
    assert!(!is_interactive_role("paragraph"));
}

#[test]
fn scenario_marker_binds_the_fixture_id_and_hash() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("fixture.toml");
    fs::write(&path, "schema_version = 2\nid = \"fixture-id\"\n").expect("write scenario fixture");

    let marker = scenario_marker(&path).expect("build scenario marker");
    assert!(marker.starts_with("Test scenario: fixture-id sha256:"));
    assert_eq!(marker.len(), "Test scenario: fixture-id sha256:".len() + 64);
}

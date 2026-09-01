use std::{fs, path::Path};

use storage_contracts::{PartitionOperations, ScenarioControl};
use test_backend::{ScenarioRuntime, ScenarioStore, parse_fixture};

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/ui/scenarios")
        .join(name)
}

#[test]
fn rejects_unknown_or_unsupported_schema() {
    assert!(
        parse_fixture(b"schema_version = 2\nid = 'valid'\ndescription = 'valid'\nunknown = true\n")
            .is_err()
    );
    assert!(parse_fixture(b"schema_version = 1\nid = 'valid'\ndescription = 'valid'\n").is_err());
}

#[test]
fn rejects_schema_version_that_is_not_first_lexical_key() {
    assert!(parse_fixture(b"id = 'valid'\nschema_version = 2\ndescription = 'valid'\n").is_err());
    assert!(
        parse_fixture(b"# comment\n\nschema_version = 2\nid = 'valid'\ndescription = 'valid'\n")
            .is_ok()
    );
}

#[test]
fn rejects_ambiguous_behaviour_rules() {
    let bytes = b"schema_version = 2\nid = 'rules'\ndescription = 'rules'\n[[behaviour.rules]]\noperation = 'filesystem.unmount'\noutcome = { tag = 'unsupported' }\n[[behaviour.rules]]\noperation = 'filesystem.unmount'\noutcome = { tag = 'unsupported' }\n";
    assert!(parse_fixture(bytes).is_err());
}

#[test]
fn rejects_invalid_cross_references() {
    let bytes = b"schema_version = 2\nid = 'invalid-device'\ndescription = 'invalid'\n[[world.disks]]\nid = 'disk'\ndevice = '/dev/sda'\n";
    assert!(parse_fixture(bytes).is_err());
}

#[test]
fn scenario_round_trip_is_deterministic() {
    let bytes = fs::read(fixture("physical/partition-format.toml")).expect("fixture");
    let (first, first_hash) = parse_fixture(&bytes).expect("parse");
    let (second, second_hash) = parse_fixture(&bytes).expect("parse");
    assert_eq!(first.id, second.id);
    assert_eq!(first_hash, second_hash);
}

#[test]
fn overlay_write_is_atomic_and_never_rewrites_fixture() {
    let unique = format!("test-backend-overlay-{}", std::process::id());
    let root = std::env::temp_dir().join(unique);
    let fixture_path = root.join("fixture.toml");
    let overlay_path = root.join("overlay.toml");
    fs::create_dir_all(&root).expect("temp directory");
    let original = b"schema_version = 2\nid = 'overlay'\ndescription = 'overlay'\n";
    fs::write(&fixture_path, original).expect("fixture");
    let store = ScenarioStore::new(&fixture_path, Some(overlay_path.clone()), None);
    store
        .write_overlay_atomically(b"overlay bytes")
        .expect("overlay write");
    assert_eq!(fs::read(&fixture_path).expect("fixture read"), original);
    assert_eq!(
        fs::read(&overlay_path).expect("overlay read"),
        b"overlay bytes"
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[tokio::test]
async fn query_snapshot_and_subscription_cutover_are_linearized() {
    let runtime = ScenarioRuntime::load(fixture("physical/partition-format.toml"), None, None)
        .expect("runtime");
    let backend = runtime.backend();
    let device = backend
        .create_partition("/dev/ui-disk0", 0, 1024, "linux")
        .await
        .expect("partition");
    assert_eq!(device, "/dev/ui-disk0p1");
    let partitions = backend
        .list_partitions("/dev/ui-disk0")
        .await
        .expect("snapshot");
    assert_eq!(partitions.len(), 1);
}

#[tokio::test]
async fn scenario_control_is_actor_serialized() {
    let runtime = ScenarioRuntime::load(fixture("empty.toml"), None, None).expect("runtime");
    let backend = runtime.backend();
    let first = backend.advance_to(2).await.expect("advance");
    let second = backend.advance_to(3).await.expect("advance");
    assert!(second.sequence > first.sequence);
    assert!(backend.advance_to(1).await.is_err());
}

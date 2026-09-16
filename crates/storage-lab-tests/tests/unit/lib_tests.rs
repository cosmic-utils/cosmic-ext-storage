use super::{LabFixture, LabRoot, is_lab_loop_device};

#[test]
fn rejects_non_loop_and_physical_device_patterns() {
    for device in [
        "/dev/sda",
        "/dev/sda1",
        "/dev/nvme0n1",
        "/dev/vda",
        "/dev/loop",
    ] {
        assert!(!is_lab_loop_device(device), "must reject {device}");
    }
    assert!(is_lab_loop_device("/dev/loop42"));
}

#[test]
fn fixture_cleanup_is_idempotent_after_partial_setup() {
    let mut fixture = LabFixture::create("partial-cleanup").expect("create lab root");
    let root = fixture.root().expect("fixture root").path().to_owned();

    fixture.cleanup().expect("first cleanup");
    fixture.cleanup().expect("second cleanup");
    assert!(!root.exists());
}

#[test]
fn fixture_paths_reject_parent_traversal() {
    let root = LabRoot::create("path-validation").expect("create lab root");
    assert!(root.sparse_file("../outside.img", 1).is_err());
    assert!(root.sparse_file("..", 1).is_err());
    assert!(root.sparse_file(".", 1).is_err());
    for name in [
        "",
        " disk.img",
        "disk\nloop\t/dev/sda",
        "disk\t.img",
        "disk\r.img",
        "disk\\img",
    ] {
        assert!(
            root.sparse_file(name, 1).is_err(),
            "must reject ledger-ambiguous name {name:?}"
        );
    }
    root.sparse_file("unique.img", 1).expect("first allocation");
    assert!(root.sparse_file("unique.img", 2).is_err());
    root.remove().expect("remove lab root");
}

#[test]
fn ledger_rejects_non_owned_and_physical_device_patterns() {
    let fixture = LabFixture::create("ownership").unwrap();
    for path in [
        "/dev/loop0",
        "/dev/loop999",
        "/dev/sda",
        "/dev/nvme0n1",
        "/dev/vda",
        "/dev/mapper/root",
    ] {
        assert!(fixture.owned_loop(std::path::Path::new(path)).is_err());
    }
}

#[test]
fn ledger_survives_cleanup_and_records_root_lifecycle() {
    let mut fixture = LabFixture::create("evidence").unwrap();
    let ledger = fixture.ledger_path().to_owned();
    fixture.cleanup().unwrap();
    let text = std::fs::read_to_string(&ledger).unwrap();
    assert!(text.starts_with("root\t/tmp/storage-lab/evidence-"));
    assert!(text.contains("removed-root\t"));
    std::fs::remove_file(ledger).unwrap();
}

#[test]
fn labels_cannot_escape_or_be_empty() {
    for label in ["", "../escape", "has space", "has\nnewline", "é"] {
        assert!(LabRoot::create(label).is_err());
    }
}

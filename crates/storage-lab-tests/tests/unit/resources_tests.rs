use super::*;
#[test]
fn group_names_cannot_be_options_or_host_groups() {
    for name in ["root", "--force", "storage-lab-a/b", "storage-lab-a\n"] {
        assert!(validate_group_name(name).is_err());
    }
    assert!(validate_group_name("storage-lab-fixture-123").is_ok());
}
#[test]
fn unowned_ancestry_is_rejected_without_mutation() {
    let fixture = LabFixture::create("ancestry").unwrap();
    for path in [
        "/dev/sda",
        "/dev/nvme0n1",
        "/dev/loop0",
        "/dev/../sda",
        "/tmp/disk",
    ] {
        assert!(fixture.verify_derived_device(Path::new(path)).is_err());
    }
}

use super::*;
#[test]
fn alias_creation_handles_udev_races_without_overwriting_foreign_targets() {
    let fixture = crate::LabFixture::create("alias-race").unwrap();
    let root = fixture.root().unwrap().path();
    let target = root.join("missing-owned-node");
    let link = root.join("alias");
    assert!(ensure_array_alias(&link, &target, || Ok(())).unwrap());
    // read_link, unlike exists(), recognizes an already-created dangling alias.
    assert!(!ensure_array_alias(&link, &target, || panic!("must not recreate")).unwrap());
    fs::remove_file(&link).unwrap();
    assert!(
        !ensure_array_alias(&link, &target, || {
            std::os::unix::fs::symlink(&target, &link)?;
            Ok(())
        })
        .unwrap()
    );
    fs::remove_file(&link).unwrap();
    let foreign = root.join("foreign");
    assert!(
        ensure_array_alias(&link, &target, || {
            std::os::unix::fs::symlink(&foreign, &link)?;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(fs::read_link(&link).unwrap(), foreign);
    assert!(ensure_array_alias(&link, &target, || Ok(())).is_err());
    fs::remove_file(&link).unwrap();
    fs::write(&link, b"not an alias").unwrap();
    assert!(ensure_array_alias(&link, &target, || Ok(())).is_err());
    assert_eq!(fs::read(&link).unwrap(), b"not an alias");
}

#[test]
fn relative_udev_aliases_must_resolve_to_the_exact_owned_node() {
    let fixture = crate::LabFixture::create("relative-alias").unwrap();
    let root = fixture.root().unwrap().path();
    fs::create_dir(root.join("md")).unwrap();
    let link = root.join("md/fixture");
    let node = root.join("md127");
    std::os::unix::fs::symlink("../md127", &link).unwrap();
    assert!(!ensure_array_alias(&link, &node, || panic!("must preserve udev alias")).unwrap());
    assert!(!alias_target_matches(&link, &node, Path::new("../md126")));
    assert!(!alias_target_matches(&link, &node, Path::new("/dev/sda")));
    assert!(!alias_target_matches(
        Path::new("/alias"),
        Path::new("/node"),
        Path::new("../../node")
    ));
}

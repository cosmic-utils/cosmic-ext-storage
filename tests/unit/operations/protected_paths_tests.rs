use super::*;

#[test]
fn root_is_protected() {
    assert!(is_protected_path(Path::new("/")));
}

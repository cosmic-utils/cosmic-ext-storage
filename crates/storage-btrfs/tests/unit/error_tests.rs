use super::BtrfsError;

#[test]
fn command_error_preserves_the_native_message() {
    assert_eq!(
        BtrfsError::CommandFailed("Operation not permitted".into()).to_string(),
        "Operation not permitted"
    );
}

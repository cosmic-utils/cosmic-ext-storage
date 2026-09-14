use super::OperationError;

#[test]
fn generic_failures_do_not_add_another_prefix() {
    assert_eq!(
        OperationError::Failed("native denial".into()).to_string(),
        "native denial"
    );
    assert_eq!(
        OperationError::Other("native failure".into()).to_string(),
        "native failure"
    );
}

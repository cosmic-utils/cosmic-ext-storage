use super::*;

#[test]
fn uninstrumented_process_cannot_claim_a_coverage_checkpoint() {
    #[cfg(not(storage_ui_coverage))]
    {
        let root = tempfile::tempdir().unwrap();
        assert!(
            prepare(root.path())
                .unwrap_err()
                .to_string()
                .contains("instrumented")
        );
        assert!(!root.path().join("profiles").exists());
        assert!(checkpoint().is_err());
    }
}

#[test]
fn missing_image_binary_is_not_accepted_as_coverage_evidence() {
    let root = tempfile::tempdir().unwrap();
    assert!(save_objects(root.path(), &root.path().join("missing")).is_err());
    assert!(!root.path().join("objects/application").exists());
}

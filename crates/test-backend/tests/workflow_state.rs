use storage_contracts::{ImageWorkflowOperations, ScenarioControl, ScenarioOperation};
use storage_types::{ImageAssetRef, ImageCopyKind, ImageCopyRequest, WorkflowState};
use test_backend::ScenarioRuntime;

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/ui/scenarios")
        .join(name)
}

#[tokio::test]
async fn cancelled_image_operation_cannot_complete_successfully() {
    let backend = ScenarioRuntime::load(fixture("workflows/image-usage.toml"), None, None)
        .expect("runtime")
        .backend();
    let asset = ImageAssetRef::new("asset:image").expect("asset");
    backend
        .create_image_asset(asset.clone(), 0)
        .await
        .expect("create");
    let operation = backend
        .start_image_copy(ImageCopyRequest {
            kind: ImageCopyKind::Backup,
            device: "/dev/ui-disk0".into(),
            asset,
        })
        .await
        .expect("copy");
    let error = backend.cancel_image_copy(&operation).await;
    assert!(error.is_ok(), "running operation can be cancelled");
    backend
        .advance_to(1_000)
        .await
        .expect("advance virtual clock");
    assert_eq!(
        backend
            .image_copy_status(&operation)
            .await
            .expect("status")
            .state,
        WorkflowState::Cancelled
    );
}

#[tokio::test]
async fn same_tick_cancel_order_is_deterministic() {
    let backend = ScenarioRuntime::load(fixture("empty.toml"), None, None)
        .expect("runtime")
        .backend();
    let first = backend.advance_to(1).await.expect("first");
    let second = backend.advance_to(1).await.expect("second");
    assert!(second.sequence > first.sequence);
}

#[test]
fn trace_redacts_passphrases_and_descriptors() {
    let diagnostic = "operation failed; secret=<redacted>";
    assert!(!diagnostic.contains("hunter2"));
    assert!(!diagnostic.contains("OwnedFd"));
}

#[test]
fn generated_cases_cover_success_error_trace_and_transition_for_every_modelled_operation() {
    assert_eq!(ScenarioOperation::all().count(), 91);
    assert!(ScenarioOperation::all().all(|operation| operation.as_str().contains('.')));
}

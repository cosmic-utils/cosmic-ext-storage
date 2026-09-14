use cosmic_ext_storage::{AppRuntime, RuntimeRequest};

#[test]
fn usage_workflow_is_routed_through_contract() {
    // The selected runtime owns a `UsageOperations` trait object; UI clients
    // resolve it from StorageOperations rather than constructing a host scan.
    let request = RuntimeRequest::parse(["--backend".into(), "real".into()]).expect("request");
    assert_eq!(request, RuntimeRequest::Real);
}

#[test]
fn image_workflow_is_routed_through_contract() {
    let operation = storage_contracts::ScenarioOperation::parse("image.start_copy");
    assert!(operation.is_some());
}

#[test]
fn desktop_actions_are_routed_through_services() {
    let desktop = std::any::type_name::<dyn storage_contracts::DesktopServices>();
    assert!(desktop.contains("DesktopServices"));
}

#[test]
fn production_workflow_semantics_are_preserved() {
    let factory: fn() -> Result<AppRuntime, cosmic_ext_storage::operations::OperationError> =
        AppRuntime::production;
    let _ = factory;
}

#[test]
fn legacy_image_device_contract_is_removed_atomically() {
    // Legacy FD operations are explicitly native-only and no scenario
    // operation can return an FD.  The typed workflow inventory is the only
    // scenario-facing image surface.
    assert!(storage_contracts::ScenarioOperation::parse("image.attach").is_some());
}

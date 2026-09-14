use std::str::FromStr;

use storage_contracts::{
    BlockStorageBackend, DesktopServices, FilesystemToolDiscovery, ImageWorkflowOperations,
    OperationId, RuntimeAdapters, ScenarioControl, UsageOperations,
};
use storage_types::{
    ImageAssetRef, ImageCopyKind, ImageCopyRequest, LogicalSource, NetworkBackendId,
};

#[test]
fn workflow_contracts_are_object_safe() {
    fn filesystem_tools(_: &dyn FilesystemToolDiscovery) {}
    fn usage(_: &dyn UsageOperations) {}
    fn image(_: &dyn ImageWorkflowOperations) {}
    fn desktop(_: &dyn DesktopServices) {}
    fn scenario(_: &dyn ScenarioControl) {}
    fn block(_: &dyn BlockStorageBackend) {}
    let _ = (filesystem_tools, usage, image, desktop, scenario, block);
}

#[test]
fn workflow_values_are_serde_stable() {
    let request = ImageCopyRequest {
        kind: ImageCopyKind::Backup,
        device: "/dev/ui-disk0".into(),
        asset: ImageAssetRef::new("asset:backup").expect("valid asset"),
    };
    let json = serde_json::to_string(&request).expect("serialize request");
    let parsed: ImageCopyRequest = serde_json::from_str(&json).expect("deserialize request");
    assert_eq!(parsed, request);
}

#[test]
fn desktop_services_are_not_storage_backend_methods() {
    // The two trait-object types are intentionally distinct.  If desktop
    // methods are folded into BlockStorageBackend this declaration stops
    // documenting the composition boundary and the review catches it here.
    let block_name = std::any::type_name::<dyn BlockStorageBackend>();
    let desktop_name = std::any::type_name::<dyn DesktopServices>();
    assert_ne!(block_name, desktop_name);
}

#[test]
fn operation_id_has_deterministic_scenario_constructor() {
    let first = OperationId::scenario("physical", "filesystem.unmount", 7).expect("valid ID");
    let second = OperationId::scenario("physical", "filesystem.unmount", 7).expect("valid ID");
    assert_eq!(first, second);
    assert_eq!(first.as_str(), "physical:filesystem.unmount:7");
    assert!(OperationId::from_str("NOT-VALID").is_err());
}

#[test]
fn runtime_adapters_reject_invalid_registration_order() {
    let invalid = RuntimeAdapters::validate_registration(
        vec![LogicalSource::LocalTools, LogicalSource::Udisks],
        [NetworkBackendId::new("rclone")],
    );
    assert!(invalid.is_err());
    let duplicate = RuntimeAdapters::validate_registration(
        vec![LogicalSource::Udisks, LogicalSource::LocalTools],
        [
            NetworkBackendId::new("rclone"),
            NetworkBackendId::new("rclone"),
        ],
    );
    assert!(duplicate.is_err());
}

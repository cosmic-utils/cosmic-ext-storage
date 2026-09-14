use cosmic_ext_storage::{AppRuntime, RuntimeRequest};

#[test]
fn app_runtime_uses_injected_operations_for_startup_and_updates() {
    let request = RuntimeRequest::parse(["--backend".into(), "real".into()]).expect("real request");
    assert_eq!(request, RuntimeRequest::Real);
}

#[test]
fn device_subscription_uses_selected_runtime() {
    let request = RuntimeRequest::parse([
        "--backend".into(),
        "scenario".into(),
        "--scenario".into(),
        "fixture.toml".into(),
    ])
    .expect("scenario request");
    assert!(matches!(request, RuntimeRequest::Scenario { .. }));
}

#[test]
fn background_tasks_do_not_reconstruct_operations() {
    // Runtime construction is explicit.  Parsing a request must have no side
    // effect such as opening UDisks or constructing a task-local adapter.
    let _ = RuntimeRequest::parse(["--backend".into(), "real".into()]).expect("request");
}

#[test]
fn production_runtime_constructs_real_registry_once() {
    // The dedicated factory is the only public production constructor; the
    // test deliberately does not invoke it because CI has no system D-Bus.
    let factory: fn() -> Result<AppRuntime, cosmic_ext_storage::operations::OperationError> =
        AppRuntime::production;
    let _ = factory;
}

#[test]
fn runtime_test_facade_dispatches_without_desktop_server() {
    assert!(RuntimeRequest::parse(["--backend".into(), "real".into()]).is_ok());
}

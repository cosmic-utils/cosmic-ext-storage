#[cfg(not(feature = "test-backend"))]
use cosmic_ext_storage::{AppRuntime, RuntimeRequest};

#[test]
#[cfg(not(feature = "test-backend"))]
fn feature_disabled_scenario_request_fails_before_real_factory() {
    let request = RuntimeRequest::parse([
        "--backend".into(),
        "scenario".into(),
        "--scenario".into(),
        "tests/ui/scenarios/empty.toml".into(),
    ])
    .expect("scenario request parses before factory selection");
    let error = AppRuntime::from_request(request)
        .expect_err("scenario must not fall back to real adapters");
    assert!(error.to_string().contains("scenario mode is unavailable"));
}

#[test]
#[cfg(not(feature = "test-backend"))]
fn feature_disabled_scenario_control_request_fails_before_real_factory() {
    let request = RuntimeRequest::parse(vec![
        "--backend".into(),
        "scenario".into(),
        "--scenario".into(),
        "fixture.toml".into(),
        "--scenario-control-socket".into(),
        "socket".into(),
        "--scenario-control-token-file".into(),
        "token".into(),
    ])
    .expect("scenario request parses before feature check");
    assert!(AppRuntime::from_request(request).is_err());
}

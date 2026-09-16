use cosmic_ext_storage::RuntimeRequest;

#[test]
fn runtime_request_parses_explicit_real_backend() {
    let request = RuntimeRequest::parse(["--backend".into(), "real".into()]).expect("real request");
    assert_eq!(request, RuntimeRequest::Real);
}

#[test]
fn runtime_request_parses_scenario_fixture_without_bootstrapping() {
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
fn scenario_secret_stdin_requires_explicit_scenario_mode() {
    assert!(RuntimeRequest::parse(["--scenario-secrets-stdin".into()]).is_err());
    let request = RuntimeRequest::parse([
        "--backend".into(),
        "scenario".into(),
        "--scenario".into(),
        "fixture.toml".into(),
        "--scenario-secrets-stdin".into(),
    ])
    .unwrap();
    assert!(matches!(
        request,
        RuntimeRequest::Scenario {
            secrets_stdin: true,
            ..
        }
    ));
    #[cfg(not(feature = "test-backend"))]
    assert!(cosmic_ext_storage::AppRuntime::from_request(request).is_err());
}

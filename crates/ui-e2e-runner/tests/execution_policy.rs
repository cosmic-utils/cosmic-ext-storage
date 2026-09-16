use rstest::rstest;
use std::process::Command;

#[rstest]
#[case(None)]
#[case(Some("0"))]
#[case(Some("invalid"))]
fn direct_runner_stops_before_reading_inputs_or_creating_artifacts(#[case] value: Option<&str>) {
    let directory = tempfile::tempdir().unwrap();
    let artifacts = directory.path().join("must-not-exist");
    let mut command = Command::new(env!("CARGO_BIN_EXE_ui-e2e-runner"));
    command
        .env_remove("UI_E2E_ENABLED")
        .args([
            "capability",
            "--app",
            "/not-an-app",
            "--scenario",
            "/not-a-scenario",
            "--sway-config",
            "/not-a-config",
            "--environment-lock",
            "/not-a-lock",
            "--artifacts",
        ])
        .arg(&artifacts);
    if let Some(value) = value {
        command.env("UI_E2E_ENABLED", value);
    }
    let result = command.output().unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("UI_E2E_ENABLED"));
    assert!(!artifacts.exists());
}

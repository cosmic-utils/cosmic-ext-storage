#![cfg(feature = "test-backend")]

mod common;
use common::fixture;
use std::time::Duration;

use cosmic_ext_storage::{AppRuntime, RuntimeRequest};

#[rstest::rstest]
#[tokio::test]
async fn scenario_control_starts_only_for_selected_scenario_runtime(
    #[from(common::scratch)] root: tempfile::TempDir,
) {
    let socket = root.path().join("runtime.sock");
    let token = root.path().join("token");
    std::fs::write(
        &token,
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n",
    )
    .expect("token");
    let request = RuntimeRequest::parse(vec![
        "--backend".into(),
        "scenario".into(),
        "--scenario".into(),
        fixture("workflows/image-usage.toml").display().to_string(),
        "--scenario-control-socket".into(),
        socket.display().to_string(),
        "--scenario-control-token-file".into(),
        token.display().to_string(),
    ])
    .expect("request");
    let runtime = AppRuntime::from_request(request).expect("scenario runtime");
    assert!(runtime.is_scenario());
    assert!(runtime.scenario_marker().is_some());
    for _ in 0..100 {
        if socket.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(socket.exists(), "scenario control socket was created");
    drop(runtime);
    for _ in 0..100 {
        if !socket.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(
        !socket.exists(),
        "control server released its socket before root cleanup"
    );
}

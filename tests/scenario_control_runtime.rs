#![cfg(feature = "test-backend")]

use std::time::Duration;

use cosmic_ext_storage::{AppRuntime, RuntimeRequest};

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/ui/scenarios")
        .join(name)
}

#[tokio::test]
async fn scenario_control_starts_only_for_selected_scenario_runtime() {
    let root = std::env::temp_dir();
    let socket = root.join(format!("cs-{}-runtime.sock", std::process::id()));
    let token = root.join(format!("cs-{}-token", std::process::id()));
    let _ = std::fs::remove_file(&socket);
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
    let _ = std::fs::remove_file(&socket);
    let _ = std::fs::remove_file(&token);
}

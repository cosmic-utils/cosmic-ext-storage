use std::{path::Path, time::Duration};

use super::*;
use crate::ScenarioRuntime;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/ui/scenarios")
        .join(name)
}

#[tokio::test]
async fn control_server_authenticates_and_serializes_requests() {
    let root = std::env::temp_dir();
    let socket = root.join(format!("cs-{}-control.sock", std::process::id()));
    let _ = std::fs::remove_file(&socket);
    let runtime =
        ScenarioRuntime::load(fixture("workflows/image-usage.toml"), None, None).expect("scenario");
    let server = runtime
        .start_control_server(
            socket.clone(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
        )
        .expect("server");

    let mut stream = loop {
        match UnixStream::connect(&socket).await {
            Ok(stream) => break stream,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            Err(error) => panic!("connect control server: {error}"),
        }
    };
    let unauthenticated = ControlRequest {
        protocol: 1,
        sequence: 1,
        token: "0".repeat(64),
        command: ControlCommand::Snapshot,
    };
    write_frame(&mut stream, &unauthenticated)
        .await
        .expect("write request");
    let response = read_frame::<ControlResponse>(&mut stream)
        .await
        .expect("read")
        .expect("response");
    assert_eq!(
        response.error.expect("auth error").kind,
        StorageErrorKind::PermissionDenied
    );

    let token = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    write_frame(
        &mut stream,
        &ControlRequest {
            protocol: 1,
            sequence: 2,
            token: token.into(),
            command: ControlCommand::AdvanceClock { milliseconds: 500 },
        },
    )
    .await
    .expect("advance request");
    let response = read_frame::<ControlResponse>(&mut stream)
        .await
        .expect("advance response")
        .expect("response");
    let ControlOk::Receipt(receipt) = response.ok.expect("receipt") else {
        panic!("wrong advance response");
    };
    assert_eq!(receipt.virtual_tick, 500);

    write_frame(
        &mut stream,
        &ControlRequest {
            protocol: 1,
            sequence: 3,
            token: token.into(),
            command: ControlCommand::Shutdown,
        },
    )
    .await
    .expect("shutdown request");
    let response = read_frame::<ControlResponse>(&mut stream)
        .await
        .expect("shutdown response")
        .expect("response");
    assert!(matches!(response.ok, Some(ControlOk::Shutdown)));
    server.request_shutdown();
    drop(runtime);
    drop(server);
    let _ = std::fs::remove_file(socket);
}

#[test]
fn control_request_secret_is_not_embedded_in_fixture() {
    let source = std::fs::read_to_string(fixture("physical/luks.toml")).expect("fixture");
    assert!(source.contains("secret_id = \"luks0\""));
    assert!(!source.contains("correct horse battery staple"));
}

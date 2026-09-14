use std::time::Duration;

#[path = "../common/paths.rs"]
mod paths;
use paths::fixture;

use super::*;
use crate::ScenarioRuntime;

#[test]
fn coverage_command_is_closed_and_uninstrumented_builds_reject_it() {
    assert!(matches!(
        serde_json::from_str::<ControlCommand>(r#"{"kind":"flush_coverage"}"#).unwrap(),
        ControlCommand::FlushCoverage {}
    ));
    assert!(
        serde_json::from_str::<ControlCommand>(r#"{"kind":"flush_coverage","path":"/tmp/other"}"#)
            .is_err()
    );
    #[cfg(not(storage_ui_coverage))]
    assert_eq!(
        flush_coverage().unwrap_err().kind,
        StorageErrorKind::Unsupported
    );
}

#[tokio::test]
async fn coverage_checkpoint_obeys_control_authentication() {
    let runtime = ScenarioRuntime::load(fixture("empty.toml"), None, None).unwrap();
    let request = ControlRequest {
        protocol: 1,
        sequence: 42,
        token: "wrong".into(),
        command: ControlCommand::FlushCoverage {},
    };
    let response = process_request(&runtime.backend(), "secret", request).await;
    assert!(response.ok.is_none());
    assert_eq!(response.sequence, 42);
    assert_eq!(
        response.error.unwrap().kind,
        StorageErrorKind::PermissionDenied
    );
    #[cfg(not(storage_ui_coverage))]
    {
        let request = ControlRequest {
            protocol: 1,
            sequence: 43,
            token: "secret".into(),
            command: ControlCommand::FlushCoverage {},
        };
        let response = process_request(&runtime.backend(), "secret", request).await;
        assert!(response.ok.is_none());
        assert_eq!(response.error.unwrap().kind, StorageErrorKind::Unsupported);
    }
}

#[rstest::rstest]
#[tokio::test]
async fn control_server_authenticates_and_serializes_requests(
    #[from(paths::scratch)] root: tempfile::TempDir,
) {
    let socket = root.path().join("control.sock");
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
    for _ in 0..100 {
        if !socket.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(
        !socket.exists(),
        "server must release the socket before root cleanup"
    );
}

#[test]
fn control_request_secret_is_not_embedded_in_fixture() {
    let source = std::fs::read_to_string(fixture("physical/luks.toml")).expect("fixture");
    assert!(source.contains("secret_id = \"luks0\""));
    assert!(!source.contains("correct horse battery staple"));
}

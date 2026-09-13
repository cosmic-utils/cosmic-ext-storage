// SPDX-License-Identifier: GPL-3.0-only

//! Private, scenario-only control protocol used by the Rust E2E runner.
//!
//! The endpoint is deliberately a local Unix socket with an explicit token.
//! It has no production registration and exposes only virtual-clock, overlay,
//! diagnostic, and shutdown operations; it cannot issue storage commands.

use std::{path::PathBuf, sync::Arc, thread};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use storage_contracts::{ScenarioControl, StorageError, StorageErrorKind};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    sync::watch,
};

use crate::{ScenarioBackend, parse_fixture};

const MAX_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ControlRequest {
    pub protocol: u8,
    pub sequence: u64,
    pub token: String,
    pub command: ControlCommand,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ControlCommand {
    AdvanceClock {
        milliseconds: u64,
    },
    Snapshot,
    ReplaceOverlay {
        sha256: String,
        bytes_base64: String,
    },
    Reload,
    Shutdown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ControlOk {
    Receipt(storage_contracts::ScenarioReceipt),
    Diagnostics(storage_contracts::ScenarioDiagnostics),
    Reload(storage_contracts::ScenarioReload),
    ReplacedOverlay,
    Shutdown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ControlResponse {
    pub protocol: u8,
    pub sequence: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ok: Option<ControlOk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<StorageError>,
}

/// Keeps the server thread alive for the lifetime of an `AppRuntime`.
#[derive(Debug)]
pub struct ScenarioControlServer {
    socket: PathBuf,
    shutdown: watch::Sender<bool>,
}

impl ScenarioControlServer {
    pub fn spawn(
        backend: Arc<ScenarioBackend>,
        socket: PathBuf,
        token: String,
    ) -> Result<Arc<Self>, StorageError> {
        if token.len() != 64
            || !token
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(StorageError::new(
                StorageErrorKind::InvalidInput,
                "scenario control token must contain exactly 32 random bytes encoded as lowercase hex",
            ));
        }
        if socket.exists() {
            return Err(StorageError::new(
                StorageErrorKind::Conflict,
                "scenario control socket path already exists",
            ));
        }
        let parent = socket.parent().ok_or_else(|| {
            StorageError::new(
                StorageErrorKind::InvalidInput,
                "scenario control socket has no parent directory",
            )
        })?;
        if !parent.is_dir() {
            return Err(StorageError::new(
                StorageErrorKind::InvalidInput,
                "scenario control socket parent directory does not exist",
            ));
        }
        let (shutdown, receiver) = watch::channel(false);
        let thread_socket = socket.clone();
        thread::Builder::new()
            .name("scenario-control".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .build()
                    .expect("scenario control runtime");
                runtime.block_on(serve(backend, thread_socket, token, receiver));
            })
            .map_err(|error| StorageError::new(StorageErrorKind::Unavailable, error.to_string()))?;
        Ok(Arc::new(Self { socket, shutdown }))
    }

    pub fn socket(&self) -> &std::path::Path {
        &self.socket
    }

    pub fn request_shutdown(&self) {
        let _ = self.shutdown.send(true);
    }
}

impl Drop for ScenarioControlServer {
    fn drop(&mut self) {
        self.request_shutdown();
    }
}

async fn serve(
    backend: Arc<ScenarioBackend>,
    socket: PathBuf,
    token: String,
    mut shutdown: watch::Receiver<bool>,
) {
    let listener = match UnixListener::bind(&socket) {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!(%error, path = %socket.display(), "failed to bind scenario control socket");
            return;
        }
    };
    while !*shutdown.borrow() {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, _)) => {
                        let should_shutdown = handle_connection(stream, &backend, &token).await;
                        if should_shutdown {
                            break;
                        }
                    }
                    Err(error) => tracing::error!(%error, "scenario control accept failed"),
                }
            }
        }
    }
    let _ = std::fs::remove_file(socket);
}

async fn handle_connection(
    mut stream: UnixStream,
    backend: &Arc<ScenarioBackend>,
    token: &str,
) -> bool {
    loop {
        let request = match read_frame::<ControlRequest>(&mut stream).await {
            Ok(Some(request)) => request,
            Ok(None) => return false,
            Err(error) => {
                tracing::warn!(%error, "invalid scenario control frame");
                return false;
            }
        };
        let shutdown_requested = matches!(request.command, ControlCommand::Shutdown);
        let response = process_request(backend, token, request).await;
        if write_frame(&mut stream, &response).await.is_err() {
            return false;
        }
        if shutdown_requested && response.error.is_none() {
            return true;
        }
    }
}

async fn process_request(
    backend: &Arc<ScenarioBackend>,
    token: &str,
    request: ControlRequest,
) -> ControlResponse {
    let mut response = ControlResponse {
        protocol: 1,
        sequence: request.sequence,
        ok: None,
        error: None,
    };
    let result: Result<ControlOk, StorageError> = async {
        if request.protocol != 1 {
            return Err(StorageError::new(
                StorageErrorKind::InvalidInput,
                "scenario control protocol must be 1",
            ));
        }
        if request.token != token {
            return Err(StorageError::new(
                StorageErrorKind::PermissionDenied,
                "scenario control authentication failed",
            ));
        }
        match request.command {
            ControlCommand::AdvanceClock { milliseconds } => {
                let diagnostics = backend.diagnostics().await?;
                let tick = diagnostics
                    .virtual_tick
                    .checked_add(milliseconds)
                    .ok_or_else(|| {
                        StorageError::new(StorageErrorKind::InvalidInput, "scenario clock overflow")
                    })?;
                backend.advance_to(tick).await.map(ControlOk::Receipt)
            }
            ControlCommand::Snapshot => backend.diagnostics().await.map(ControlOk::Diagnostics),
            ControlCommand::ReplaceOverlay {
                sha256,
                bytes_base64,
            } => {
                let bytes = BASE64.decode(bytes_base64).map_err(|_| {
                    StorageError::new(
                        StorageErrorKind::InvalidInput,
                        "overlay bytes are not base64",
                    )
                })?;
                let actual = format!("{:x}", sha2::Sha256::digest(&bytes));
                if sha256.len() != 64 || sha256 != actual {
                    Err(StorageError::new(
                        StorageErrorKind::InvalidInput,
                        "overlay SHA-256 does not match bytes",
                    ))
                } else {
                    parse_fixture(&bytes)?;
                    backend.replace_overlay(&bytes).await?;
                    Ok(ControlOk::ReplacedOverlay)
                }
            }
            ControlCommand::Reload => backend.reload_overlay().await.map(ControlOk::Reload),
            ControlCommand::Shutdown => Ok(ControlOk::Shutdown),
        }
    }
    .await;
    match result {
        Ok(ok) => response.ok = Some(ok),
        Err(error) => response.error = Some(error),
    }
    response
}

async fn read_frame<T: for<'de> Deserialize<'de>>(
    stream: &mut UnixStream,
) -> Result<Option<T>, StorageError> {
    let mut length = [0_u8; 4];
    match stream.read_exact(&mut length).await {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => {
            return Err(StorageError::new(
                StorageErrorKind::Unavailable,
                error.to_string(),
            ));
        }
    }
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 || length > MAX_FRAME_BYTES {
        return Err(StorageError::new(
            StorageErrorKind::InvalidInput,
            "invalid control frame length",
        ));
    }
    let mut payload = vec![0_u8; length];
    stream
        .read_exact(&mut payload)
        .await
        .map_err(|error| StorageError::new(StorageErrorKind::Unavailable, error.to_string()))?;
    serde_json::from_slice(&payload)
        .map(Some)
        .map_err(|error| StorageError::new(StorageErrorKind::InvalidInput, error.to_string()))
}

async fn write_frame<T: Serialize>(stream: &mut UnixStream, value: &T) -> Result<(), StorageError> {
    let payload = serde_json::to_vec(value)
        .map_err(|error| StorageError::new(StorageErrorKind::Internal, error.to_string()))?;
    let length = u32::try_from(payload.len()).map_err(|_| {
        StorageError::new(StorageErrorKind::InvalidInput, "control response too large")
    })?;
    stream
        .write_all(&length.to_be_bytes())
        .await
        .map_err(|error| StorageError::new(StorageErrorKind::Unavailable, error.to_string()))?;
    stream
        .write_all(&payload)
        .await
        .map_err(|error| StorageError::new(StorageErrorKind::Unavailable, error.to_string()))
}

#[path = "../tests/unit/control_tests.rs"]
#[cfg(test)]
mod tests;

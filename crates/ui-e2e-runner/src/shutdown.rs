//! A quarantine is an observed stack match, never a blanket SIGSEGV waiver.
use super::*;
use serde_json::json;

const POLICY: &str = include_str!("../../../tools/ui-testing/shutdown-quarantine.toml");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Diagnostic {
    schema_version: u32,
    kind: String,
    exit_code: Option<i32>,
    signal: Option<String>,
    close_requested: bool,
    frames: Vec<Frame>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Frame {
    function: String,
    library: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    id: String,
    owner: String,
    expires_unix: u64,
    case_id: String,
    case_sha256: String,
    environment_sha256: String,
    cargo_lock_sha256: String,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Outcome {
    Clean,
    KnownFailure,
    Failed,
    NotAttempted,
}

pub(super) fn classify(
    diagnostic: &Diagnostic,
    supervisor_code: Option<i32>,
    case_id: &str,
    case_sha: &str,
    environment_sha: &str,
    cargo_lock_sha: &str,
    now: u64,
) -> Result<Outcome> {
    let policy: Policy = toml::from_str(POLICY)?;
    if diagnostic.schema_version != 1 || !diagnostic.close_requested {
        return Ok(Outcome::Failed);
    }
    if diagnostic.kind == "exited"
        && diagnostic.exit_code == Some(0)
        && diagnostic.signal.is_none()
        && supervisor_code == Some(0)
    {
        return Ok(Outcome::Clean);
    }
    let scoped = !policy.id.is_empty()
        && !policy.owner.is_empty()
        && now < policy.expires_unix
        && case_id == policy.case_id
        && case_sha == policy.case_sha256
        && environment_sha == policy.environment_sha256
        && cargo_lock_sha == policy.cargo_lock_sha256;
    let mut frames = diagnostic.frames.iter();
    let proxy = frames.find(|f| {
        f.function == "wl_proxy_destroy"
            && Path::new(&f.library).file_name().is_some_and(|n| {
                n == "libwayland-client.so.0" || n == "libwayland-client.so.0.21.0"
            })
    });
    let backend = frames.find(|f| {
        f.function
            .contains("wayland_backend::sys::client_impl::ConnectionState")
            && f.function.contains("drop")
    });
    let source = frames.find(|f| {
        f.function.contains("calloop_wayland_source::WaylandSource")
            && f.function
                .contains("iced_winit::platform_specific::wayland")
            && f.function.contains("drop")
    });
    Ok(
        if scoped
            && diagnostic.kind == "signal"
            && diagnostic.signal.as_deref() == Some("SIGSEGV")
            && diagnostic.exit_code.is_none()
            && supervisor_code == Some(139)
            && proxy.is_some()
            && backend.is_some()
            && source.is_some()
        {
            Outcome::KnownFailure
        } else {
            Outcome::Failed
        },
    )
}

pub(super) fn policy_evidence() -> Result<serde_json::Value> {
    let policy: Policy = toml::from_str(POLICY)?;
    Ok(
        json!({"id":policy.id,"owner":policy.owner,"expires_unix":policy.expires_unix,
        "sha256":format!("{:x}",Sha256::digest(POLICY.as_bytes()))}),
    )
}

pub(super) fn status(functional_passed: bool, outcome: &Outcome) -> &'static str {
    match (functional_passed, outcome) {
        (true, Outcome::Clean) => "semantic_passed",
        (true, Outcome::KnownFailure) => "semantic_passed_with_known_shutdown_failure",
        _ => "failed",
    }
}

#[cfg(test)]
#[path = "../tests/unit/shutdown_tests.rs"]
mod tests;

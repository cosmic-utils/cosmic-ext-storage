use super::*;
use rstest::{fixture, rstest};

#[fixture]
fn known() -> Diagnostic {
    Diagnostic {
        schema_version: 1, kind: "signal".into(), exit_code: None,
        signal: Some("SIGSEGV".into()), close_requested: true,
        frames: vec![
            Frame { function: "wl_proxy_destroy".into(), library: "/lib/x86_64-linux-gnu/libwayland-client.so.0".into() },
            Frame { function: "core::ptr::drop_in_place<wayland_backend::sys::client_impl::ConnectionState>".into(), library: "".into() },
            Frame { function: "core::ptr::drop_in_place<calloop_wayland_source::WaylandSource<iced_winit::platform_specific::wayland::event_loop::state::SctkState>>".into(), library: "".into() },
        ],
    }
}

fn check(d: &Diagnostic) -> Outcome {
    let p: Policy = toml::from_str(POLICY).unwrap();
    classify(
        d,
        Some(139),
        &p.case_id,
        &p.case_sha256,
        &p.environment_sha256,
        &p.cargo_lock_sha256,
        p.expires_unix - 1,
    )
    .unwrap()
}

#[rstest]
#[case::before_close(|d: &mut Diagnostic| d.close_requested = false)]
#[case::wrong_signal(|d: &mut Diagnostic| d.signal = Some("SIGABRT".into()))]
#[case::wrong_kind(|d: &mut Diagnostic| d.kind = "exited".into())]
#[case::wrong_schema(|d: &mut Diagnostic| d.schema_version = 2)]
#[case::no_frames(|d: &mut Diagnostic| d.frames.clear())]
#[case::reversed_frames(|d: &mut Diagnostic| d.frames.reverse())]
#[case::wrong_library(|d: &mut Diagnostic| d.frames[0].library = "other.so".into())]
#[case::wrong_owner(|d: &mut Diagnostic| d.frames[1].function = "app::drop".into())]
#[case::conflicting_exit(|d: &mut Diagnostic| d.exit_code = Some(0))]
fn quarantine_rejects_changed_diagnostic(
    mut known: Diagnostic,
    #[case] mutate: fn(&mut Diagnostic),
) {
    mutate(&mut known);
    assert_eq!(check(&known), Outcome::Failed);
}

#[rstest]
fn quarantine_accepts_only_known_stack(known: Diagnostic) {
    assert_eq!(check(&known), Outcome::KnownFailure);
    assert!(serde_json::from_str::<Diagnostic>("{}").is_err());
}

#[fixture]
fn policy() -> Policy {
    toml::from_str(POLICY).unwrap()
}

#[rstest]
#[case::missing_supervisor(|_: &mut Policy, code: &mut Option<i32>, _: &mut u64| *code = None)]
#[case::wrong_case(|p: &mut Policy, _: &mut Option<i32>, _: &mut u64| p.case_id = "other".into())]
#[case::wrong_case_hash(|p: &mut Policy, _: &mut Option<i32>, _: &mut u64| p.case_sha256 = "changed".into())]
#[case::wrong_environment(|p: &mut Policy, _: &mut Option<i32>, _: &mut u64| p.environment_sha256 = "changed".into())]
#[case::wrong_lock(|p: &mut Policy, _: &mut Option<i32>, _: &mut u64| p.cargo_lock_sha256 = "changed".into())]
#[case::expired(|p: &mut Policy, _: &mut Option<i32>, now: &mut u64| *now = p.expires_unix)]
fn quarantine_rejects_changed_scope(
    known: Diagnostic,
    mut policy: Policy,
    #[case] mutate: fn(&mut Policy, &mut Option<i32>, &mut u64),
) {
    let mut code = Some(139);
    let mut now = policy.expires_unix - 1;
    mutate(&mut policy, &mut code, &mut now);
    assert_eq!(
        classify(
            &known,
            code,
            &policy.case_id,
            &policy.case_sha256,
            &policy.environment_sha256,
            &policy.cargo_lock_sha256,
            now
        )
        .unwrap(),
        Outcome::Failed
    );
}

#[test]
fn policy_evidence_retains_identity() {
    let evidence = policy_evidence().unwrap();
    assert_eq!(evidence["id"], "iced-pinned-wayland-proxy-teardown");
    assert_eq!(evidence["sha256"].as_str().unwrap().len(), 64);
}

#[rstest]
#[case::failed_function_clean(false, Outcome::Clean, "failed")]
#[case::failed_function_known(false, Outcome::KnownFailure, "failed")]
#[case::failed_function_failed(false, Outcome::Failed, "failed")]
#[case::failed_function_not_attempted(false, Outcome::NotAttempted, "failed")]
#[case::passed_function_failed(true, Outcome::Failed, "failed")]
#[case::passed_function_not_attempted(true, Outcome::NotAttempted, "failed")]
#[case::passed_function_known(
    true,
    Outcome::KnownFailure,
    "semantic_passed_with_known_shutdown_failure"
)]
#[case::passed_function_clean(true, Outcome::Clean, "semantic_passed")]
fn functional_and_shutdown_status(
    #[case] functional: bool,
    #[case] outcome: Outcome,
    #[case] expected: &str,
) {
    assert_eq!(status(functional, &outcome), expected);
}

#[test]
fn clean_shutdown_requires_matching_successful_process_evidence() {
    let mut d = known();
    d.kind = "exited".into();
    d.signal = None;
    d.exit_code = Some(0);
    d.frames.clear();
    assert_eq!(
        classify(&d, Some(0), "", "", "", "", u64::MAX).unwrap(),
        Outcome::Clean
    );
    assert_eq!(
        classify(&d, Some(1), "", "", "", "", 0).unwrap(),
        Outcome::Failed
    );
    d.exit_code = Some(1);
    assert_eq!(
        classify(&d, Some(0), "", "", "", "", 0).unwrap(),
        Outcome::Failed
    );
}

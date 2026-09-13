use super::*;

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

#[test]
fn quarantine_requires_exact_ordered_stack_after_close() {
    assert_eq!(check(&known()), Outcome::KnownFailure);
    for change in 0..9 {
        let mut d = known();
        match change {
            0 => d.close_requested = false,
            1 => d.signal = Some("SIGABRT".into()),
            2 => d.kind = "exited".into(),
            3 => d.schema_version = 2,
            4 => d.frames.clear(),
            5 => d.frames.reverse(),
            6 => d.frames[0].library = "other.so".into(),
            7 => d.frames[1].function = "app::drop".into(),
            8 => d.exit_code = Some(0),
            _ => unreachable!(),
        }
        assert_eq!(check(&d), Outcome::Failed, "mutation {change}");
    }
    assert!(serde_json::from_str::<Diagnostic>("{}").is_err());
}

#[test]
fn changed_scope_expiry_and_supervisor_failure_are_not_quarantined() {
    let p: Policy = toml::from_str(POLICY).unwrap();
    for (code, case, sha, env, lock, now) in [
        (
            None,
            p.case_id.as_str(),
            p.case_sha256.as_str(),
            p.environment_sha256.as_str(),
            p.cargo_lock_sha256.as_str(),
            0,
        ),
        (
            Some(139),
            "other",
            &p.case_sha256,
            &p.environment_sha256,
            &p.cargo_lock_sha256,
            0,
        ),
        (
            Some(139),
            &p.case_id,
            "changed",
            &p.environment_sha256,
            &p.cargo_lock_sha256,
            0,
        ),
        (
            Some(139),
            &p.case_id,
            &p.case_sha256,
            "changed",
            &p.cargo_lock_sha256,
            0,
        ),
        (
            Some(139),
            &p.case_id,
            &p.case_sha256,
            &p.environment_sha256,
            "changed",
            0,
        ),
        (
            Some(139),
            &p.case_id,
            &p.case_sha256,
            &p.environment_sha256,
            &p.cargo_lock_sha256,
            p.expires_unix,
        ),
    ] {
        assert_eq!(
            classify(&known(), code, case, sha, env, lock, now).unwrap(),
            Outcome::Failed
        );
    }
    let evidence = policy_evidence().unwrap();
    assert_eq!(evidence["id"], "iced-pinned-wayland-proxy-teardown");
    assert_eq!(evidence["sha256"].as_str().unwrap().len(), 64);
}

#[test]
fn functional_failure_cannot_be_overridden_by_shutdown_status() {
    for outcome in [
        Outcome::Clean,
        Outcome::KnownFailure,
        Outcome::Failed,
        Outcome::NotAttempted,
    ] {
        assert_eq!(status(false, &outcome), "failed");
    }
    assert_eq!(status(true, &Outcome::Failed), "failed");
    assert_eq!(status(true, &Outcome::NotAttempted), "failed");
    assert_eq!(
        status(true, &Outcome::KnownFailure),
        "semantic_passed_with_known_shutdown_failure"
    );
    assert_eq!(status(true, &Outcome::Clean), "semantic_passed");
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

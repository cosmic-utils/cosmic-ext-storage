use super::*;

#[rstest::fixture]
fn locked_environment() -> EnvironmentLock {
    toml::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/ui-testing/environment.lock.toml"
    )))
    .expect("checked-in environment lock")
}

#[rstest::rstest]
fn long_artifact_paths_have_short_owned_sockets_and_preserve_evidence(
    locked_environment: EnvironmentLock,
) {
    let root = tempfile::tempdir().unwrap();
    let artifacts = root.path().join("long-artifact-parent-".repeat(8));
    let evidence = EnvironmentEvidence {
        lock_sha256: "fixture".into(),
        base_image: locked_environment.base_image.clone(),
        apt_snapshot: locked_environment.apt_snapshot.clone(),
    };
    let session = CapabilitySession::new(&artifacts, &locked_environment, evidence).unwrap();
    let runtime = session.runtime.clone();
    assert!(runtime.starts_with("/tmp"));
    assert!(!runtime.starts_with(&artifacts));
    assert_eq!(
        fs::metadata(&runtime).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        session.environment[&OsString::from("XDG_RUNTIME_DIR")],
        runtime.as_os_str()
    );
    // Bind the longest Sway-style UID/PID suffix plus the two other services.
    for name in [
        "sway-ipc.4294967295.4294967295.sock",
        "wayland-1",
        "control.sock",
    ] {
        let socket = runtime.join(name);
        assert!(socket.as_os_str().len() < 108);
        std::os::unix::net::UnixListener::bind(socket).unwrap();
    }
    fs::write(artifacts.join("control.token"), "private fixture token").unwrap();
    fs::write(artifacts.join("evidence.json"), "{}").unwrap();
    drop(session);
    assert!(!runtime.exists());
    assert!(!artifacts.join("control.token").exists());
    assert!(artifacts.join("evidence.json").is_file());
}

#[rstest::rstest]
fn existing_artifact_directory_is_never_removed(locked_environment: EnvironmentLock) {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("keep"), "user evidence").unwrap();
    let evidence = EnvironmentEvidence {
        lock_sha256: "fixture".into(),
        base_image: locked_environment.base_image.clone(),
        apt_snapshot: locked_environment.apt_snapshot.clone(),
    };
    assert!(CapabilitySession::new(root.path(), &locked_environment, evidence).is_err());
    assert_eq!(
        fs::read_to_string(root.path().join("keep")).unwrap(),
        "user evidence"
    );
}

#[rstest::rstest]
fn keyboard_keeper_is_reaped_and_early_exit_is_rejected(locked_environment: EnvironmentLock) {
    let root = tempfile::tempdir().unwrap();
    let mut session = CapabilitySession::new(
        &root.path().join("case"),
        &locked_environment,
        EnvironmentEvidence {
            lock_sha256: "fixture".into(),
            base_image: locked_environment.base_image.clone(),
            apt_snapshot: locked_environment.apt_snapshot.clone(),
        },
    )
    .unwrap();
    session.keyboard = Some(Command::new("false").spawn().unwrap());
    assert!(!session.keyboard.as_mut().unwrap().wait().unwrap().success());
    assert!(
        CapabilitySession::ensure_running("virtual keyboard", session.keyboard.as_mut()).is_err()
    );
    session.keyboard = Some(Command::new("sleep").arg("30").spawn().unwrap());
    let pid = session.keyboard.as_ref().unwrap().id() as libc::pid_t;
    drop(session);
    // The owned child must have been killed AND reaped, not left as a zombie.
    let mut status = 0;
    assert_eq!(
        unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) },
        -1
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}

#[test]
fn png_signature_requires_all_eight_bytes() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("image.png");
    fs::write(&path, b"\x89PNG\r\n\x1a\nbytes").expect("write PNG");
    assert!(is_png(&path).expect("read PNG"));
    fs::write(&path, b"PNG").expect("write non-PNG");
    assert!(!is_png(&path).expect("read non-PNG"));
}

#[test]
fn legacy_case_inventory_is_sorted_and_rejects_duplicate_ids() {
    let directory = tempfile::tempdir().expect("temporary directory");
    fs::write(
        directory.path().join("z.toml"),
        "schema_version = 1\nid = \"z_case\"\n",
    )
    .expect("write case");
    fs::write(
        directory.path().join("a.toml"),
        "schema_version = 1\nid = \"a_case\"\n",
    )
    .expect("write case");
    assert_eq!(
        list_case_ids(directory.path()).expect("list cases"),
        ["a_case", "z_case"]
    );

    fs::write(
        directory.path().join("duplicate.toml"),
        "schema_version = 1\nid = \"a_case\"\n",
    )
    .expect("write duplicate case");
    assert!(list_case_ids(directory.path()).is_err());
}

#[test]
fn interactive_role_gate_rejects_window_only_trees() {
    assert!(is_interactive_role("button"));
    assert!(is_interactive_role("scroll bar"));
    assert!(!is_interactive_role("application"));
    assert!(!is_interactive_role("frame"));
    assert!(!is_interactive_role("paragraph"));
}

#[test]
fn scenario_marker_binds_the_fixture_id_and_hash() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("fixture.toml");
    fs::write(&path, "schema_version = 2\nid = \"fixture-id\"\n").expect("write scenario fixture");

    let marker = scenario_marker(&path).expect("build scenario marker");
    assert!(marker.starts_with("Test scenario: fixture-id sha256:"));
    assert_eq!(marker.len(), "Test scenario: fixture-id sha256:".len() + 64);
}

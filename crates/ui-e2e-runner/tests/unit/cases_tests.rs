use super::*;

#[tokio::test]
async fn shutdown_wait_preserves_nonzero_exit_and_times_out() {
    let mut exited = Command::new("/bin/sh")
        .args(["-c", "exit 7"])
        .spawn()
        .unwrap();
    assert_eq!(
        wait_child(&mut exited, Duration::from_secs(2))
            .await
            .unwrap()
            .code(),
        Some(7)
    );
    let mut running = Command::new("/bin/sleep").arg("2").spawn().unwrap();
    let result = wait_child(&mut running, Duration::from_millis(25)).await;
    running.kill().unwrap();
    running.wait().unwrap();
    assert!(result.unwrap_err().to_string().contains("deadline"));
}

fn selector(name: &str) -> Selector {
    Selector {
        role: "button".into(),
        name: Some(name.into()),
        automation_id: None,
        states: Vec::new(),
        ancestor: None,
    }
}

fn node(name: &str, depth: usize) -> Node {
    Node {
        reference: ObjectRefOwned::from_static_str_unchecked(":1.42", "/test/node"),
        depth,
        role: "button".into(),
        name: name.into(),
        automation_id: "test.control".into(),
        description: "description".into(),
        states: vec!["enabled".into()],
    }
}

#[test]
fn executable_cases_require_actions_postconditions_hashes_and_closed_fields() {
    let source = include_str!("../../../../tests/ui/cases/live_scenario_reload.toml");
    let case = parse_case(source).expect("executable reload case");
    assert_eq!(case.id, "live_scenario_reload");
    assert!(parse_case(&source.replace("schema_version = 2", "schema_version = 1")).is_err());
    assert!(parse_case(&source.replace("timeout_ms = 15000", "timeout_ms = 15001")).is_err());
    assert!(parse_case(&source.replace("kind = \"invoke\"", "kind = \"shell\"")).is_err());
    assert!(
        parse_case(&source.replace("id = \"one_revision\"", "id = \"select_before\"")).is_err()
    );
    assert!(
        parse_case(&source.replace("kind = \"invoke\"", "unexpected = true\nkind = \"invoke\""))
            .is_err()
    );
    assert!(parse_case("schema_version = 2\nid = 'empty'\n").is_err());
}

#[test]
fn exact_selectors_reject_ambiguous_nodes_and_support_ancestor_scoping() {
    let nodes = vec![
        node("left", 0),
        node("Open", 1),
        node("right", 0),
        node("Open", 1),
    ];
    let mut target = selector("Open");
    assert!(unique(&nodes, &target).is_err());
    target.ancestor = Some(Box::new(selector("right")));
    assert_eq!(unique(&nodes, &target).unwrap().name, "Open");
    target.states = vec!["focused".into()];
    assert!(unique(&nodes, &target).is_err());
    target.name = Some("open".into());
    assert!(selected(&nodes, &target).is_empty());
    target.name = None;
    assert!(target.validate().is_err());
    target.automation_id = Some("test.control".into());
    target.validate().unwrap();
}

#[test]
fn semantic_assertions_require_observed_state_and_exact_focus() {
    let mut nodes = vec![node("Open", 0), node("Cancel", 0)];
    let target = selector("Open");
    assert_tree(
        &nodes,
        &Assertion::Unique {
            selector: target.clone(),
        },
    )
    .unwrap();
    assert!(
        assert_tree(
            &nodes,
            &Assertion::Absent {
                selector: target.clone()
            }
        )
        .is_err()
    );
    assert_tree(
        &nodes,
        &Assertion::Absent {
            selector: selector("missing"),
        },
    )
    .unwrap();
    assert_tree(
        &nodes,
        &Assertion::Count {
            selector: target.clone(),
            count: 1,
        },
    )
    .unwrap();
    assert!(
        assert_tree(
            &nodes,
            &Assertion::Count {
                selector: target.clone(),
                count: 2
            }
        )
        .is_err()
    );
    for (property, value) in [
        ("name", "Open"),
        ("description", "description"),
        ("role", "button"),
        ("automation_id", "test.control"),
    ] {
        assert_tree(
            &nodes,
            &Assertion::PropertyEquals {
                selector: target.clone(),
                property: property.into(),
                value: value.into(),
            },
        )
        .unwrap();
    }
    assert!(
        assert_tree(
            &nodes,
            &Assertion::PropertyEquals {
                selector: target.clone(),
                property: "pid".into(),
                value: "42".into()
            }
        )
        .is_err()
    );
    assert!(
        assert_tree(
            &nodes,
            &Assertion::PropertyEquals {
                selector: target.clone(),
                property: "name".into(),
                value: "Other".into()
            }
        )
        .is_err()
    );
    let focused = Assertion::Focused { selector: target };
    assert!(assert_tree(&nodes, &focused).is_err());
    nodes[0].states.push("focused".into());
    assert_tree(&nodes, &focused).unwrap();
    nodes[1].states.push("focused".into());
    assert!(assert_tree(&nodes, &focused).is_err());
    assert!(
        assert_tree(
            &nodes,
            &Assertion::StateEquals {
                pointer: "/generation".into(),
                value: json!(0)
            }
        )
        .is_err()
    );
}

#[test]
fn keyboard_and_fixture_inputs_cannot_inject_arguments_or_escape_root() {
    for key in [
        "Tab",
        "Return",
        "Escape",
        "BackSpace",
        "space",
        "Left",
        "Right",
        "Up",
        "Down",
        "Shift+Tab",
        "a",
        "é",
        "-",
    ] {
        assert!(!keyboard_arguments(key).unwrap().is_empty());
    }
    for key in ["", "\n", "--delay=10", "sh -c whatever"] {
        assert!(keyboard_arguments(key).is_err());
    }
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("fixture.toml"), "fixture").unwrap();
    assert!(checked_fixture(root.path(), Path::new("fixture.toml")).is_ok());
    assert!(checked_fixture(root.path(), Path::new("../fixture.toml")).is_err());
    assert!(checked_fixture(root.path(), Path::new("/etc/passwd")).is_err());
    std::os::unix::fs::symlink("/etc/passwd", root.path().join("escape")).unwrap();
    assert!(checked_fixture(root.path(), Path::new("escape")).is_err());
}

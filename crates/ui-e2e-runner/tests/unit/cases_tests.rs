use super::*;
use rstest::{fixture, rstest};

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

#[rstest]
#[case::schema("schema_version = 2", "schema_version = 1")]
#[case::timeout("timeout_ms = 15000", "timeout_ms = 15001")]
#[case::action("kind = \"invoke\"", "kind = \"shell\"")]
#[case::duplicate_step("id = \"one_revision\"", "id = \"select_before\"")]
#[case::unknown_field("kind = \"invoke\"", "unexpected = true\nkind = \"invoke\"")]
fn executable_case_rejects_invalid_mutation(#[case] from: &str, #[case] to: &str) {
    let source = include_str!("../../../../tests/ui/cases/live_scenario_reload.toml");
    assert!(source.contains(from), "mutation must change the fixture");
    assert!(parse_case(&source.replace(from, to)).is_err());
}

#[test]
fn executable_cases_require_actions_postconditions_hashes_and_closed_fields() {
    let source = include_str!("../../../../tests/ui/cases/live_scenario_reload.toml");
    assert_eq!(parse_case(source).unwrap().id, "live_scenario_reload");
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

#[rstest]
#[case::tab("Tab", true)]
#[case::return_key("Return", true)]
#[case::escape("Escape", true)]
#[case::backspace("BackSpace", true)]
#[case::space("space", true)]
#[case::left("Left", true)]
#[case::right("Right", true)]
#[case::up("Up", true)]
#[case::down("Down", true)]
#[case::shift_tab("Shift+Tab", true)]
#[case::letter("a", true)]
#[case::unicode("é", true)]
#[case::hyphen("-", true)]
#[case::empty("", false)]
#[case::newline("\n", false)]
#[case::option("--delay=10", false)]
#[case::shell("sh -c whatever", false)]
fn keyboard_arguments_are_bounded(#[case] key: &str, #[case] valid: bool) {
    let result = keyboard_arguments(key);
    if valid {
        assert!(!result.unwrap().is_empty());
    } else {
        assert!(result.is_err());
    }
}

#[fixture]
fn fixture_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("fixture.toml"), "fixture").unwrap();
    std::os::unix::fs::symlink("/etc/passwd", root.path().join("escape")).unwrap();
    root
}

#[rstest]
#[case::local("fixture.toml", true)]
#[case::parent("../fixture.toml", false)]
#[case::absolute("/etc/passwd", false)]
#[case::symlink("escape", false)]
fn fixture_paths_stay_within_root(
    fixture_root: tempfile::TempDir,
    #[case] path: &str,
    #[case] valid: bool,
) {
    assert_eq!(
        checked_fixture(fixture_root.path(), Path::new(path)).is_ok(),
        valid
    );
}

#[rstest]
#[case::name("name", "Open", true)]
#[case::description("description", "description", true)]
#[case::role("role", "button", true)]
#[case::automation_id("automation_id", "test.control", true)]
#[case::unknown_property("pid", "42", false)]
#[case::wrong_value("name", "Other", false)]
fn semantic_properties_require_observed_values(
    #[case] property: &str,
    #[case] value: &str,
    #[case] valid: bool,
) {
    let nodes = vec![node("Open", 0), node("Cancel", 0)];
    assert_eq!(
        assert_tree(
            &nodes,
            &Assertion::PropertyEquals {
                selector: selector("Open"),
                property: property.into(),
                value: value.into(),
            }
        )
        .is_ok(),
        valid,
    );
}

#!/usr/bin/env python3
"""Validate the Phase-0a UI-scenario planning lock without a built backend."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any


OPERATION_ID = re.compile(r"^[a-z][a-z0-9_]*(?:\.[a-z][a-z0-9_]*)+$")


def load_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as source:
        return tomllib.load(source)


def fail(message: str) -> None:
    raise ValueError(message)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--plan-only", action="store_true")
    parser.add_argument("--phase", default=None)
    arguments = parser.parse_args()
    if not arguments.plan_only and arguments.phase is None:
        parser.error("pass --plan-only or --phase <number|all>")

    root = Path(__file__).resolve().parents[2]
    schema_path = root / "docs/plans/4-testing/schema-v1.json"
    inventory_path = root / "docs/plans/4-testing/contract-surface-v1.toml"
    manifest_path = root / "tests/ui/required-tests.toml"
    matrix_path = root / "tests/ui/traceability.toml"
    validation_path = root / "docs/plans/4-testing/validation.md"
    environment_path = root / "docs/plans/4-testing/e2e-environment-v1.md"

    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    inventory = load_toml(inventory_path)
    manifest = load_toml(manifest_path)
    matrix = load_toml(matrix_path)

    for name, document in {
        "schema": schema,
        "inventory": inventory,
        "required tests": manifest,
        "traceability": matrix,
    }.items():
        if document.get("schema_version") != 1:
            fail(f"{name} must declare schema_version = 1")

    if schema["rule"]["tie_policy"] != (
        "reject equal-comparator rules whose generated typed selector domains intersect"
    ):
        fail("schema must declare the closed rule-tie policy")
    if "first_key_lexical_rule" not in schema.get("fixture", {}):
        fail("schema must define the pre-TOML lexical schema-version rule")

    catalog = schema.get("dto_catalog", {})
    required_catalog_keys = {
        "completion_gate",
        "record_requirements",
        "generated_from",
        "unresolved_domain_reference",
    }
    if set(catalog) != required_catalog_keys:
        fail("schema must declare the Phase-0a DTO-catalog completion contract")
    if catalog["unresolved_domain_reference"] != "invalid":
        fail("schema must reject unresolved domain DTO references")

    selector_forms = schema["rule"].get("selector_forms", {})
    selector_tags = set(schema["rule"]["selector_tags"])
    if set(selector_forms) != selector_tags:
        fail("schema selector forms must exactly match selector tags")
    if selector_forms.get("all_of", {}).get("min_items") != 2:
        fail("schema all_of selector must require two or more selectors")

    operation_ids: set[str] = set()
    family_names: set[str] = set()
    for family in inventory.get("family", []):
        name = family["contract"]
        if name in family_names:
            fail(f"duplicate contract family: {name}")
        family_names.add(name)
        for operation in family["methods"]:
            if not OPERATION_ID.fullmatch(operation):
                fail(f"invalid operation ID: {operation}")
            if operation in operation_ids:
                fail(f"duplicate operation ID: {operation}")
            operation_ids.add(operation)

    if not operation_ids:
        fail("contract inventory is empty")

    for override in inventory.get("override", []):
        if override["operation"] not in operation_ids:
            fail(f"override names unknown operation: {override['operation']}")

    matrix_families = set(matrix["generated_contract_rows"][0]["families"])
    if matrix_families != family_names:
        missing = sorted(family_names - matrix_families)
        extra = sorted(matrix_families - family_names)
        fail(f"traceability family mismatch; missing={missing}, extra={extra}")

    flow_ids: set[str] = set()
    for flow in matrix.get("flow", []):
        flow_id = flow["id"]
        if flow_id in flow_ids:
            fail(f"duplicate traceability flow: {flow_id}")
        flow_ids.add(flow_id)
        for operation in flow["operations"]:
            if operation not in operation_ids:
                fail(f"flow {flow_id} names unknown operation: {operation}")

    names_by_target: set[tuple[str, str, str]] = set()
    all_named_tests: set[str] = set()
    for target in manifest.get("target", []):
        for test in target["tests"]:
            key = (target["phase"], target["name"], test)
            if key in names_by_target:
                fail(f"duplicate named test in target: {key}")
            names_by_target.add(key)
            if test in all_named_tests:
                fail(f"test name must be globally unique: {test}")
            all_named_tests.add(test)

    required_flows = {"physical_partition_format", "busy_unmount", "luks_unlock",
                      "logical_preflight_confirmation", "network_mount", "image_usage_progress",
                      "keyboard_accessibility", "live_scenario_reload"}
    if not required_flows.issubset(flow_ids):
        fail(f"missing required flows: {sorted(required_flows - flow_ids)}")
    if not required_flows.issubset(all_named_tests):
        fail(f"missing required E2E names: {sorted(required_flows - all_named_tests)}")

    validation = validation_path.read_text(encoding="utf-8")
    undocumented = sorted(test for test in all_named_tests if f"`{test}`" not in validation)
    if undocumented:
        fail(f"validation.md omits manifest tests: {undocumented}")

    environment = environment_path.read_text(encoding="utf-8")
    for required_field in (
        "base_image",
        "apt_snapshot",
        "packages",
        "fonts",
        "sway_command",
        "capture_helper",
        "input_helper",
        "toolchain",
        "home",
        "xdg_config_home",
        "xdg_cache_home",
        "locale_env",
        "fontconfig_file",
        "cosmic_theme_fixture",
        "atspi_coordinate_space",
    ):
        if f"{required_field} =" not in environment:
            fail(f"E2E environment lock contract omits {required_field}")

    print(
        f"UI scenario bootstrap valid: {len(operation_ids)} operations, "
        f"{len(flow_ids)} UI flows, {len(all_named_tests)} named tests"
    )
    if arguments.phase is not None:
        selected = manifest.get("target", [])
        if arguments.phase != "all":
            selected = [target for target in selected if target["phase"] == arguments.phase]
            if not selected:
                fail(f"no required-test target is registered for phase {arguments.phase}")
        for target in selected:
            run_target(root, target)
    return 0


def run_target(root: Path, target: dict[str, Any]) -> None:
    kind = target["kind"]
    expected = target["tests"]
    if kind == "rust-integration":
        command = ["cargo", "test", "-p", target["package"], "--locked"]
        features = target.get("features", [])
        if features:
            command.extend(["--features", ",".join(features)])
        command.extend(["--test", target["name"], "--", "--list"])
    elif kind == "rust-bin":
        command = [
            "cargo", "test", "-p", target["package"], "--locked",
            "--bin", target["name"], "--", "--list",
        ]
    elif kind == "e2e-case":
        command = [
            "cargo", "run", "-p", "ui-e2e-runner", "--locked", "--",
            "list-cases", "--root", target["name"],
        ]
    else:
        fail(f"unsupported required-test target kind: {kind}")
    completed = subprocess.run(command, cwd=root, text=True, capture_output=True)
    output = completed.stdout + completed.stderr
    if completed.returncode:
        fail(f"cannot list target {target['name']}:\n{output}")
    if kind == "e2e-case":
        listed = [line.strip() for line in completed.stdout.splitlines() if line.strip()]
        for test in expected:
            if listed.count(test) != 1:
                fail(f"E2E case {test!r} must appear exactly once in {target['name']}")
        return
    for test in expected:
        pattern = rf"(?m)^(?:[A-Za-z0-9_]+::)*{re.escape(test)}: test$"
        count = len(re.findall(pattern, output))
        if count != 1:
            fail(f"required test {test!r} must appear exactly once in target {target['name']}")


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (KeyError, TypeError, ValueError) as error:
        print(f"ui scenario plan lock invalid: {error}", file=sys.stderr)
        raise SystemExit(1) from error

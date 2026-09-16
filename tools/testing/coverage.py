#!/usr/bin/env python3
"""Fail-closed first-party coverage gate; consumes LLVM JSON and matching LCOV.

No thresholds can be overridden on the command line. Host and native-lab
evidence are mandatory; full-ui mode additionally requires every UI case.
"""
from __future__ import annotations

import argparse
from collections import defaultdict
import datetime as dt
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tomllib
import execution_policy

UI_CASES = {
    "physical_partition_format", "busy_unmount", "luks_unlock",
    "logical_preflight_confirmation", "network_mount", "image_usage_progress",
    "keyboard_accessibility", "live_scenario_reload",
}
EXCEPTION_FIELDS = {"path", "start_line", "end_line", "reason", "evidence", "owner", "expires"}


def source_path(value: str, root: Path) -> str | None:
    """Exact workspace source boundary, not a production ignore regex."""
    path = Path(value)
    if path.is_absolute():
        try:
            path = path.relative_to(root)
        except ValueError:
            # The baked image's fixed build root is a documented equivalence.
            for build_root in ("/workspace", "/opt/ui-test/source"):
                try:
                    path = path.relative_to(build_root)
                    break
                except ValueError:
                    continue
            else:
                return None
    parts = path.parts
    if ".." in parts or not parts or path.suffix != ".rs":
        return None
    if parts[0] == "src":
        return path.as_posix()
    if len(parts) >= 4 and parts[0] in {"crates", "tools"} and parts[2] == "src":
        return path.as_posix()
    return None


def scope(path: str) -> str:
    return "application" if path.startswith("src/") else "/".join(path.split("/")[:2])


def workspace_sources(root: Path) -> dict[str, Path]:
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    result = {}
    for member in manifest.get("workspace", {}).get("members", ["."]):
        if any(char in member for char in "*?[]"):
            raise ValueError("workspace source inventory requires explicit member paths")
        directory = (root / member / "src").resolve()
        if not directory.is_relative_to(root) or not directory.is_dir():
            raise ValueError("workspace member lacks an in-repository source directory")
        for path in directory.rglob("*.rs"):
            relative = path.relative_to(root).as_posix()
            if source_path(relative, root) != relative:
                raise ValueError("workspace source is outside the audited path boundary")
            result[relative] = path
    return result


def threshold(name: str) -> int:
    return 100 if name in {
        "crates/storage-lab-tests", "crates/test-backend", "crates/ui-e2e-runner",
        "crates/storage-contracts", "crates/storage-types", "crates/storage-sys",
    } else 98


def workspace_inputs(root: Path) -> dict[str, str]:
    """Build/test inputs, not just production Rust, invalidate saved evidence."""
    paths = {root / name for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "build.rs", "justfile", ".dockerignore")}
    for name in ("src", "crates", "tools", "tests", "resources", "i18n", ".github", ".config"):
        paths.update(path for path in (root / name).rglob("*") if not {"target", "__pycache__", ".git"} & set(path.relative_to(root).parts))
    return {path.relative_to(root).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in sorted(paths) if path.is_file()}


def read_lcov(text: str, root: Path) -> dict[str, dict[int, int]]:
    result: dict[str, dict[int, int]] = {}
    current = None
    for line in text.splitlines():
        if line.startswith("SF:"):
            current = source_path(line[3:], root)
            if current is not None:
                result.setdefault(current, {})
        elif line.startswith("DA:") and current is not None:
            number, count, *_ = line[3:].split(",")
            number, count = int(number), int(count)
            if number <= 0 or count < 0:
                raise ValueError("invalid LCOV line/count")
            result[current][number] = max(result[current].get(number, 0), count)
        elif line == "end_of_record":
            current = None
    if not result:
        raise ValueError("LCOV contains no first-party executable code")
    return result


def read_functions(document: dict, root: Path) -> dict[str, dict[tuple[int, int, str], int]]:
    functions = defaultdict(dict)
    for unit in document["data"]:
        for function in unit.get("functions", []):
            if not function["regions"]:
                continue
            start, start_column, end, end_column, _, file_id, _, *_ = function["regions"][0]
            path = source_path(function["filenames"][file_id], root)
            if path is None:
                continue
            # Function coverage measures source definitions, not separate
            # monomorphizations or crate hashes in host/container builds.
            # Preserve columns so two closures on one line remain distinct.
            key = (start, end, f"{start_column}:{end_column}")
            functions[path][key] = max(functions[path].get(key, 0), function["count"])
    if not functions:
        raise ValueError("LLVM JSON lacks first-party per-function records")
    return dict(functions)


def changed_lines(root: Path, base: str) -> dict[str, set[int]]:
    # Resolve the ref first; git cannot interpret it as an option or path.
    commit = subprocess.check_output(["git", "rev-parse", "--verify", f"{base}^{{commit}}"], cwd=root, text=True).strip()
    diff = subprocess.check_output(["git", "diff", "--no-ext-diff", "--unified=0", commit, "--", "*.rs"], cwd=root, text=True)
    result = defaultdict(set)
    current = None
    for line in diff.splitlines():
        if line.startswith("+++ b/"):
            current = source_path(line[6:], root)
        elif line.startswith("@@") and current is not None:
            match = re.match(r"@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@", line)
            if match is None:
                raise ValueError("unrecognized git diff hunk")
            start = int(match[1])
            count = int(match[2]) if match[2] is not None else 1
            result[current].update(range(start, start + count))
    return dict(result)


def validate_evidence(document: dict, root: Path, mode: str = "full-ui") -> set[str]:
    if mode not in execution_policy.MODES:
        raise ValueError("unknown coverage execution mode")
    sources = set()
    tests = set()
    seen = set()
    ui = set()
    for profile in document.get("profiles", []):
        path = (root / profile["path"]).resolve()
        if path in seen or not path.is_file() or path.suffix != ".profraw":
            raise ValueError("missing or duplicate raw coverage profile")
        seen.add(path)
        data = path.read_bytes()
        if not data or hashlib.sha256(data).hexdigest() != profile["sha256"]:
            raise ValueError("coverage profile hash mismatch")
        source = profile["source"]
        if source not in {"host", "lab", "ui"} or not profile.get("tests"):
            raise ValueError("profile requires a known source and executed test names")
        sources.add(source)
        tests.update(profile["tests"])
        if source == "ui":
            if mode != "full-ui":
                raise ValueError("UI profiles cannot enter a non-rendered report")
            proof = profile.get("execution", {})
            report_path = root / proof.get("path", "")
            if not report_path.is_file() or hashlib.sha256(report_path.read_bytes()).hexdigest() != proof.get("sha256"):
                raise ValueError("missing or changed executed UI report")
            report = json.loads(report_path.read_text())
            case = report.get("case", "")
            if case not in UI_CASES or profile["tests"] != [case]:
                raise ValueError("UI profile must name exactly its executed case")
            manifest = root / "tests/ui/cases" / f"{case}.toml"
            program = tomllib.loads(manifest.read_text())
            if not report.get("coverage_enabled") or report.get("functional_status") != "passed" or report.get("status") not in {"semantic_passed", "semantic_passed_with_known_shutdown_failure"}:
                raise ValueError("UI report is not an instrumented semantic pass")
            if report.get("case_sha256") != hashlib.sha256(manifest.read_bytes()).hexdigest() or report.get("completed_steps") != [step["id"] for step in program["step"]]:
                raise ValueError("UI report case changed or steps were not executed")
            ui.update(profile["tests"])
    if mode == "non-rendered" and sources != {"host", "lab"}:
        raise ValueError("host and lab profiles are required for non-rendered coverage")
    if mode == "full-ui" and sources != {"host", "lab", "ui"}:
        raise ValueError("host, lab and executed UI profiles are required for the final report")
    if mode == "full-ui" and not UI_CASES <= ui:
        raise ValueError(f"missing executed UI coverage sources: {sorted(UI_CASES - ui)}")
    return tests


def validate_provenance(document: dict, root: Path, mode: str = "full-ui") -> None:
    if mode not in execution_policy.MODES:
        raise ValueError("unknown coverage execution mode")
    if document.get("host_exit") != 0 or document.get("lab_exit") != 0:
        raise ValueError("failed or missing test-run exit status")
    if mode == "non-rendered" and (document.get("ui_exit") is not None or document.get("ui_status") != "deferred"):
        raise ValueError("non-rendered UI evidence must be explicitly deferred, not passed")
    if mode == "full-ui" and document.get("ui_exit") != 0:
        raise ValueError("failed or missing interactive UI run exit status")
    if document.get("input_sha256") != workspace_inputs(root):
        raise ValueError("build/test inputs changed or were omitted after instrumentation")
    hashes = document.get("source_sha256", {})
    if not hashes:
        raise ValueError("missing source revision hashes")
    if set(hashes) != set(workspace_sources(root)):
        raise ValueError("workspace source inventory changed or was omitted")
    for path, expected in hashes.items():
        if source_path(path, root) != path:
            raise ValueError("unexpected source revision path")
        if hashlib.sha256((root / path).read_bytes()).hexdigest() != expected:
            raise ValueError(f"source changed after instrumentation: {path}")
    objects = document.get("objects", [])
    if not objects:
        raise ValueError("matching instrumented executables are required")
    for entry in objects:
        path = (root / entry["path"]).resolve()
        if not path.is_relative_to(root / "target/coverage"):
            raise ValueError("coverage executable must belong to this run")
        if hashlib.sha256(path.read_bytes()).hexdigest() != entry["sha256"]:
            raise ValueError("instrumented executable hash mismatch")


def exceptions(document: dict, lines: dict, functions: dict, tests: set[str], today: dt.date) -> dict[str, set[int]]:
    if set(document) != {"exceptions"} or not isinstance(document["exceptions"], list):
        raise ValueError("exception manifest must contain only an exceptions list")
    result = defaultdict(set)
    for entry in document["exceptions"]:
        if set(entry) != EXCEPTION_FIELDS:
            raise ValueError("exception fields do not match the audited schema")
        path = entry["path"]
        if path not in lines or any(char in path for char in "*?[]"):
            raise ValueError("exception must name an exact reported source file")
        start, end = entry["start_line"], entry["end_line"]
        if type(start) is not int or type(end) is not int or start < 1 or end < start:
            raise ValueError("invalid exception line range")
        selected = set(range(start, end + 1))
        if not selected <= lines[path].keys() or result[path] & selected:
            raise ValueError("exception includes non-executable or duplicate lines")
        if not all(isinstance(entry[key], str) and entry[key].strip() for key in ("reason", "owner", "evidence")):
            raise ValueError("exception requires reason, owner and compensating evidence")
        if entry["evidence"] not in tests:
            raise ValueError("exception compensating test was not executed")
        expiry = dt.date.fromisoformat(str(entry["expires"]))
        if expiry <= today:
            raise ValueError("coverage exception has expired")
        result[path].update(selected)
    for path, selected in result.items():
        for start, end, _ in functions.get(path, {}):
            executable = {line for line in lines[path] if start <= line <= end}
            if executable and executable <= selected:
                raise ValueError("exception may not cover an entire function")
    production = sum(len(entries) for path, entries in lines.items()
                     if not path.startswith("tools/") and scope(path) not in {
                         "crates/storage-lab-tests", "crates/test-backend", "crates/ui-e2e-runner"})
    if result and sum(map(len, result.values())) * 100 >= production * 2:
        raise ValueError("excepted lines must remain below 2%")
    return dict(result)


def evaluate(lines: dict, functions: dict, changed: dict, exempt: dict) -> tuple[dict, list[str]]:
    totals = defaultdict(lambda: [0, 0, 0, 0])
    failures = []
    for path, entries in lines.items():
        for line, count in entries.items():
            if line in exempt.get(path, set()):
                continue
            for name in (scope(path), "workspace"):
                totals[name][0] += 1
                totals[name][1] += count > 0
            if line in changed.get(path, set()) and count == 0:
                failures.append(f"changed uncovered line: {path}:{line}")
    for path, entries in functions.items():
        for (start, end, _), count in entries.items():
            for name in (scope(path), "workspace"):
                totals[name][2] += 1
                totals[name][3] += count > 0
            if count == 0 and any(start <= line <= end for line in changed.get(path, set()) - exempt.get(path, set())):
                failures.append(f"changed uncovered function: {path}:{start}")
    report = {}
    for name, (line_total, line_hit, fn_total, fn_hit) in sorted(totals.items()):
        required = threshold(name)
        report[name] = {"line_total": line_total, "line_covered": line_hit, "function_total": fn_total, "function_covered": fn_hit, "required_percent": required}
        if not line_total or not fn_total or line_hit * 100 < line_total * required or fn_hit * 100 < fn_total * required:
            failures.append(f"threshold regression: {name} requires {required}% lines and functions")
    return report, failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True)
    parser.add_argument("--mode", choices=execution_policy.MODES, default="non-rendered")
    parser.add_argument("--summary", type=Path)
    parser.add_argument("--lcov", type=Path)
    parser.add_argument("--evidence", type=Path)
    parser.add_argument("--exceptions", type=Path, default=Path("docs/plans/5-testing-v2/coverage-exceptions.toml"))
    args = parser.parse_args()
    output = Path("target/coverage") / args.mode
    args.summary = args.summary or output / "summary.json"
    args.lcov = args.lcov or output / "lcov.info"
    args.evidence = args.evidence or output / "evidence.json"
    root = Path(__file__).resolve().parents[2]
    lines = read_lcov(args.lcov.read_text(), root)
    functions = read_functions(json.loads(args.summary.read_text()), root)
    if set(lines) != set(functions):
        raise ValueError("LLVM and LCOV source inventories differ")
    evidence = json.loads(args.evidence.read_text())
    evidence_failures = []
    try:
        execution_policy.validate(evidence, root, args.mode)
    except (KeyError, ValueError, OSError) as error:
        evidence_failures.append(str(error))
    for name, path in (("summary.json", args.summary), ("lcov.info", args.lcov)):
        if evidence.get("reports", {}).get(name) != hashlib.sha256(path.read_bytes()).hexdigest():
            evidence_failures.append(f"report hash missing or stale: {name}")
    tests = set()
    for check in (validate_provenance, validate_evidence):
        try:
            result = check(evidence, root, args.mode)
            if result is not None:
                tests = result
        except (KeyError, ValueError, OSError) as error:
            evidence_failures.append(str(error))
    exempt = exceptions(tomllib.loads(args.exceptions.read_text()), lines, functions, tests, dt.date.today())
    report, failures = evaluate(lines, functions, changed_lines(root, args.base), exempt if not evidence_failures else {})
    expected_scopes = {scope(path) for path in workspace_sources(root)}
    for missing in sorted(expected_scopes - report.keys()):
        failures.append(f"missing workspace package coverage: {missing}")
    failures = evidence_failures + failures
    unmapped = sorted(set(workspace_sources(root)) - set(lines))
    # Files can contain only type declarations: do not invent executable line
    # counts, but expose every absent mapping for review instead of hiding it.
    if unmapped:
        failures.append("unmeasured workspace sources require mapping review: " + ", ".join(unmapped))
    print(json.dumps({"mode": args.mode, "ui_status": evidence.get("ui_status"),
                      "scopes": report, "unmapped_sources": unmapped, "failures": failures}, indent=2))
    return bool(failures)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (KeyError, ValueError, OSError, subprocess.CalledProcessError) as error:
        print(json.dumps({"scopes": {}, "failures": [str(error)]}, indent=2))
        print(f"coverage gate failed: {error}", file=sys.stderr)
        sys.exit(1)

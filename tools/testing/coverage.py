#!/usr/bin/env python3
"""Fail-closed first-party coverage gate; consumes LLVM JSON and matching LCOV.

No thresholds can be overridden on the command line. Container and interactive
UI evidence are mandatory; a host-only report is never an acceptance report.
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
            try:
                path = path.relative_to("/workspace")
            except ValueError:
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


def threshold(name: str) -> int:
    return 100 if name in {
        "crates/storage-lab-tests", "crates/test-backend", "crates/ui-e2e-runner",
        "crates/storage-contracts", "crates/storage-types", "crates/storage-sys",
    } else 98


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


def validate_evidence(document: dict, root: Path) -> set[str]:
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
            ui.update(profile["tests"])
    if sources != {"host", "lab", "ui"}:
        raise ValueError("host, lab and executed UI profiles are required for the final report")
    if not UI_CASES <= ui:
        raise ValueError(f"missing executed UI coverage sources: {sorted(UI_CASES - ui)}")
    return tests


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
    if sum(map(len, result.values())) * 100 >= sum(map(len, lines.values())) * 2:
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
    parser.add_argument("--summary", type=Path, default=Path("target/coverage/summary.json"))
    parser.add_argument("--lcov", type=Path, default=Path("target/coverage/lcov.info"))
    parser.add_argument("--evidence", type=Path, default=Path("target/coverage/evidence.json"))
    parser.add_argument("--exceptions", type=Path, default=Path("docs/plans/5-testing-v2/coverage-exceptions.toml"))
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    lines = read_lcov(args.lcov.read_text(), root)
    functions = read_functions(json.loads(args.summary.read_text()), root)
    if set(lines) != set(functions):
        raise ValueError("LLVM and LCOV source inventories differ")
    tests = validate_evidence(json.loads(args.evidence.read_text()), root)
    exempt = exceptions(tomllib.loads(args.exceptions.read_text()), lines, functions, tests, dt.date.today())
    report, failures = evaluate(lines, functions, changed_lines(root, args.base), exempt)
    print(json.dumps({"scopes": report, "failures": failures}, indent=2))
    return bool(failures)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (KeyError, ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"coverage gate failed: {error}", file=sys.stderr)
        sys.exit(1)

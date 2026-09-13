#!/usr/bin/env python3
"""Build, execute and merge real host/container coverage before acceptance.

Uses cargo-llvm-cov's compiler environment and the existing Testcontainers
suite, not a second storage case runner. Missing interactive UI execution is a
hard acceptance failure even when an intermediate report can be generated.
"""
from __future__ import annotations

import argparse
import hashlib
import html
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import re
import shlex
import shutil
import subprocess
import tarfile
import tempfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
_gate_spec = importlib.util.spec_from_file_location("storage_coverage_gate", Path(__file__).with_name("coverage.py"))
gate = importlib.util.module_from_spec(_gate_spec)
_gate_spec.loader.exec_module(gate)


def digest(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def command(args, *, env=None, output=None, check=True):
    print("+ " + shlex.join(map(str, args)), flush=True)
    with output.open("wb") if output else open(os.devnull, "wb") as stream:
        result = subprocess.run(args, cwd=ROOT, env=env, stdout=stream if output else None)
    if check:
        result.check_returncode()
    return result.returncode


def unpack_lab(archive: Path, destination: Path, target: str) -> tuple[list[Path], Path]:
    """Accept only regular profiles and the one matching baked executable."""
    destination.mkdir(parents=True, exist_ok=False)
    profiles = []
    executable = None
    seen = set()
    with tarfile.open(archive, "r:gz") as source:
        for member in source:
            path = PurePosixPath(member.name)
            if path.is_absolute() or ".." in path.parts:
                raise ValueError("coverage archive contains path traversal")
            if member.isdir():
                continue
            if not member.isfile() or member.size <= 0 or member.size > 2 * 1024**3:
                raise ValueError("coverage archive must contain bounded regular files")
            is_profile = path.parent == PurePosixPath("tmp/storage-lab-profiles") and path.suffix == ".profraw"
            is_executable = str(path) == f"opt/storage-lab/bin/storage-lab-{target}"
            if not (is_profile or is_executable) or path in seen:
                raise ValueError("unexpected or duplicate coverage archive entry")
            seen.add(path)
            output = destination / path.name
            with source.extractfile(member) as data, output.open("xb") as dest:
                shutil.copyfileobj(data, dest)
            if is_profile:
                profiles.append(output)
            else:
                executable = output
    if not profiles or executable is None:
        raise ValueError("lab coverage requires raw profiles and their matching executable")
    return profiles, executable


def write_html(output: Path, lines: dict, functions: dict) -> None:
    """Render the same union as the gate, never an arbitrary ELF's line view."""
    directory = output / "html"
    directory.mkdir(exist_ok=True)
    report, _ = gate.evaluate(lines, functions, {}, {})
    rows = []
    for name, counts in report.items():
        rows.append(f"<tr><td>{html.escape(name)}</td><td>{counts['line_covered']}/{counts['line_total']}</td>"
                    f"<td>{counts['function_covered']}/{counts['function_total']}</td></tr>")
    links = []
    style = "<style>body{font:16px system-ui;margin:2rem}td{padding:.4rem}pre{line-height:1.5}.hit{background:#d8f5de}.miss{background:#ffdddd}a{color:#1455a0}</style>"
    for number, (path, entries) in enumerate(sorted(lines.items())):
        page = f"source-{number}.html"
        links.append(f'<li><a href="{page}">{html.escape(path)}</a></li>')
        source = []
        for line, text in enumerate((ROOT / path).read_text().splitlines(), 1):
            state = "hit" if entries.get(line, 0) else "miss"
            cls = f' class="{state}"' if line in entries else ""
            source.append(f'<span id="L{line}"{cls}>{line:5} {html.escape(text)}</span>')
        (directory / page).write_text(f"<!doctype html><meta charset=utf-8>{style}<title>{html.escape(path)}</title><h1>{html.escape(path)}</h1><pre>" + "\n".join(source) + "</pre>")
    (directory / "index.html").write_text("<!doctype html><meta charset=utf-8>" + style + "<title>Executed coverage</title><h1>Executed coverage</h1><p>Raw union of executed profiles, before exceptions. Acceptance also requires every test source and all thresholds; see acceptance.json.</p><table><tr><th>Scope</th><th>Lines</th><th>Functions</th></tr>" + "".join(rows) + "</table><ul>" + "".join(links) + "</ul>")


def export_reports(output: Path, run: Path, llvm: Path, groups: list[dict]) -> None:
    # Different feature/build roots can use the same symbol with different
    # coverage mappings. Feeding all ELFs to one llvm-cov invocation silently
    # chooses one file's line mapping. Export matching build groups separately
    # and union source definitions/line counts with the audited gate instead.
    document = {"type": "llvm.coverage.json.export", "version": "2.0.1", "data": []}
    lcov = []
    sources = [str(path) for path in gate.workspace_sources(ROOT).values()]
    for number, group in enumerate(groups):
        profile_list = run / f"group-{number}.profiles"
        profile_list.write_text("\n".join(map(str, group["profiles"])) + "\n")
        profdata = run / f"group-{number}.profdata"
        command([llvm / "llvm-profdata", "merge", "-sparse", "--failure-mode=any", "-f", profile_list, "-o", profdata])
        common = [f"-path-equivalence=/workspace,{ROOT}", f"-instr-profile={profdata}", *[f"-object={path}" for path in group["objects"]]]
        summary = run / f"group-{number}.json"
        trace = run / f"group-{number}.lcov"
        command([llvm / "llvm-cov", "export", *common, "--sources", *sources], output=summary)
        command([llvm / "llvm-cov", "export", "-format=lcov", *common, "--sources", *sources], output=trace)
        document["data"].extend(json.loads(summary.read_text())["data"])
        lcov.append(trace.read_text())
    (output / "summary.json").write_text(json.dumps(document))
    (output / "lcov.info").write_text("\n".join(lcov))
    lines = gate.read_lcov("\n".join(lcov), ROOT)
    functions = gate.read_functions(document, ROOT)
    write_html(output, lines, functions)


def finish_report(output: Path, run: Path, llvm: Path, groups: list[dict], evidence: dict, base: str) -> int:
    export_reports(output, run, llvm, groups)
    evidence["reports"] = {name: digest(output / name) for name in ("summary.json", "lcov.info")}
    (output / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
    acceptance = command(["python3", "tools/testing/coverage.py", "--base", base], output=output / "acceptance.json", check=False)
    print(f"Reports: {output}. Host exit={evidence['host_exit']}; lab exit={evidence['lab_exit']}; acceptance={acceptance}")
    return int(bool(evidence["host_exit"] or evidence["lab_exit"] or acceptance))


def llvm_directory() -> Path:
    sysroot = Path(subprocess.check_output(["rustc", "--print", "sysroot"], cwd=ROOT, text=True).strip())
    version = subprocess.check_output(["rustc", "-vV"], cwd=ROOT, text=True)
    host = next(line.split(": ", 1)[1] for line in version.splitlines() if line.startswith("host: "))
    return sysroot / "lib/rustlib" / host / "bin"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True)
    parser.add_argument("--report-only", action="store_true", help="Re-export the last successful host/lab run only if its sources, profiles and ELFs are unchanged")
    args = parser.parse_args()
    output = ROOT / "target/coverage"
    output.mkdir(parents=True, exist_ok=True)
    if args.report_only:
        evidence = json.loads((output / "evidence.json").read_text())
        gate.validate_provenance(evidence, ROOT)
        # validate_evidence also requires UI execution. Validate raw bytes here
        # without claiming the incomplete source set passes acceptance.
        for profile in evidence["profiles"]:
            if digest(ROOT / profile["path"]) != profile["sha256"]:
                raise ValueError("raw profile changed since execution")
        run = (ROOT / evidence["profiles"][0]["path"]).parent.parent
        if run.parent != output or not run.name.startswith("run-"):
            raise ValueError("unexpected saved coverage run directory")
        groups = [{key: [ROOT / path for path in paths] for key, paths in group.items()} for group in evidence["groups"]]
        return finish_report(output, run, llvm_directory(), groups, evidence, args.base)
    run = Path(tempfile.mkdtemp(prefix="run-", dir=output))
    (output / "acceptance.json").write_text(json.dumps({"run": run.name, "scopes": {}, "failures": ["coverage run has not finished"]}) + "\n")
    source_hashes = {relative: digest(path) for relative, path in gate.workspace_sources(ROOT).items()}
    host_profiles = run / "host-profiles"
    host_profiles.mkdir()
    # Reuse compilation cache, but never reuse raw profiles from an earlier run.
    build_env = os.environ | {"CARGO_TARGET_DIR": str(output / "host-build")}
    compiler_env = subprocess.check_output(["cargo", "llvm-cov", "show-env"], cwd=ROOT, env=build_env, text=True)
    for line in compiler_env.splitlines():
        key, value = line.split("=", 1)
        build_env[key] = shlex.split(value)[0]
    build_env["LLVM_PROFILE_FILE"] = "/dev/null"
    artifacts = run / "host-build.jsonl"
    command(["cargo", "test", "--workspace", "--all-features", "--locked", "--no-run", "--message-format=json-render-diagnostics"], env=build_env, output=artifacts)
    objects = []
    for line in artifacts.read_text().splitlines():
        artifact = json.loads(line)
        if artifact.get("reason") == "compiler-artifact" and artifact.get("profile", {}).get("test") and artifact.get("executable"):
            objects.append(Path(artifact["executable"]))
    if not objects:
        raise ValueError("Cargo returned no host test executables")
    host_log = run / "host-tests.log"
    host_code = command(["cargo", "test", "--workspace", "--all-features", "--locked"], env=build_env | {"LLVM_PROFILE_FILE": str(host_profiles / "host-%m-%p.profraw")}, output=host_log, check=False)
    host_tests = re.findall(r"^test (.+) \.\.\. ok$", host_log.read_text(), re.MULTILINE)
    profiles = list(host_profiles.glob("*.profraw"))
    groups = [{"profiles": profiles.copy(), "objects": objects.copy()}]
    lab_groups = {}
    evidence = {"profiles": []}
    before = set((ROOT / "target/storage-lab-artifacts").glob("run-*"))
    junit = ROOT / "target/nextest/storage-lab/junit.xml"
    previous_junit = digest(junit) if junit.is_file() else None
    lab_code = command(["just", "test-lab"], env=build_env | {"STORAGE_LAB": "1", "STORAGE_LAB_COVERAGE": "1", "LLVM_PROFILE_FILE": str(host_profiles / "bridge-%m-%p.profraw")}, output=run / "lab-tests.log", check=False)
    # Nextest's store-dir is relative to the workspace, independently of the
    # Cargo build target directory. Require a fresh report from this execution.
    if not junit.is_file() or digest(junit) == previous_junit:
        raise ValueError(f"missing or stale instrumented bridge JUnit: {junit}")
    shutil.copyfile(junit, run / "bridge.junit.xml")
    bridge_tests = [case.attrib["name"] for case in ET.parse(junit).iter("testcase")
                    if case.find("skipped") is None]
    bridge_artifacts = run / "bridge-build.jsonl"
    command(["cargo", "test", "--locked", "-p", "storage-lab-tests", "--features", "outer-bridge", "--test", "bridge", "--no-run", "--message-format=json-render-diagnostics"], env=build_env, output=bridge_artifacts)
    bridge_objects = [Path(artifact["executable"]) for line in bridge_artifacts.read_text().splitlines()
                      if (artifact := json.loads(line)).get("reason") == "compiler-artifact"
                      and artifact.get("executable") and artifact.get("profile", {}).get("test")]
    if not bridge_objects or not bridge_tests:
        raise ValueError("instrumented bridge must execute tests and supply its matching ELF")
    objects.extend(path for path in bridge_objects if path not in objects)
    groups[0]["objects"] = objects.copy()
    fresh_host = list(host_profiles.glob("*.profraw"))
    profiles = fresh_host.copy()
    groups[0]["profiles"] = fresh_host
    evidence["profiles"].extend(dict(path=str(path.relative_to(ROOT)), sha256=digest(path), source="host", tests=host_tests + bridge_tests) for path in fresh_host)
    after = set((ROOT / "target/storage-lab-artifacts").glob("run-*"))
    for directory in sorted(after - before):
        archive = directory / "profiles/inner.tar.gz"
        if not archive.is_file():
            raise ValueError(f"missing lab profile archive: {directory}")
        target, test = (directory / "inner-test.selection.txt").read_text().strip().split("\t")
        raw, executable = unpack_lab(archive, run / "lab" / directory.name, target)
        profiles.extend(raw)
        objects.append(executable)
        group = lab_groups.setdefault(digest(executable), {"profiles": [], "objects": [executable]})
        group["profiles"].extend(raw)
        evidence["profiles"].extend(dict(path=str(path.relative_to(ROOT)), sha256=digest(path), source="lab", tests=[test]) for path in raw)
    if not profiles:
        raise ValueError("no executed coverage profiles")
    sysroot = Path(subprocess.check_output(["rustc", "--print", "sysroot"], cwd=ROOT, text=True).strip())
    rust_version = subprocess.check_output(["rustc", "-vV"], cwd=ROOT, text=True)
    host = next(line.split(": ", 1)[1] for line in rust_version.splitlines() if line.startswith("host: "))
    llvm = sysroot / "lib/rustlib" / host / "bin"
    evidence["objects"] = [dict(path=str(path.relative_to(ROOT)), sha256=digest(path)) for path in objects]
    evidence["rustc"] = rust_version
    evidence["run"] = run.name
    evidence["source_sha256"] = source_hashes
    evidence["host_exit"] = host_code
    evidence["lab_exit"] = lab_code
    groups.extend(lab_groups.values())
    evidence["groups"] = [{key: [str(path.relative_to(ROOT)) for path in paths] for key, paths in group.items()} for group in groups]
    evidence["ui_status"] = "missing executed interactive cases; capability inventory is not coverage"
    (output / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
    profile_list = run / "profiles.txt"
    profile_list.write_text("\n".join(map(str, profiles)) + "\n")
    profdata = run / "merged.profdata"
    command([llvm / "llvm-profdata", "merge", "-sparse", "--failure-mode=any", "-f", profile_list, "-o", profdata])
    return finish_report(output, run, llvm, groups, evidence, args.base)


if __name__ == "__main__":
    raise SystemExit(main())

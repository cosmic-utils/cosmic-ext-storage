"""Regression tests for the acceptance checker (no Docker or LLVM required)."""
import datetime as dt
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from run_coverage import gate


class CoverageGateTests(unittest.TestCase):
    def test_source_and_executable_changes_invalidate_the_report(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text('[workspace]\nmembers = ["."]\n')
            source = root / "src/lib.rs"
            source.parent.mkdir()
            source.write_text("pub fn f() {}")
            executable = root / "target/coverage/run-1/test"
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b"instrumented-elf")
            evidence = dict(host_exit=0, lab_exit=0, ui_exit=0, input_sha256=gate.workspace_inputs(root),
                            source_sha256={"src/lib.rs": hashlib.sha256(source.read_bytes()).hexdigest()},
                            objects=[dict(path=str(executable.relative_to(root)), sha256=hashlib.sha256(executable.read_bytes()).hexdigest())])
            gate.validate_provenance(evidence, root)
            extra = root / "src/new.rs"
            extra.write_text("pub fn untested() {}")
            with self.assertRaisesRegex(ValueError, "inputs changed"):
                gate.validate_provenance(evidence, root)
            extra.unlink()
            with self.assertRaisesRegex(ValueError, "exit status"):
                gate.validate_provenance(evidence | {"lab_exit": 1}, root)
            source.write_text("pub fn changed() {}")
            with self.assertRaisesRegex(ValueError, "inputs changed"):
                gate.validate_provenance(evidence, root)
            source.write_text("pub fn f() {}")
            fixture = root / "tests/fixture.toml"
            fixture.parent.mkdir()
            fixture.write_text("changed = true")
            with self.assertRaisesRegex(ValueError, "inputs changed"):
                gate.validate_provenance(evidence, root)
            fixture.unlink()
            executable.write_bytes(b"different-build")
            with self.assertRaisesRegex(ValueError, "executable hash mismatch"):
                gate.validate_provenance(evidence, root)

    def test_changed_uncovered_line_fails(self):
        lines = {"src/app.rs": {line: int(line != 5) for line in range(1, 101)}}
        functions = {"src/app.rs": {(1, 100, "main"): 1}}
        _, failures = gate.evaluate(lines, functions, {"src/app.rs": {5}}, {})
        self.assertEqual(failures, ["changed uncovered line: src/app.rs:5"])

    def test_changed_uncovered_function_fails(self):
        _, failures = gate.evaluate({"src/app.rs": {1: 1}},
                                   {"src/app.rs": {(1, 1, "main"): 0}},
                                   {"src/app.rs": {1}}, {})
        self.assertIn("changed uncovered function: src/app.rs:1", failures)

    def test_threshold_regression_fails(self):
        for path, required in [("src/app.rs", 98), ("crates/storage-sys/src/image.rs", 100)]:
            lines = {path: {line: int(line < required) for line in range(100)}}
            functions = {path: {(1, 100, "main"): 1}}
            self.assertEqual(gate.evaluate(lines, functions, {}, {})[1], [])
            lines[path][0] = 0
            self.assertTrue(gate.evaluate(lines, functions, {}, {})[1])

    def test_expired_or_broad_exception_fails(self):
        lines = {"src/app.rs": {line: 0 for line in range(1, 101)}}
        functions = {"src/app.rs": {(1, 100, "main"): 1}}
        entry = dict(path="src/app.rs", start_line=1, end_line=1, reason="kernel fault",
                     evidence="fault_test", owner="maintainer", expires="2099-01-01")
        def validate(value):
            return gate.exceptions({"exceptions": [value]}, lines, functions, {"fault_test"}, dt.date(2026, 9, 13))
        self.assertEqual(validate(entry), {"src/app.rs": {1}})
        for change in [dict(expires="2026-09-13"), dict(path="src/*"), dict(end_line=2),
                       dict(evidence="not_executed"), dict(owner=""), dict(start_line=0),
                       dict(end_line=101), dict(unknown=True)]:
            with self.subTest(change=change), self.assertRaises(ValueError):
                validate(entry | change)
        # Even a small function is not an exception-sized escape hatch.
        functions["src/app.rs"] = {(1, 1, "tiny"): 0}
        with self.assertRaisesRegex(ValueError, "entire function"):
            validate(entry)

    def test_lab_profile_is_required_for_final_report(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            records = []
            for source in ["host", "lab"]:
                path = root / f"{source}.profraw"
                path.write_bytes(source.encode())
                records.append(dict(path=str(path), source=source,
                                    sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                                    tests=[f"{source}_test"]))
            with self.assertRaisesRegex(ValueError, "host, lab and executed UI"):
                gate.validate_evidence({"profiles": records[:2]}, root)
            for case in sorted(gate.UI_CASES):
                manifest = root / "tests/ui/cases" / f"{case}.toml"
                manifest.parent.mkdir(parents=True, exist_ok=True)
                manifest.write_text('[[step]]\nid = "assertion"\n')
                report = root / f"{case}.json"
                report.write_text(json.dumps(dict(case=case, coverage_enabled=True, functional_status="passed", status="semantic_passed", completed_steps=["assertion"], case_sha256=hashlib.sha256(manifest.read_bytes()).hexdigest())))
                profile = root / f"{case}.profraw"
                profile.write_bytes(b"ui")
                records.append(dict(path=str(profile), source="ui", tests=[case], sha256=hashlib.sha256(profile.read_bytes()).hexdigest(), execution=dict(path=str(report), sha256=hashlib.sha256(report.read_bytes()).hexdigest())))
            self.assertIn("lab_test", gate.validate_evidence({"profiles": records}, root))
            generated = "native_case::case_01_capability"
            records[1]["tests"].append(generated)
            self.assertIn(generated, gate.validate_evidence({"profiles": records}, root))
            # Generated Rust cases cannot stand in for an unexecuted UI source.
            with self.assertRaisesRegex(ValueError, "missing executed UI coverage sources"):
                gate.validate_evidence({"profiles": records[:-1]}, root)
            proof = records[-1].pop("execution")
            with self.assertRaisesRegex(ValueError, "executed UI report"):
                gate.validate_evidence({"profiles": records}, root)
            records[-1]["execution"] = proof
            records[-1]["sha256"] = "wrong"
            with self.assertRaisesRegex(ValueError, "hash mismatch"):
                gate.validate_evidence({"profiles": records}, root)

    def test_shutdown_quarantine_cannot_substitute_for_ui_profiles(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            records = []
            for source in ["host", "lab"]:
                path = root / f"{source}.profraw"
                path.write_bytes(source.encode())
                records.append(dict(path=str(path), source=source,
                                    sha256=hashlib.sha256(path.read_bytes()).hexdigest(), tests=[source]))
            document = dict(profiles=records, ui_status="semantic_passed_with_known_shutdown_failure")
            with self.assertRaisesRegex(ValueError, "executed UI profiles"):
                gate.validate_evidence(document, root)
            empty = root / "ui.profraw"
            empty.touch()
            records.append(dict(path=str(empty), source="ui", tests=sorted(gate.UI_CASES),
                                sha256=hashlib.sha256(b"").hexdigest()))
            with self.assertRaisesRegex(ValueError, "profile hash mismatch"):
                gate.validate_evidence(document, root)
            empty.unlink()
            with self.assertRaisesRegex(ValueError, "missing or duplicate"):
                gate.validate_evidence(document, root)

    def test_third_party_and_test_source_are_excluded_by_exact_path_rule(self):
        root = Path("/repo")
        for path in ["/repo/tests/example.rs", "/repo/target/build/vendor/src/lib.rs",
                     "/cargo/registry/crate/src/lib.rs", "/repo/crates/name/tests/test.rs",
                     "src/../outside.rs", "/different/project/src/lib.rs"]:
            self.assertIsNone(gate.source_path(path, root), path)
        for path in ["src/app.rs", "crates/storage-sys/src/lib.rs", "crates/test-backend/src/lib.rs"]:
            for prefix in ["", "/repo/", "/workspace/"]:
                self.assertEqual(gate.source_path(prefix + path, root), path)

    def test_lcov_counts_are_merged_without_dropping_uncovered_lines(self):
        text = "SF:/repo/src/app.rs\nDA:1,0\nDA:2,1\nend_of_record\nSF:/repo/src/app.rs\nDA:1,2\nDA:3,0\nend_of_record"
        self.assertEqual(gate.read_lcov(text, Path("/repo")), {"src/app.rs": {1: 2, 2: 1, 3: 0}})
        with self.assertRaises(ValueError):
            gate.read_lcov("SF:/repo/src/app.rs\nDA:0,-1", Path("/repo"))
        with self.assertRaises(ValueError):
            gate.read_lcov("SF:/third-party/lib.rs\nDA:1,1", Path("/repo"))

    def test_function_definitions_merge_across_builds_and_instantiations(self):
        definitions = []
        for name, count in [("host_generic_u32", 0), ("lab_generic_u32", 1), ("host_generic_u64", 0)]:
            definitions.append(dict(name=name, count=count, filenames=["/repo/src/app.rs"], regions=[[1, 1, 4, 2, count, 0, 0, 0]]))
        functions = gate.read_functions({"data": [{"functions": definitions}]}, Path("/repo"))
        self.assertEqual(functions, {"src/app.rs": {(1, 4, "1:2"): 1}})


if __name__ == "__main__":
    unittest.main()

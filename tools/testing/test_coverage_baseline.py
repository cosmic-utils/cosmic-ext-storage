"""Acceptance-policy regressions; synthetic reports test policy, not coverage."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import coverage_baseline as baseline
import execution_policy


class BaselineTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / "src").mkdir()
        (self.root / "src/mod.rs").write_text("mod app;\n")
        (self.root / "script.py").write_text("print('support')\n")
        self.scope = dict(line_covered=40, line_total=100, function_covered=4, function_total=10)
        self.scopes = {"application": self.scope.copy(), "workspace": self.scope.copy()}
        self.sources = ["src/app.rs", "src/mod.rs"]
        self.document = dict(schema_version=1, mode="non-rendered",
                             approved="2026-09-16 user-approved no-regression policy",
                             evidence=[dict(revision="a" * 40, run="run-executed", report_sha256="b" * 64)],
                             scopes=copy.deepcopy(self.scopes), sources=self.sources,
                             unmapped=baseline.file_hashes(self.root, ["src/mod.rs"]),
                             unmeasured_support=baseline.file_hashes(self.root, ["script.py"]))

    def evaluate(self, **changes):
        args = dict(document=self.document, scopes=self.scopes, source_names=self.sources,
                    unmapped=["src/mod.rs"], support=["script.py"], root=self.root)
        return baseline.evaluate(**(args | changes))

    def test_equal_or_improved_coverage_passes_without_waiving_gaps(self):
        self.assertEqual(self.evaluate(), [])
        self.scopes["application"]["line_covered"] += 1
        self.assertEqual(self.evaluate(), [])
        self.assertEqual(self.evaluate(unmapped=[]), [], "a newly measured file is an improvement")

    def test_each_package_line_and_function_regression_fails(self):
        for name in self.scopes:
            for kind in ("line", "function"):
                with self.subTest(scope=name, kind=kind):
                    scopes = copy.deepcopy(self.scopes)
                    scopes[name][f"{kind}_covered"] -= 1
                    self.assertIn(f"baseline regression: {name} {kind} coverage", self.evaluate(scopes=scopes))
        expanded = copy.deepcopy(self.scopes)
        expanded["application"]["line_total"] += 1
        self.assertTrue(self.evaluate(scopes=expanded), "uncovered additions dilute coverage")

    def test_new_sources_missing_packages_and_changed_unmeasured_code_fail(self):
        self.assertTrue(self.evaluate(source_names=self.sources + ["src/new.rs"]))
        self.assertTrue(self.evaluate(scopes={"workspace": self.scope}))
        self.assertTrue(self.evaluate(support=["script.py", "src/mod.rs"]))
        (self.root / "src/mod.rs").write_text("pub fn uncovered() {}\n")
        self.assertTrue(self.evaluate())
        (self.root / "script.py").write_text("print('new unmeasured code')\n")
        self.assertEqual(len(self.evaluate()), 2)

    def test_malformed_or_unrecorded_baselines_fail_closed(self):
        for change in ({"schema_version": 2}, {"mode": "full-ui"}, {"approved": ""},
                       {"evidence": []}, {"evidence": [{}]}, {"unknown": True},
                       {"sources": ["src/app.rs", "src/app.rs"]},
                       {"unmapped": {"src/mod.rs": "not-a-hash"}}):
            with self.subTest(change=change), self.assertRaises(ValueError):
                self.evaluate(document=self.document | change)
        for hit, total in ((-1, 10), (11, 10), (0, 0), (True, 10), (1.0, 10)):
            altered = copy.deepcopy(self.document)
            altered["scopes"]["application"].update(line_covered=hit, line_total=total)
            with self.assertRaises(ValueError):
                self.evaluate(document=altered)

    def test_baseline_cannot_relabel_full_ui_or_captured_target_evidence(self):
        path = self.root / execution_policy.BASELINE
        path.parent.mkdir(parents=True)
        path.write_text(json.dumps(self.document))
        with self.assertRaises(ValueError):
            execution_policy.acceptance(self.root, "full-ui", "baseline")
        choice = execution_policy.acceptance(self.root, "non-rendered", "baseline")
        policy_file = self.root / "target/coverage/non-rendered/run-1/execution-policy.json"
        policy_file.parent.mkdir(parents=True)
        policy_file.write_text(json.dumps(execution_policy.resolve("non-rendered", {}) | {"acceptance": choice}))
        evidence = dict(mode="non-rendered", acceptance=choice, execution_policy=dict(
            path=str(policy_file.relative_to(self.root)), sha256=hashlib.sha256(policy_file.read_bytes()).hexdigest()))
        execution_policy.validate(evidence, self.root, "non-rendered", expected_acceptance=choice)
        with self.assertRaisesRegex(ValueError, "acceptance policy"):
            execution_policy.validate(evidence, self.root, "non-rendered", expected_acceptance={"policy": "target"})
        path.write_text(json.dumps(self.document) + "\n")
        with self.assertRaisesRegex(ValueError, "baseline"):
            execution_policy.validate(evidence, self.root, "non-rendered", expected_acceptance=choice)
        path.unlink()
        with self.assertRaises(FileNotFoundError):
            execution_policy.acceptance(self.root, "non-rendered")

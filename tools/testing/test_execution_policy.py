import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import execution_policy as policy
import run_coverage
from run_coverage import gate


class ExecutionPolicyTests(unittest.TestCase):
    def test_strict_flag_and_explicit_full_mode(self):
        self.assertEqual(policy.resolve("non-rendered", {})["ui_e2e_enabled"], False)
        for flag in ("", "true", " 1", "2"):
            with self.subTest(flag=flag), self.assertRaises(ValueError):
                policy.resolve("non-rendered", {"UI_E2E_ENABLED": flag})
        with self.assertRaises(ValueError):
            policy.resolve("full-ui", {})
        self.assertEqual(policy.resolve("full-ui", {"UI_E2E_ENABLED": "1"})["mode"], "full-ui")
        with self.assertRaises(ValueError):
            policy.resolve("unknown", {})

    def test_full_mode_rejects_before_any_build(self):
        with patch.dict("os.environ", {"UI_E2E_ENABLED": "0"}), \
             patch("sys.argv", ["run_coverage", "--base", "HEAD", "--mode", "full-ui"]), \
             patch.object(run_coverage, "command") as command:
            with self.assertRaises(ValueError):
                run_coverage.main()
            command.assert_not_called()

    def test_mode_and_original_capture_cannot_be_relabelled(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "target/coverage/non-rendered/run-1/execution-policy.json"
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps(policy.resolve("non-rendered", {})))
            evidence = dict(mode="non-rendered", execution_policy=dict(
                path=str(path.relative_to(root)), sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
            policy.validate(evidence, root, "non-rendered")
            with self.assertRaisesRegex(ValueError, "mode mismatch"):
                policy.validate(evidence, root, "full-ui")
            with self.assertRaisesRegex(ValueError, "outside its coverage mode"):
                policy.validate(evidence | {"mode": "full-ui"}, root, "full-ui")
            path.write_text(json.dumps(policy.resolve("full-ui", {"UI_E2E_ENABLED": "1"})))
            with self.assertRaisesRegex(ValueError, "changed since"):
                policy.validate(evidence, root, "non-rendered")

    def test_non_rendered_still_requires_real_host_and_lab(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            records = []
            for source in ("host", "lab"):
                path = root / f"{source}.profraw"
                path.write_bytes(source.encode())
                records.append(dict(path=str(path), source=source, tests=[source],
                                    sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
            self.assertEqual(gate.validate_evidence(dict(profiles=records), root, "non-rendered"), {"host", "lab"})
            with self.assertRaisesRegex(ValueError, "host and lab"):
                gate.validate_evidence(dict(profiles=records[:1]), root, "non-rendered")
            with self.assertRaisesRegex(ValueError, "executed UI"):
                gate.validate_evidence(dict(profiles=records), root, "full-ui")
            records[-1]["source"] = "ui"
            with self.assertRaisesRegex(ValueError, "cannot enter"):
                gate.validate_evidence(dict(profiles=records), root, "non-rendered")

    def test_deferred_is_not_a_fake_success_and_failure_is_not_optional(self):
        for change in ({"host_exit": 1}, {"lab_exit": 1}, {"ui_exit": 0}, {"ui_status": "passed"}):
            with self.subTest(change=change), self.assertRaises(ValueError):
                gate.validate_provenance(dict(host_exit=0, lab_exit=0, ui_exit=None,
                                              ui_status="deferred") | change, Path("/unused"), "non-rendered")

    def test_production_view_remains_in_scope_and_uncovered(self):
        path = "src/views/dialogs/partition.rs"
        self.assertEqual(gate.source_path(path, Path("/repo")), path)
        report, failures = gate.evaluate({path: {1: 0}}, {path: {(1, 1, "view"): 0}}, {path: {1}}, {})
        self.assertEqual(report["application"]["line_covered"], 0)
        self.assertIn(f"changed uncovered line: {path}:1", failures)

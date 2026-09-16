"""Exercise the real inventory validator with generated libtest identities."""
import importlib.util
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("ui_assert_tests", ROOT / "tools/ui-testing/assert_tests.py")
validator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validator)


class RstestSelectionTests(unittest.TestCase):
    def target(self, name):
        return dict(kind="rust-integration", package="test-backend", name="fixture_composition", tests=[name])

    def run_listing(self, name, listing):
        result = subprocess.CompletedProcess([], 0, listing, "")
        with patch.object(validator.subprocess, "run", side_effect=lambda command, **_:
                          subprocess.CompletedProcess([], 0, "0 tests\n", "")
                          if "--ignored" in command else result):
            validator.run_target(ROOT, self.target(name))

    def test_generated_identity_is_accepted_exactly(self):
        self.run_listing("async_fixture_override::case_1_partition", "async_fixture_override::case_1_partition: test\n")

    def test_parent_and_stale_case_cannot_select_descendants(self):
        for name in ("async_fixture_override", "async_fixture_override::case_2_partition"):
            with self.subTest(name=name), self.assertRaises(ValueError):
                self.run_listing(name, "async_fixture_override::case_1_partition: test\n")

    def test_duplicate_and_missing_names_fail(self):
        name = "async_fixture_override::case_1_partition"
        for listing in ("0 tests\n", f"{name}: test\n{name}: test\n"):
            with self.subTest(listing=listing), self.assertRaises(ValueError):
                self.run_listing(name, listing)

    def test_library_target_uses_real_libtest_selection_and_features(self):
        name = "update::tests::boundary::case_1_zero"
        target = dict(kind="rust-lib", package="cosmic-ext-storage", name="cosmic_ext_storage",
                      features=["test-backend"], tests=[name])
        result = subprocess.CompletedProcess([], 0, f"{name}: test\n", "")
        with patch.object(validator.subprocess, "run", side_effect=[
                result, subprocess.CompletedProcess([], 0, "0 tests\n", "")]) as run:
            validator.run_target(ROOT, target)
        self.assertEqual(run.call_args_list[0].args[0], ["cargo", "test", "-p", "cosmic-ext-storage",
                         "--locked", "--features", "test-backend", "--lib", "--", "--list"])
        self.assertEqual(run.call_args_list[1].args[0], run.call_args_list[0].args[0] + ["--ignored"])
        for listing in ("0 tests\n", f"{name}: test\n{name}: test\n"):
            with self.subTest(listing=listing), patch.object(validator.subprocess, "run",
                    return_value=subprocess.CompletedProcess([], 0, listing, "")):
                with self.assertRaises(ValueError):
                    validator.run_target(ROOT, target)

    def test_ignored_required_identity_and_failed_ignore_discovery_are_rejected(self):
        name = "async_fixture_override::case_1_partition"
        ordinary = subprocess.CompletedProcess([], 0, f"{name}: test\n", "")
        for ignored in (ordinary, subprocess.CompletedProcess([], 7, "", "listing failed")):
            with self.subTest(ignored=ignored.returncode), patch.object(validator.subprocess, "run",
                    side_effect=[ordinary, ignored]), self.assertRaises(ValueError):
                validator.run_target(ROOT, self.target(name))


if __name__ == "__main__":
    unittest.main()

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
        with patch.object(validator.subprocess, "run", return_value=result):
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


if __name__ == "__main__":
    unittest.main()

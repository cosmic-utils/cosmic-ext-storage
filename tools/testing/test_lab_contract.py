"""Host-only checks for the shared lab runner's checked-in execution contract."""
from pathlib import Path
import re
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]


class LabExecutionContractTests(unittest.TestCase):
    def test_storage_lab_group_is_serial_and_has_no_retries(self):
        config = tomllib.loads((ROOT / ".config/nextest.toml").read_text())
        self.assertEqual(config["test-groups"]["storage-lab"]["max-threads"], 1)
        profile = config["profile"]["storage-lab"]
        self.assertEqual(profile["retries"], 0)
        self.assertFalse(profile["fail-fast"])
        self.assertEqual(profile["slow-timeout"], {"period": "5m", "terminate-after": 1})
        self.assertIn({"filter": "package(storage-lab-tests)", "test-group": "storage-lab"}, profile["overrides"])
        self.assertTrue(profile["junit"]["store-failure-output"])

    def test_ci_local_command_uses_identical_image_digest_and_test_selection(self):
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        job = workflow.split("\n  storage-lab:\n", 1)[1].split("\n  ui-e2e:\n", 1)[0]
        self.assertIn('STORAGE_LAB: "1"', job)
        self.assertIn("run: just test-lab", job)
        self.assertNotIn("docker ", job)
        recipe = (ROOT / "justfile").read_text().split("\ntest-lab:\n", 1)[1].split("\n\n", 1)[0]
        self.assertIn("--file tools/storage-lab/Containerfile", recipe)
        self.assertIn("--profile storage-lab", recipe)
        self.assertIn("--test bridge --run-ignored ignored-only", recipe)
        self.assertNotIn("|| true", recipe)
        image = (ROOT / "tools/storage-lab/Containerfile").read_text()
        bases = re.findall(r"^FROM (\S+)", image, re.MULTILINE)
        self.assertEqual(len(bases), 2)
        self.assertEqual(bases[0], bases[1])
        self.assertRegex(bases[0], r"@sha256:[0-9a-f]{64}$")

    def test_native_selection_cannot_succeed_without_a_matching_test(self):
        runner = (ROOT / "tools/storage-lab/run-tests.sh").read_text()
        self.assertIn("--ignored --exact", runner)
        self.assertIn("--list", runner)
        self.assertIn("exit 64", runner)
        self.assertNotIn("|| true", runner)


if __name__ == "__main__":
    unittest.main()

"""Execution contracts for the shared Bash scripts; no host storage services.

Set STORAGE_SCRIPT_CONTAINER_TESTS=1 after building the existing UI/lab images
to also exercise container-only helpers in disposable, network-isolated containers.
"""
import io
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
CAPTURE = ROOT / "tools/testing/fixtures/capture-argv.sh"
UI = "tools/ui-testing/"
LAB = "tools/storage-lab/"


def run_script(path, *args, env=None):
    return subprocess.run(["bash", str(ROOT / path), *args], cwd=ROOT,
                          env=env, capture_output=True)


class ShellContractTests(unittest.TestCase):
    def test_named_scripts_use_bash_and_parse(self):
        scripts = [*ROOT.glob(f"{UI}*.sh"), *ROOT.glob(f"{LAB}*.sh")]
        self.assertEqual(len(scripts), 7)
        for script in scripts:
            with self.subTest(script=script.name):
                self.assertIn(script.read_text().splitlines()[0],
                              ("#!/bin/bash", "#!/usr/bin/env bash"))
                subprocess.run(["bash", "-n", str(script)], check=True)

    def test_launchers_preserve_arguments_isolation_and_exit(self):
        with tempfile.TemporaryDirectory(prefix="shell contract ") as directory:
            (Path(directory) / "docker").symlink_to(CAPTURE)
            for script, args, tail in [
                ("run-capability.sh", [], ["capability"]),
                ("run-case.sh", ["tests/ui/cases/live_scenario_reload.toml"],
                 ["execute", "tests/ui/cases/live_scenario_reload.toml", "false"]),
            ]:
                for status in (0, 7, 139):
                    with self.subTest(script=script, status=status):
                        env = dict(os.environ, PATH=directory + os.pathsep + os.environ["PATH"],
                                   UI_COVERAGE="0", SCRIPT_TEST_EXIT=str(status))
                        result = run_script(UI + script, *args, env=env)
                        self.assertEqual(result.returncode, status, result.stderr)
                        argv = result.stdout.decode().rstrip("\0").split("\0")
                        self.assertEqual(argv, [
                            "run", "--rm", "--network", "none", "--mount",
                            f"type=bind,src={ROOT},dst=/workspace,readonly", "--mount",
                            f"type=bind,src={ROOT}/ui-artifacts,dst=/workspace/ui-artifacts",
                            "-w", "/workspace", "cosmic-storage-ui-e2e:local", "/bin/bash",
                            "/workspace/tools/ui-testing/container-runner.sh", *tail])
            result = run_script(UI + "run-case.sh", "tests/ui/cases/live_scenario_reload.toml",
                                env=dict(env, UI_COVERAGE="1", SCRIPT_TEST_EXIT="0"))
            self.assertEqual(result.returncode, 0)
            self.assertTrue(result.stdout.endswith(b"true\0"))

    def test_invalid_launch_arguments_never_call_docker(self):
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / "docker").symlink_to(CAPTURE)
            env = dict(os.environ, PATH=directory + os.pathsep + os.environ["PATH"],
                       SCRIPT_TEST_EXIT="99", UI_COVERAGE="invalid")
            for script, args, status in [
                ("run-capability.sh", ["extra"], 64),
                ("run-case.sh", [], 64),
                ("run-case.sh", ["tests/ui/cases/../outside.toml"], 64),
                ("run-case.sh", ["/absolute.toml"], 64),
                ("run-case.sh", ["tests/ui/cases/missing.toml"], 1),
                ("run-case.sh", ["tests/ui/cases/live_scenario_reload.toml"], 64),
            ]:
                with self.subTest(script=script, args=args):
                    result = run_script(UI + script, *args, env=env)
                    self.assertEqual(result.returncode, status)
                    self.assertEqual(result.stdout, b"")

    def test_container_helpers_reject_invalid_arguments_before_side_effects(self):
        for script, cases in [
            (UI + "container-runner.sh", [[], ["unknown"], ["capability", "extra"],
                ["execute"], ["execute", "tests/ui/cases/../outside.toml", "true"],
                ["execute", "tests/ui/cases/live_scenario_reload.toml", "invalid"]]),
            (LAB + "collect-evidence.sh", [[], ["unknown"], ["devices", "extra"],
                ["services", "extra"], ["profiles"], ["profiles", ""],
                ["profiles", "../outside"], ["profiles", "x;exit 0"]]),
        ]:
            for args in cases:
                with self.subTest(script=script, args=args):
                    self.assertEqual(run_script(script, *args).returncode, 64)


@unittest.skipUnless(os.environ.get("STORAGE_SCRIPT_CONTAINER_TESTS") == "1",
                     "requires explicitly enabled disposable-container script checks")
class ContainerShellContractTests(unittest.TestCase):
    def container(self, image, command, mounts=(), status=0):
        argv = ["docker", "run", "--rm", "--network", "none", "--entrypoint", "/bin/bash",
                "--mount", f"type=bind,src={ROOT},dst=/workspace,readonly",
                "-e", f"SCRIPT_TEST_EXIT={status}"]
        for source, destination, readonly in mounts:
            argv += ["--mount", f"type=bind,src={source},dst={destination}" +
                     (",readonly" if readonly else "")]
        return subprocess.run([*argv, image, *command], capture_output=True, timeout=30)


class UiContainerShellContractTests(ContainerShellContractTests):
    def test_ui_bootstrap_executes_both_modes_and_preserves_exit(self):
        with tempfile.TemporaryDirectory() as directory:
            for mode in (["capability"], ["execute", "tests/ui/cases/live_scenario_reload.toml", "true"]):
                for status in (0, 7, 139):
                    with self.subTest(mode=mode, status=status):
                        result = self.container("cosmic-storage-ui-e2e:local",
                            ["/workspace/" + UI + "container-runner.sh", *mode],
                            [(directory, "/workspace/ui-artifacts", False),
                             (CAPTURE, "/opt/ui-test/bin/ui-e2e-runner", True)], status)
                        self.assertEqual(result.returncode, status, result.stderr)
                        argv = result.stdout.decode().rstrip("\0").split("\0")
                        if mode[0] == "capability":
                            expected = ["capability", "--scenario", "/workspace/tests/ui/scenarios/empty.toml",
                                        "--artifacts", "/workspace/ui-artifacts/capability"]
                        else:
                            expected = ["execute", "--case", mode[1], "--root", "/workspace",
                                        "--artifacts", "/workspace/ui-artifacts/executed", "--coverage", "true"]
                        self.assertEqual(argv, [*expected, "--app", "/opt/ui-test/bin/cosmic-ext-storage",
                            "--sway-config", "/workspace/tools/ui-testing/sway.conf",
                            "--environment-lock", "/workspace/tools/ui-testing/environment.lock.toml"])


class LabContainerShellContractTests(ContainerShellContractTests):
    def test_profile_archive_requires_instrumentation_and_matching_binary(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binaries = root / "bin"
            profiles = root / "profiles"
            binaries.mkdir()
            profiles.mkdir()
            (binaries / "storage-lab-fixture").write_bytes(b"matching ELF fixture")
            (profiles / "test.profraw").write_bytes(b"profile fixture")
            mounts = [(binaries, "/opt/storage-lab/bin", True),
                      (profiles, "/tmp/storage-lab-profiles", True)]
            for mode, target, success in [("0", "fixture", False), ("1", "missing", False),
                                           ("1", "fixture", True)]:
                with self.subTest(mode=mode, target=target):
                    (binaries / "coverage-mode").write_text(mode)
                    result = self.container("cosmic-storage-lab:local",
                        ["/workspace/" + LAB + "collect-evidence.sh", "profiles", target], mounts)
                    self.assertEqual(result.returncode == 0, success, result.stderr)
                    if success:
                        with tarfile.open(fileobj=io.BytesIO(result.stdout), mode="r:gz") as archive:
                            self.assertEqual(archive.extractfile("tmp/storage-lab-profiles/test.profraw").read(),
                                             b"profile fixture")
                            self.assertEqual(archive.extractfile("opt/storage-lab/bin/storage-lab-fixture").read(),
                                             b"matching ELF fixture")


if __name__ == "__main__":
    unittest.main()

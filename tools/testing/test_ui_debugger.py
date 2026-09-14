import json
from pathlib import Path
import runpy
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "ui-testing/debug-app.py"


class DebuggerTests(unittest.TestCase):
    def load(self, root, frames=()):
        callbacks = {}
        commands = []
        class Frame:
            def __init__(self, index):
                self.index = index
            def name(self):
                return frames[self.index]
            def pc(self):
                return 42
            def older(self):
                return Frame(self.index + 1) if self.index + 1 < len(frames) else None
        fake = SimpleNamespace(
            events=SimpleNamespace(
                exited=SimpleNamespace(connect=lambda f: callbacks.update(exited=f)),
                stop=SimpleNamespace(connect=lambda f: callbacks.update(stopped=f))),
            newest_frame=lambda: Frame(0) if frames else None,
            solib_name=lambda _: "libwayland-client.so.0",
            execute=lambda command, **_: commands.append(command))
        env = {"UI_DEBUG_REPORT": str(root / "debugger.json"), "UI_CLOSE_MARKER": str(root / "close-requested")}
        with patch.dict(sys.modules, {"gdb": fake}), patch.dict("os.environ", env):
            runpy.run_path(str(SCRIPT))
        return callbacks, commands, env

    def test_clean_and_early_exit_are_distinct(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            callbacks, commands, env = self.load(root)
            with patch.dict("os.environ", env):
                callbacks["exited"](SimpleNamespace(exit_code=0))
                self.assertFalse(json.loads((root / "debugger.json").read_text())["close_requested"])
                (root / "close-requested").touch()
                callbacks["exited"](SimpleNamespace(exit_code=0))
            result = json.loads((root / "debugger.json").read_text())
            self.assertEqual(result["kind"], "exited")
            self.assertEqual(result["exit_code"], 0)
            self.assertTrue(result["close_requested"])
            self.assertFalse(commands)
            self.assertFalse((root / "debugger.tmp").exists())

    def test_signal_preserves_stack_and_cannot_be_overwritten_by_kill(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            callbacks, commands, env = self.load(root, ["wl_proxy_destroy", None])
            (root / "close-requested").touch()
            with patch.dict("os.environ", env):
                callbacks["stopped"](SimpleNamespace(stop_signal="SIGSEGV"))
                callbacks["exited"](SimpleNamespace(exit_code=0))
            result = json.loads((root / "debugger.json").read_text())
            self.assertEqual(result["signal"], "SIGSEGV")
            self.assertEqual(result["frames"][0]["function"], "wl_proxy_destroy")
            self.assertEqual(result["frames"][1]["function"], "")
            self.assertEqual(commands, ["thread apply all bt 80", "kill", "quit 139"])

    def test_unknown_stops_and_bounded_stacks_are_recorded_not_retried(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            callbacks, commands, env = self.load(root, ["unknown"] * 100)
            with patch.dict("os.environ", env):
                callbacks["stopped"](SimpleNamespace())
            result = json.loads((root / "debugger.json").read_text())
            self.assertIsNone(result["signal"])
            self.assertEqual(len(result["frames"]), 80)
            self.assertNotIn("continue", commands)
            self.assertFalse(result["close_requested"])


if __name__ == "__main__":
    unittest.main()

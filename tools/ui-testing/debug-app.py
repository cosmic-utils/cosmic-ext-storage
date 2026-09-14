"""GDB-side diagnostics for the one owned UI inferior; never retries an action."""
import json
import os
from pathlib import Path

import gdb


terminal_signal = False


def write_report(report):
    path = Path(os.environ["UI_DEBUG_REPORT"])
    report.update(schema_version=1, close_requested=Path(os.environ["UI_CLOSE_MARKER"]).is_file())
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(report, indent=2) + "\n")
    temporary.replace(path)


def exited(event):
    if not terminal_signal:
        write_report(dict(kind="exited", exit_code=getattr(event, "exit_code", None), signal=None, frames=[]))


def stopped(event):
    global terminal_signal
    terminal_signal = True
    frames = []
    frame = gdb.newest_frame()
    while frame is not None and len(frames) < 80:
        frames.append(dict(function=frame.name() or "", library=gdb.solib_name(frame.pc()) or ""))
        frame = frame.older()
    write_report(dict(kind="signal", exit_code=None, signal=getattr(event, "stop_signal", None), frames=frames))
    # Preserve the full stack without locals/arguments. No host core collector,
    # ptrace capability, attach-to-unrelated-process, or signal suppression.
    gdb.execute("thread apply all bt 80")
    gdb.execute("kill", to_string=True)
    gdb.execute("quit 139")


gdb.events.exited.connect(exited)
gdb.events.stop.connect(stopped)

# UI testing commands

`just ui-e2e` is the environment capability check, not the eight-case suite.
It proves the image-built application exposes an accessibility tree and that
the private Sway session can capture pixels and receive virtual keyboard input.

Capability and executed-case commands share `container-runner.sh`. The host
launchers and container helpers use Bash in normal and instrumented execution;
the pinned runtime already supplies Bash. Both commands mount source read-only
and only `ui-artifacts` writable. `input-probe.sh` is the named child program
used by the virtual-keyboard check, not a second test runner. The capability
recipe also runs disposable-container bootstrap argument/exit contract tests,
using the same image locally and in CI. Those test doubles are contract checks,
not evidence of application coverage.

`just ui-e2e-case tests/ui/cases/live_scenario_reload.toml` runs a version-2
program against the actual application through AT-SPI. Fresh artifacts under
`ui-artifacts/executed` include actions completed, control responses, trees,
screenshots, and separate functional/shutdown results. The workspace is mounted read-only; only
the artifact directory is writable. No host bus, home, network, or devices are
shared. A successful semantic result is `semantic_passed`, not full visual
acceptance; baseline approval and comparison remain unimplemented.

The target-ID routing bug is fixed on the existing dependency base. A separate
iced Wayland teardown crash is narrowly quarantined; see
[the diagnosis](../../docs/plans/5-testing-v2/ui-action-routing-blocker.md).
The other seven manifests are still inventory-only version 1 and are rejected
by the execution command. Required-test metadata remains explicitly planned.

Do not update goldens or declare coverage from the capability/inventory checks.

## Durable shutdown reporting

Executed cases start their owned app under the image's pinned GDB, without
host core access, an extra ptrace capability, or a second execution. The
debugger captures the crashing thread's stack, never retries/continues an
action, and returns failure for signals. The runner saves `functional.json`,
`control.json`, the final accessibility tree, and `final.png` **before** asking
the app to close. It then saves `debugger.json`, `shutdown.json`, and the
combined schema-2 `execution.json`.

- `semantic_passed`: all functional gates and clean shutdown passed.
- `semantic_passed_with_known_shutdown_failure`: functional gates passed;
  only the diagnosed, scoped Wayland teardown stack was quarantined. Command
  exit is zero with an explicit warning, not a claim of clean shutdown.
- `failed`: any functional failure, early crash, unknown stack/signal,
  missing diagnostics, unsuccessful close request, or shutdown timeout.

`shutdown-quarantine.toml` pins the exact case, environment and Cargo.lock
hashes, an owner, and expiry (2026-10-13 UTC). Scope changes fail closed and
require review; do not automatically refresh the hashes or extend expiry.
The matcher requires SIGSEGV after the close marker and an ordered stack
through `wl_proxy_destroy`, the Wayland backend's connection destructor, and
iced's SCTK calloop source cleanup. Signal number alone never qualifies.

On failure the runner terminates its owned debugger/app process group; the
outer container is removed as usual. The debugger does not suppress crashes
or pretend a killed app flushed LLVM counters. Coverage remains independent:
missing/empty profiles and missing executed UI coverage still fail acceptance.

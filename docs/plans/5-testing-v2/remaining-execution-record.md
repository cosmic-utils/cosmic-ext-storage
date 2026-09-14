# Testing V2 remaining execution

Status: initial regression batch implemented; shell collector integration
awaits the interpreter decision below. No final acceptance or coverage
completion claimed.

Started on 2026-09-14 from `4-ui-testing` commit
`5ff5e049756c5389f74a2bb0622b9612c219730f`, on branch
`codex/testing-v2-completion`. The only pre-existing edit was the approved
remaining-phase implementation-plan update; it is preserved on this branch.
Completed phases and historical evidence are not being rewritten.

Branch correction (2026-09-14): at the user's request, fast-forwarded both
local commits (`455fff7`, `9cec518`) into `4-ui-testing` and deleted the local
`codex/testing-v2-completion` branch. It had never been pushed and had no
remote branch or PR. All subsequent implementation belongs directly on
`4-ui-testing`; another branch requires an explicit request. The active plan
now reflects this, including final coverage comparison against the existing
PR's actual base rather than against `4-ui-testing` itself.

## Baseline and early feasibility

Evidence is saved under ignored `target/testing-v2-completion/`.
Current Cargo/Nextest discovery, strict Clippy and host baseline gates passed:
282 host tests, 36 explicitly ignored native/probe entries, 26 Python tests,
and 80 required inventory entries before this batch. Formatting passed.
The fresh combined run `run-eu4hrjun` used the exact starting commit as
its changed-code base and retained valid source/input/ELF evidence. Host,
native and UI exit statuses were zero. All 17 instrumented native cases
passed in 141.493s (Nextest `06f120d6-a713-4758-80c7-ecebb083cef6`).
Reload completed all nine steps and matched the unchanged known shutdown
quarantine. The separate normal UI capability check also passed.

Rust baseline: 11,547/29,348 lines (39.35%) and 1,336/3,471 functions (38.49%).
Acceptance failed on all ten package/aggregate scopes and the seven absent
executed UI cases. There were no changed-code failures when comparing the
unchanged starting code to its exact base; this is not a waiver of the final
changed-code gate. Nine fewer lines were observed than in the dated adoption
run, with the same definitions and covered function count; do not claim a
regression or a gain from that runtime-dependent variation without a
source-aligned rerun. The baseline reports were saved before source edits.

Full per-file missing line/function identities are in
`baseline/gap-ledger.json` (SHA-256
`ce0ea3c8f63333a09ad3e45cdb5785edf1d21620705bd4d38304aa1c9b11dfd5`).
See [the gap ledger](remaining-gap-ledger.md) for scope totals and work owners.

Before large gap-filling batches, evaluate established non-Rust collectors
with small success/failure/unexecuted-code probes. This work must not invent
metrics for unsupported source or treat a passing script test as coverage.
No GUI/toolchain/Rust dependency pins, quarantine scope or thresholds changed.

## Non-Rust collector proof

Python: installed `coverage==7.16.1` only in an ignored local virtualenv.
Its JSON format 3 records named function regions including an uncalled
function with zero body hits. Both a successful call and a deliberate
exception retain usable coverage, and the exception exits 1. The existing
26 Python tests pass under instrumentation. Their line measurements are:

| Script | Covered / executable lines |
| --- | ---: |
| `tools/testing/coverage.py` | 202 / 273 |
| `tools/testing/run_coverage.py` | 137 / 287 |
| `tools/ui-testing/assert_tests.py` | 32 / 179 |
| `tools/ui-testing/debug-app.py` | 27 / 27 |

These are unit-suite observations only. The debugger test uses its existing
fake GDB module; this is not new live-GDB collection evidence. Child processes,
container execution, source inventory and function-metric validation still
need integration before final support coverage can pass.

Shell: the upstream v43 release has no matching Docker `v43` tag. Tested the
available official `kcov/kcov:v42` image by immutable digest
`sha256:30c442617f3d8e040bf0ec2cba19cc2ee517b668f3a3d50b2d3de1c435138a8a`;
its binary reports `kcov v41-31-g3a8c`. This is a probe dependency, not an
adopted project pin or a replacement for either pinned application image.

- Success/failure probe: exits 0/7 preserved, uncalled function body remains
  uncovered, reports contain 4/7 and 5/7 hit lines respectively.
- Interpreter probe: normal `/bin/sh` prints `interpreter=posix-sh`; kcov
  prints `interpreter=5.1.4(1)-release`. It substitutes Bash for the shebang.
- The inline `sh -ec` child prints success, but its body lines remain marked
  uncovered in kcov. This is not valid evidence that the child did not run,
  and must not be hidden with an exclusion.
- `--bash-parser=/bin/dash` preserves the interpreter and exits 0, but reports
  0/9 hit lines despite observed execution. This alternative fails feasibility.
- Kcov's Cobertura output supplies lines, not a verified function inventory.
  Current maintained `.sh` scripts declare no shell functions; do not turn
  that observation into a general function-coverage claim for future helpers.

The [kcov manual](https://raw.githubusercontent.com/SimonKagstrom/kcov/v43/doc/kcov.1)
documents Bash parsing and optional `/bin/sh` interception via substitution.
[ShellSpec's coverage documentation](https://github.com/shellspec/shellspec#code-coverage)
restricts measurement to shells with DEBUG traps (Bash, zsh, ksh), so adding
that runner does not solve Dash measurement. Python format/reference:
[coverage.py JSON reporting](https://coverage.readthedocs.io/en/7.16.1/commands/cmd_json.html).

Decision approved (2026-09-14): explicitly standardize maintained runtime test
scripts on Bash in normal and instrumented runs, extracting inline child-shell
code into tracked scripts and preserving one execution path. No second runner,
custom trace collector, threshold reduction or unsupported metric is approved.
Probe sources/reports remain under the ignored evidence directory; they have
not become a maintained execution mechanism. Build recipes still need their
own measurement-boundary audit.

## First deterministic regression batch

Moved the old `models::helpers::tests::{test_build_simple_tree,test_build_nested_tree}`
assertion bodies into scenario-feature-gated integration target `volume_models`.
The functions retain their names; their target/module identity intentionally
changes. Removed the old optional host-D-Bus constructor and early returns.
The new rstest fixture derives `FilesystemsClient` from the existing owned
root `scenario` fixture. No product algorithm or storage transport changed.

Added four named empty/foreign/orphan/anonymous-root cases and one coherent
nested mutation sequence covering lookup, ownership-preserving clone,
mount-state queries, updates and direct/nested removals. Cargo and Nextest
both pass all seven cases with no skips. Added their exact discovered names
to the required manifest and its active validation appendix. Default-feature
builds explicitly do not include this scenario-only target; ordinary CI's
all-feature suite and mandatory named inventory select it.

After the batch: workspace Nextest passes 287 tests (36 explicit ignored
native/probe entries), default library tests pass 39, strict workspace
all-feature/all-target Clippy passes, Python passes 26, all 87 required names
validate, and formatting/diff checks pass. The seven model cases also pass
with `DBUS_SYSTEM_BUS_ADDRESS=unix:path=/nonexistent/cosmic-test-bus`, proving
the removed environmental dependency cannot silently bypass their assertions.
Logs are `models-no-dbus.log`, `models-clippy.log`, `default-tests.log` and
`host-after-models.log` in the evidence directory.

This batch does not claim a new full coverage percentage. Fresh final
instrumentation is required after these test/source/manifest changes. The
remaining seven UI cases, non-Rust integration, threshold work, hosted failure
probes, visual review and required-check administration are still unfinished.

## Approved Bash/runtime extraction

Implemented directly on `4-ui-testing`, with no new branch:

- Native entrypoint and exact-test launcher now explicitly use `/bin/bash`,
  retaining `set -eu` rather than silently adding pipeline-failure behavior.
- Replaced the duplicated UI bootstrap with `container-runner.sh`, called by
  `run-capability.sh` and `run-case.sh`. Both now mount source read-only and
  artifacts writable. Case arguments reject traversal and invalid coverage
  booleans before starting the container.
- Extracted lab device/service diagnostics and profile archiving into
  `collect-evidence.sh`. Rust still owns Testcontainers lifecycle, exit checks,
  cleanup interpretation and exec-stream archive transport before teardown.
  Archive targets are passed as arguments and validated, not interpolated
  into shell source. The diagnostic best-effort behavior is unchanged.
- Extracted the terminal input probe into `input-probe.sh`. Removed all four
  runtime `sh -ec` snippets from the two Rust runners and the duplicated UI
  snippet from Just/the case launcher. No application/GUI dependency, UI image
  lock or shutdown-quarantine scope changed. Build-time Containerfile commands
  remain separately inventoried work, not silently excluded coverage.
- Added four host contract tests and two explicitly container-only tests to
  the existing Python unittest suite. Host discovery runs 30 checks and marks
  the two container checks skipped; the existing `test-lab` and `ui-e2e`
  recipes each enable their matching container check locally and in CI.
  Explicit local execution of both container checks passes. They cover both UI
  modes, exact argv/mount isolation, 0/7/139 exit propagation, invalid arguments,
  and profile archive success/non-instrumented/missing-binary outcomes.

Normal execution after the runtime extraction: 17 native cases pass in
116.753 seconds (one deliberate ignored probe remains unselected), capability
passes, and all nine reload steps pass with the unchanged known-shutdown
quarantine warning. Strict workspace/all-feature/all-target Clippy, Bash syntax,
Rust formatting and diff checks pass. Logs are under the ignored
`target/testing-v2-completion/bash/` evidence directory.

The same immutable kcov probe image now observes both named children when
started by a parent Bash script: `input-probe.sh` has 3/3 executable lines hit;
the services-only invocation of `collect-evidence.sh` has 4/14, retaining device
and profile branches as uncovered. Parent success/failure exits remain 0/7.
This resolves the specific inline-child measurement problem. It does **not**
adopt that image as the runtime, establish a shell function metric, measure
across Docker/privilege boundaries, or pass the support-code coverage gate.
The probe's Bash 5.1.4 is not the pinned runtime's Bash 5.2.15; actual collector
integration in that runtime remains required.

Fresh combined instrumentation after the refactor: `run-wd_l49sl`, with
`just coverage origin/main` resolving the comparison base to
`0ba27cc2caac19acb7a8d98747f8b65058dab877`. Host, native and UI execution exit
codes are all zero; native runs 17 tests in 144.213 seconds. The deliberate
inner-failure case also supplies its archive before outcome checking. The
report accepts 89 profiles in 12 matching build groups, including real app
and runner UI profiles. No build-input/provenance/transport failure occurred.

Rust totals: **11,682 / 29,342 lines (39.81%)**, **1,360 / 3,471 functions
(39.18%)**. These include the preceding model-test batch; they are not a claim
that extracting shell code alone increased application coverage. Runtime
shell extraction also changes Rust source line locations/definitions, so the
previous raw denominator is not identical.

Acceptance correctly exits 1: seven missing executed UI cases (one diagnostic),
ten package/aggregate threshold diagnostics, 4,950 changed uncovered lines
and 707 changed uncovered functions. The changed-code findings compare the
whole branch against `main`; the initial baseline compared against its own
starting commit, so its eleven diagnostics are not directly comparable to
these 5,668. No threshold was relaxed. Complete non-Rust support coverage,
remaining UI programs and later approval/admin gates are still outstanding.
Saved report copies are under `target/testing-v2-completion/bash/reports/`;
full raw matching profiles/build groups remain in `target/coverage/run-wd_l49sl`.

## Continued execution: UI runtime and focus

The next executable keyboard probe exposed and fixed the case-name-dependent
Sway Unix-socket overflow. The runner now owns a short private temporary runtime
directory with the existing pinned `tempfile` crate, preserving artifact paths
and teardown ownership. Two mandatory rstest regressions pass; all 70 runner
tests, strict Clippy, normal capability and the original reload flow pass.
Cargo.lock and GUI/image/quarantine bindings are unchanged.

The next gate is genuinely blocked on widget-focus publication in pinned iced:
every accessibility update hardcodes focus to the window root. The keyboard
inventory remains unconverted; the exploratory program and failed artifacts
are preserved. See [the diagnosis, upstream-fix audit and scoped proposal](keyboard-focus-blocker.md).
No dependency repair, new PR, focus-assertion waiver or quarantine expansion
has been made. Fresh combined coverage is required after these source changes.

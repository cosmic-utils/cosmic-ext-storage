# Testing V2 remaining execution

Status: initial regression batch implemented; shell collector integration
awaits the interpreter decision below. No final acceptance or coverage
completion claimed.

Started on 2026-09-14 from `4-ui-testing` commit
`5ff5e049756c5389f74a2bb0622b9612c219730f`, on branch
`codex/testing-v2-completion`. The only pre-existing edit was the approved
remaining-phase implementation-plan update; it is preserved on this branch.
Completed phases and historical evidence are not being rewritten.

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

Decision requested: explicitly standardize the maintained test scripts on
Bash in normal and instrumented runs and extract inline child-shell code
into tracked scripts, preserving one execution path; or retain Dash and
continue collector investigation. No interpreter change, second runner,
custom trace collector, threshold reduction or unsupported metric is being
silently introduced. Probe sources/reports remain under the ignored evidence
directory; they have not become a maintained execution mechanism.

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

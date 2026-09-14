# rstest adoption execution record

Status: implementation, local acceptance and hosted CI acceptance complete.

Started 2026-09-14 on `codex/rstest-adoption`, from
`12fb884e2d462b91cacb1b097e8fa35826f84345`. Starting worktree contained only the
untracked future task, rstest specification and implementation plan; preserved.
Rust 1.95.0, Nextest 0.9.144, Docker 29.8.0. No toolchain or GUI pin update.

## Baseline

Evidence directory: `target/rstest-adoption/baseline/` (ignored local artifacts).
Contains the original lockfile, normal/build dependency graphs, original test
source archive, Cargo and JSON Nextest discovery, image IDs and command logs.
Every migration is compared to this source archive and discovered inventory.

- Workspace Nextest: 213 passed, 32 skipped (native/feature-gated helpers).
- Default tests, scenario-feature targets, no-default-feature checks (with and
  without scenarios), strict all-targets Clippy, fmt, 23 Python tests and required
  inventory validation passed. All commands used the pinned toolchain/lockfile.
- Fresh coverage run `run-s8o1doqa`: host/lab/UI exits all zero; 80 profiles;
  16 native cases passed in 189.645s (Nextest
  `5027b085-a16c-47af-8666-cf56914c8109`). Reload executed all nine steps and
  matched the existing known-shutdown quarantine. Artifact:
  `ui-artifacts/executed/live_scenario_reload-12-1789407522351849024`.
- Fresh baseline: 11,556/29,351 lines (39.37%), 1,337/3,472 functions (38.51%).
  Acceptance exit 1: 5,661 failures, including the seven missing executed UI
  sources and existing threshold/changed-source failures. Saved reports and
  provenance are in the baseline directory, with `SHA256SUMS`.
- Existing `tests/unit/models/helpers_tests.rs` D-Bus setup can return early;
  a green libtest result does not prove those assertions executed. Not migrated.

## Scope and ownership decisions

Scenario fixtures own their runtime; clients are derived from that runtime.
Workflow scheduling, native `LabFixture`, ancestry checks, checked teardown,
Nextest supervision and Python report tooling remain in their existing layers.
The control tests' PID-based sockets/tokens are included in temporary-resource
cleanup, with server/runtime release before deleting the temporary root.

## Host migration

- Maintained composition tests: eight pass under Cargo and Nextest; two ignored
  deliberate-failure helpers are explicitly executed/checked by passing parents.
- Lock resolution added only rstest 0.27.0, rstest_macros 0.27.0 and relative-path
  1.9.3. Both normal/build dependency graph comparisons are byte-identical to the
  baseline (default and all-features). GUI/runtime packages were not upgraded.
- Repeated scenario/workflow setup now uses owned fixtures. Private control
  tests share the test-only path/temporary-root helpers, but retain their explicit
  server construction. Success/failure and dual-world workflow contexts remain
  sequential and explicit while reusing the same fixture constructor.
- Control tests now await socket disappearance after releasing the server/runtime,
  before their owned root is removed. This checks the existing shutdown behavior;
  it does not introduce a production lifecycle redesign.
- Phase-2 workspace Nextest: 276 passed before the final six property cases;
  26 Python tests passed. Final matrix and native gates are still pending.
- Renamed awaited mutable arguments required immutable injection followed by a
  local mutable binding on rstest 0.27.0; the spec records this validated pattern.

## Native migration and selection

- Host suite now has 282 passing tests, including the final six property rows;
  all 80 mandatory inventory entries validate. Three new Python selection tests
  exercise the real validator's generated-name, stale-name and duplicate checks.
- Original scenario/workflow/inner native names are unchanged. The 15 bridge
  cases have verified zero-padded `case_01`–`case_15` names, recorded in the map.
- `run_inner_test` was removed after replacing its last callers: it was only a
  capability-target forwarding alias. Its behavior remains in `run_inner_case`.
  This is the only operational Rust source deletion; coverage comparison must
  align the shifted lines and explicitly account for this deleted wrapper.
- The new `selection.rs` inner test invokes the real selection script with a
  missing filter and asserts exit 64. Its outer test follows the normal isolated
  bridge path. This deliberately adds one container (17 total), while keeping a
  real parent profile/ELF; no coverage exception for missing profiles is needed.
- Required-case validation also checks documentation. Added the new identities
  to the active validation appendix instead of weakening that check.

## Native execution result

Normal native run: 17/17 passed in 75.043s, one host-only test excluded from
the ignored-only native selection. JUnit is saved as
`target/rstest-adoption/phase4/bridge.junit.xml`; full log is `phase4/lab.log`.
All 15 forwarding cases, the dedicated deliberate-failure case and the new
selection case executed. The new selector parent took 0.781s.

This normal run is not a speed comparison against the instrumented baseline.
The final instrumented run will provide comparable case timings. No retries,
parallel privileged tests, Cartesian products or weakened leak checks were added.

## Final validation

Normal/build dependency graphs remain byte-identical to baseline with all four
packages' dev dependencies added. Final Cargo.lock SHA-256:
`11070a571c8806d59f456d1edbeb65d428b28bbd256e6c28a3d3e7f4256ced0d`.
Quarantine binding changed only to that reviewed dev-only lock hash; all other
bindings, owner, expiry and stack classification remain unchanged. Normal and
instrumented UI runs must validate it before final acceptance.

Normal UI revalidation passed all nine steps and exited cleanly, without using
the quarantine, at
`ui-artifacts/executed/live_scenario_reload-12-1789408468008827618`.
Strict Clippy also passed after the hash rebind (`final/clippy-rebind.log`).

Instrumented native revalidation: 17/17 passed in 212.640s, Nextest run
`12577771-f1a2-457a-b70a-ad2e6da88b7e`, in coverage run `run-q1y_8rpa`.
The new selection regression took 1.155s; the original 16 cases account for
211.485s versus 189.645s before (about 11.5% slower in this pair of runs).
`final/native-comparison.json` records every case timing and baked ELF size.
Existing ELF growth is below 0.22%; the new selector ELF is about 7.5 MB.
The run contains exactly the intended 17 container cases, with no retry or
parameter-product increase. Timings include container/evidence I/O, and case
order changed with the new names. This single pair does not establish a causal
fixture overhead or a performance win; no speedup is claimed and no safety or
coverage work was removed to improve the timing.

Publication and merge were authorized on 2026-09-14. The adoption branch was
submitted as [app PR #120](https://github.com/cosmic-utils/cosmic-ext-storage/pull/120)
against `4-ui-testing`; all hosted checks must pass on the final head before merge.
No libcosmic PR or branch-protection change is authorized or needed.

## Hosted acceptance

[CI run 34879591862](https://github.com/cosmic-utils/cosmic-ext-storage/actions/runs/34879591862)
passed all seven jobs on 2026-09-14 for exact branch revision
`1539e8c0a571ec143c89f9a00f41ce2f26967890`: Rust tests, Clippy, Rustfmt,
UI scenario contract, application workflow integration, Storage lab and
UI E2E capability. No CI fixes, retries, relaxed checks or workflow changes
were needed. The initial run on `58e37ca` was superseded by a documentation
correction and automatically cancelled, not treated as acceptance evidence.

The hosted native run executed all 17 intended outer cases successfully in
60.451s, including deliberate-failure propagation and stale-selector rejection.
The Python contract suite passed all 26 tests. UI capability and the executed
live reload regression passed; the latter reported `semantic_passed` and
uploaded evidence as `live_scenario_reload-12-1789410237008078711`.
Native and UI evidence are attached to the run as `storage-lab-artifacts`
and `ui-artifacts` respectively. This closes the rstest hosted-validation
gate, not the wider Testing V2 coverage/visual-approval gates. The documentation
commit recording this evidence must itself pass existing CI before merge;
the PR checks provide that final-head audit trail.

## Final coverage and handoff

Fresh final run `run-q1y_8rpa` collected 88 real profiles. Host, native and UI
exit statuses were all zero. Independently revalidated source/input/ELF
provenance (`target/rstest-adoption/final/provenance.log`). Instrumented reload
executed all nine steps and matched the unchanged known crash signature at
`ui-artifacts/executed/live_scenario_reload-12-1789408941449314681`.
Both application and runner profiles were required and collected.

| Measure | Fresh baseline | Fresh final |
| --- | --- | --- |
| Covered / executable lines | 11,556 / 29,351 | 11,556 / 29,348 |
| Covered / executable functions | 1,337 / 3,472 | 1,336 / 3,471 |
| Line percentage | 39.37% | 39.38% |
| Function percentage | 38.51% | 38.49% |
| Coverage acceptance failures | 5,661 | 5,659 |
| Host Nextest pass count | 213 | 282 |
| Selected native pass count | 16 | 17 |

The source-aligned comparison found **no lost covered line or function in
retained source**. The deleted capability forwarding alias accounts for three
executable/covered lines and one executable/covered function. Three additional
lines were observed: lab cleanup lines 491–492 and process discovery line 73.
Those runtime-dependent observations are not claimed as coverage added by
parameterization. Every other source definition and covered set is unchanged.
`final/coverage-comparison.json` contains the exact sets and deleted definition.

Final acceptance still exits 1: the same seven required executed UI sources
are missing and the existing 98–100% coverage targets remain unmet. The two
fewer failures correspond to newly observed changed cleanup lines. No report
boundary, threshold, source requirement or exception policy was weakened.

Final local commands passed: fmt, strict workspace/all-target Clippy, workspace
Cargo and Nextest tests, default-feature root tests, explicit feature-disabled
contracts, scenario/control/workflow targets, backend tests, no-default-feature
checks with and without scenarios, 26 Python tests and all 80 required inventory
entries. A generated case also ran individually under Cargo and Nextest.
The 36 ignored host-discovery entries are explicit native targets and deliberate
failure probes; they are not claimed as host executions. Native selection excludes
one host-only bridge test that is covered by the host suite.

Cleanup audit: duplicate path builders, PID directories/sockets, numbered
diagnostic dispatch, forwarding wrappers and the unused capability alias are
removed. Specialized backend/overlay/secrets setup remains explicit where its
ownership or constructor behavior is itself under test. No operational logic
was moved into excluded test paths. Discovery and test output introduced no
secret-bearing case names; existing redaction tests still pass.

Full logs/listings, per-case timings, scoped JSON/LCOV, acceptance results and
provenance are saved under `target/rstest-adoption/{baseline,phase1,phase2,phase4,final}`.
Baseline and final report digests are in their respective `SHA256SUMS` files.
The temporary comparison scripts are audit artifacts under ignored `target/`,
not another maintained runner or a build dependency.

Adoption acceptance is complete. Operational handoff is merge into
`4-ui-testing` after final-head CI, followed by local synchronization. Wider Testing
V2 completion, the seven missing UI cases, visual approval, non-Rust coverage
and required coverage enforcement remain separate unfinished obligations.

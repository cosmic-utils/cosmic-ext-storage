# Testing V2 regrouping: production logic first, rendered UI opt-in

Status: agreed direction; implementation plan, not completed work.
Date: 2026-09-15. Continue on `4-ui-testing`; no new prototype branch.

## Decision and precedence

Pause rendered-UI acceptance and further dependency patching. Prioritise real
backend tests and tests of the business logic actually used by the application,
without creating a window, compositor, accessibility bus or screenshot.

This document supersedes the outstanding execution order and mandatory
rendered-UI requirements in `implementation-plan.md` and `spec.md` for this
paused period. It does not rewrite completed history, waive native safety,
declare the original eight-flow UI acceptance complete, or lower coverage
targets. Historical rstest plans remain untouched.

No AccessKit backport is authorised by this plan. Stop the pending proposal,
do not upgrade dependencies, and do not open upstream PRs. Re-enabling rendered
testing later is a deliberate dependency-compatibility decision, not a promise
that a future release will automatically fix every blocker.

## What exists, and what it proves

| Layer | Current implementation | Keep active? |
| --- | --- | --- |
| Rust units and contracts | Backend parsing/value tests; application models, validators and some real update helpers | Yes; deepen assertion quality and production-path coverage |
| Scenario workflows | `test-backend`, `WorkflowHarness`, rstest fixtures and `tests/application_workflows.rs` | Yes; migrate away from parallel test-only orchestration |
| Native integration | `storage-lab-tests`, private Testcontainers services and owned loop-backed devices | Yes; keep one local/CI mechanism |
| Rendered UI | `ui-e2e-runner`, Sway, AT-SPI, virtual input, screenshots and instrumented UI runs | Default off; preserve as opt-in diagnostic infrastructure |

The distinction is execution, not filenames: tests named `ui_*` may be useful
non-rendered tests. Existing `ui-test` enables embedded resources and is not an
E2E execution switch; `test-backend` remains available. Compiling libcosmic and
its system libraries may still be necessary for root application tests. This
plan promises no rendering, not a GUI-dependency-free build.

### Critical production-logic gap

`src/state/app.rs::reduce_*_workflow` methods are feature-gated by `test-backend`
and their callers are in `src/testing/workflow_harness.rs`. The normal
`src/update/*` message handlers follow separate paths. Passing scenario workflow
tests therefore does not establish coverage of those normal handlers.

For example, `src/update/mod.rs::execute_confirmed_logical_action` performs
confirmation/generation checks and schedules the real operation; the harness
drives a separate logical reducer. `verify_workflow_facade_contract` merely
checks a hard-coded list of five route names. It is not production wiring
evidence. Some older contract tests similarly check constants or construction
rather than the behavior their names imply.

## Phase A — Preserve the checkpoint and freeze dependency work

1. Inventory the dirty app files, existing app/fork commits, running task-owned
   test processes and saved evidence before editing. Preserve unrelated work
   and dependency stashes. Do not cancel other users' processes or CI runs.
2. Record fixes already made and their upstream references in the existing
   dependency ledger. Retain the archived diagnostics and failure evidence.
   Mark the AccessKit proposal paused, not approved or resolved.
3. Keep the current exact dependency pins during this regrouping. Disabling
   rendered tests is not authority to revert accessibility fixes or discard
   uncommitted app changes. Inventory fork-dependent app APIs so a future
   rollback can be reviewed as its own coherent change.
4. Checkpoint verified work separately from incomplete rendered-test work.
   In particular, startup secret ingestion is only partial infrastructure:
   do not claim live LUKS coverage or continue building its rendered transport
   during this pause. Record retained/deferred code and verification status.

Acceptance: cleanly documented starting revision/worktree and frozen pins;
no new dependency repair, upstream PR, quarantine rebind or expiry extension.

## Phase B — One explicit rendered-UI execution switch

Introduce `UI_E2E_ENABLED`, accepting exactly `0` or `1`, default `0`. Share
validation across the existing local recipes, scripts and CI entry points;
do not introduce a second test runner or use a Cargo feature as this switch.
`cargo test --workspace --all-features` must not start a compositor.

Files to inspect/change: `justfile`, `tools/ui-testing/run-{case,capability}.sh`,
`container-runner.sh`, the runner's execute/capability entry points,
`.github/workflows/ci.yml`, and their existing shell/runner contract tests.

- Default CI retains Rust tests, lint/format, scenario contracts, workflow
  integration and Storage lab. Gate both the UI capability probe and executed
  cases, including image builds and instrumentation, before launch.
- Provide an explicit manual workflow-dispatch opt-in and local
  `UI_E2E_ENABLED=1` opt-in. Feed both into the same validated setting. Normal
  PR runs default off; do not require an administrator-set variable to pause.
- Default aggregate test commands report `rendered UI: deferred (disabled)`.
  A directly requested UI-only command while disabled should explain the
  opt-in and return nonzero, not pretend it executed successfully.
- Validate before starting Docker/Sway/GDB. Pass the explicit value into the
  owned container so direct runner/container entry points do not accidentally
  bypass the gate. This is an execution policy, not a security boundary.
- Preserve case manifests, hashes and planned/executed history. Add a separate
  deferred execution-policy status; do not rewrite old successful runs as
  failures or promote planned cases. Keep validating non-rendered named tests.
- Retain the existing UI check context where practical, with a visible
  deferred summary rather than a UI-pass claim. Inspect branch protection
  read-only before renaming/removing checks; changing required checks needs
  separate authority and must not strand PRs with a missing context.

Regressions: unset/0 launches no UI processes or UI image builds; 1 selects
the existing path; malformed values fail; all-features does not enable it;
off never skips native/scenario/business-logic tests. Test opt-in dispatch with
command fakes/spies during the pause, not a new dependency-fixing UI run.

## Phase C — Coverage with explicit execution scope

Extend the existing collectors/checker; retain the cheaper workspace-scoped
LLVM export, matching profiles/ELFs, immutable input hashes and source inventory.
Do not build a second coverage implementation.

Define two recorded modes:

- `non-rendered` (default): require successful host and native-lab execution
  and their matching evidence. Record UI as deferred and `ui_exit` as absent
  or null, never a fabricated zero or synthetic profile.
- `full-ui` (explicit opt-in): retain the existing all-eight-case execution,
  normal/instrumented proof and matching app/runner-profile requirements. This
  mode still fails while the required cases/dependency gates are incomplete.

Bind mode and resolved execution policy into evidence, reports and cache/reuse
checks. The checker must take an expected mode and reject mismatches; editing
a saved report's mode must not turn a failed full run into a valid partial run.
Separate mode-specific output directories or equally strict run identities.
An explicitly requested full run while the switch is off fails immediately.
Do not silently fall back after a UI failure or merge old UI profiles into a
fresh non-rendered run.

Adapt `tools/testing/{run_coverage,coverage}.py`, their tests, `just coverage`,
the named-test inventory/validator and reporting documentation together.
Add a CI coverage job using the same non-rendered collection path; retain
failure artifacts and its own exit status rather than just running collector
unit tests. Avoid introducing a second native-case implementation in that job.

Coverage honesty and thresholds:

1. Keep every first-party production source in the inventory, including view
   and update code. UI-dependent uncovered lines remain visible. Missing LLVM
   mappings are reported as unmeasured gaps, not silently discarded or counted
   as hits. Do not hide missing data by changing compiler features.
2. Preserve package, aggregate and changed-code 98–100% targets and existing
   exception-review rules. This plan changes required execution sources in
   non-rendered mode, not numerical thresholds or the PR comparison base.
   Continue toward the highest achievable real coverage; report residual
   rendering-dependent gaps instead of inventing exclusions to make it green.
3. Distinguish evidence completeness, numerical threshold results and deferred
   UI acceptance. A valid host+lab report may still fail its coverage gate.
   A passing non-rendered gate never means original full UI acceptance passed.
4. Keep Python/shell support-source measurement obligations. Identify dormant
   UI support separately in reports, without claiming its unexecuted paths
   were covered or silently shrinking the first-party denominator.
5. Re-measure from the checkpoint. Previously reported approximately 40% Rust
   coverage is historical, not the new baseline or a completion claim.

Required regressions include missing/failed native runs, stale profiles,
missing sources, mode mismatch, stale report-only reuse, absent UI data allowed
only in non-rendered mode, full mode missing any UI case failing, unchanged
threshold enforcement, and an unexecuted production-view gap remaining visible.

## Phase D — Audit and map production behavior before migration

Build a traceability table for each flow: actual UI message handler, production
state/validator, operation executor, completion handler, existing meaningful
tests and remaining gaps. Inspect callers, not just function/test names.

Initial families:

| Family | Production areas | Required behavior |
| --- | --- | --- |
| Physical/create/format | `src/update/volumes/*`, dialog validators, operation clients | Bounds/tools, validation, submission order, busy errors, cleanup, cancellation |
| Encryption | Volume encryption handlers and secret-bearing operation boundaries | Wrong/right secret, lock/unlock, redaction, failed/stale completion |
| Logical storage | `src/update/logical.rs`, logical state, confirmation dispatch | Preflight, reviewed identity/generation, cancel, exactly-once confirmation, stale rejection |
| Network | `src/update/network.rs`, network state and adapters | Form/schema validation, failure/retry, mount/status/unmount, stale completion |
| Image and usage | `src/update/image/*`, usage handlers, workflow operations | Progress/terminal transitions, cancellation, failures and stale-result rejection |
| Reload and selection | Update/subscription paths and volume/sidebar models | Atomic refresh, identity preservation, invalid state rejection, generation ordering |

Classify shallow tests honestly. Replace tests that only assert a literal,
parse arguments or check an inventory when their claimed obligation is actual
application behavior. Retain genuine parsing/schema tests under accurate names.
Update exact rstest-generated required names; missing/ignored/duplicate names
must still fail. Inventory validation alone is never execution coverage.

## Phase E — Share production logic, one vertical flow at a time

Start with create/format validation, then logical confirmation, encryption,
network, image/usage and reload. Preserve behavior and commit each family
independently; do not create a second application framework or rewrite all UI
state at once.

For each family:

1. Characterise the current production handler and real state with assertions,
   including failures and stale events. Resolve differences from the harness
   explicitly rather than treating the test-only reducer as the specification.
2. Prefer testing existing production functions directly. Where asynchronous
   orchestration obstructs tests, extract the smallest shared decision/reducer
   and typed effect executor. Use the existing runtime/adapters and domain
   types. Production logic must compile and be used in normal builds, not only
   under `test-backend`; only fixtures/test access remain feature-gated.
3. Wire the normal message handler to that same logic. Keep iced `Task` creation
   as a thin adapter around the shared executor and feed completions through
   the same production state transition. Keep business state authoritative in
   one place, not parallel `workflows` and UI-state copies.
4. Have tests drive those exact shared entry points with owned rstest scenario
   fixtures. Use controlled completions/virtual time for ordering; no desktop,
   task-debug-string parsing, real sleeps, global adapter swapping or storage
   mutation through a UI control socket.
5. Test the message-to-intent and completion-to-state adapters as well as the
   reducer. A test calling a helper that production never reaches is not
   sufficient. Add a deliberate local fault in the real production decision
   or routing, demonstrate that its test fails, then restore it before commit.
   Do not add a dummy fault to main or create a prototype PR for this check.
6. Delete the superseded test-only reducer/executor and duplicate validation
   after callers and behavior tests migrate. Remove dead workflow state,
   wrappers and hard-coded facade checks; preserve useful fixtures, scenario
   adapters and test intent. Do not retain two mechanisms for convenience.

Each family needs assertions for success and resulting state; rejected input
with no side effect; runtime failure and cleanup; cancellation where supported;
stale/duplicate completion; refresh/operation counts; and secret safety where
applicable. Keep unsupported production behavior explicit, not mocked success.

Gate each slice with focused tests, affected backend contracts, normal and
scenario-disabled build checks, strict Clippy and formatting. Use a headless
environment with no display/accessibility service to verify runtime isolation.
Do not claim compile-time linkage to libcosmic proves a renderer was started.

## Phase F — Close backend and coverage gaps, then hand off

1. Continue the existing native outcome ledger and safe fixture cleanup tests.
   Keep real storage assertions in the Testcontainers lab and business-policy
   assertions in shared-logic tests. Neither substitutes for the other.
2. Run the complete non-rendered suite and fresh host+lab coverage; compare
   actual uncovered production functions, not numbers of parameter rows.
   Add meaningful tests before considering any documented exception.
3. Verify default CI on the actual `4-ui-testing` PR head, including the
   default-off UI boundary, native lab and coverage. Preserve deliberately red
   collector/failure-propagation checks from the outstanding plan, scoped to
   active testing. No merge or branch-protection change is implied here.
4. Update active plan/spec status, gap/outcome ledgers and user-facing testing
   docs. Report exact tested revision, remaining gaps, measured percentages,
   thresholds and explicit rendered-UI deferral. Do not mark the original
   full Testing V2 acceptance complete merely because the active jobs pass.

## Deferred work and re-enablement checklist

Preserve the rendered runner, cases, evidence and dependency-fix ledger. Do not
expand their feature work, fix additional widget protocols, implement rendered
secret transport, update goldens or refresh the Wayland quarantine while paused.

Re-enable only after an explicit review of compatible upstream releases/fixes
against our pin; a reviewed retain/remove decision for each fork patch; expiry
review of any quarantine (expired means failure); capability plus real case
validation; instrumented provenance; and human visual/accessibility review.
`UI_E2E_ENABLED=1` permits running the suite, not bypassing those checks.

## Completion checklist for this regrouping

- [x] Checkpoint preserved; dependency work frozen and proposals marked paused.
- [ ] Default local/CI execution launches no rendered-UI infrastructure.
- [ ] Explicit opt-in and disabled/error paths are regression-tested.
- [ ] Non-rendered coverage has validated provenance and honest unchanged scope.
- [ ] Traceability identifies production callers and meaningful behavior tests.
- [ ] Every migrated workflow is shared by production and tests; duplicates removed.
- [ ] Backend/native and application-logic gaps are tested at the correct layer.
- [ ] Fresh coverage, build contracts and final-head CI evidence are recorded.
- [ ] Unmet numerical gates remain failures; rendered/visual acceptance remains deferred.

Next action: finish the family-by-family production-logic migrations and final
coverage/CI verification. See `production-logic-traceability.md` for actual
callers, slice evidence and remaining state-ordering gaps.

### Execution checkpoint — 2026-09-16

Phase A checkpoint `eb359d5` preserves the previous app/fork integration and
incomplete rendered-test work, with its recorded verification limits. No live
task-owned builds or UI runs remained. Both dependency worktrees are clean at
libcosmic `2ca5a417` / iced `d38647d7a`; pins and quarantine are frozen.
PR #117 still targets `main`. Read-only GitHub inspection reports no classic
branch protection on `main`; no repository setting was changed.

Phase B/C implementation is under validation: shell/recipe/container/runner
launch guards, manual-only CI opt-in and explicit coverage modes are in place.
92 runner unit tests plus three real runner-process guard tests pass. Python
contracts: 39 pass, two explicitly container-only checks skip. Strict runner
Clippy and formatting pass. The UI-only invocations fail before file access or
process launch when disabled. No rendered run has been started.
Baseline at `95b4f66`: host tests and all 17 native outer cases passed (one
unselected bridge test). Collection returned 1 because numerical gates remain
unmet, not because UI was required. `ui_status=deferred`; no UI profiles were
used. Evidence: `target/coverage/non-rendered/run-m59zg3yg`, `evidence.json`,
`acceptance.json`, `summary.json`, `lcov.info` and HTML in that mode directory.
Comparison base was origin/main `0ba27cc2caac19acb7a8d98747f8b65058dab877`.

Measured Rust baseline: workspace lines 9,352/29,510 (31.69%), functions
1,131/3,490 (32.41%); app lines 2,110/16,944 (12.45%), functions 230/1,678
(13.71%). There are also 49 inventory sources without LLVM mappings, explicitly
reported for review (many declaration/module-only files). These percentages
describe mapped Rust code, not support-script coverage or full acceptance.
The 98–100% and changed-code gates remain unchanged and failing. Final-head
measurement must be regenerated after source changes.

Phase D caller audit is recorded in `production-logic-traceability.md`. The
create/format slice is being migrated to production handlers and has exposed
actual validation/adapter gaps; remaining families and final CI are outstanding.
This is not completion of the plan.

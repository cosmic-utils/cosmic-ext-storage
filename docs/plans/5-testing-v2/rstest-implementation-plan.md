# Implementation plan: rstest adoption

Status: implementation, local gates and hosted CI acceptance complete.
See `rstest-execution-record.md` for the exact tested revision and CI evidence.

Date: 2026-09-14. Working branch: `codex/rstest-adoption`, based on
`4-ui-testing` at `12fb884e2d462b91cacb1b097e8fa35826f84345`.

Implements [rstest-spec.md](rstest-spec.md). This is a bounded test-composition
migration, not completion of the separate [Testing V2 plan](implementation-plan.md).
Execute phases in order. Each phase's exit gate must be satisfied before the
next migration phase. Completed checkboxes are backed by [the execution record](rstest-execution-record.md).

## 1. Invariants and scope

- Use exactly `rstest = { version = "=0.27.0", default-features = false }`
  centrally, with dev-dependencies only in packages actually using it.
- Preserve the pinned toolchain, GUI dependencies, image contract, Testcontainers,
  Nextest supervision, ownership checks, explicit cleanup and evidence pipeline.
- Keep stateful lifecycle assertions together. Use explicit named cases, not
  Cartesian products for expensive lab operations. Never inject two independent
  runtimes when a client and runtime must belong to the same world.
- No resource-owning `#[once]`, rstest timeout substitution, secret-bearing
  traces/labels, new fixture registry, `rstest_reuse`, or production API redesign.
- No VMs, Python runner rewrite, published toolkit, libcosmic update or PR.
- Preserve existing unrelated work, including `docs/future.md`. Record the actual
  starting revision and dirty state; do not silently reset or switch the base.
- This document does not itself authorize implementation, publishing, or changes
  to repository branch-protection policy.

## 2. Bounded migration inventory

Paths below are repository-relative. Only convert independent assertions and
duplicated setup in these files. Anything beyond this list needs a recorded
scope decision; "similar tests" is not an open-ended migration instruction.

| Files | Planned disposition |
| --- | --- |
| `crates/ui-e2e-runner/tests/unit/shutdown_tests.rs` | Replace nine numbered diagnostic mutations, six scope/supervisor/expiry rows, and functional/shutdown status combinations with named cases and fresh diagnostics. Preserve clean-exit mismatch and malformed-diagnostic assertions. |
| `crates/ui-e2e-runner/tests/unit/cases_tests.rs` | Parameterize independent case-schema mutations, keyboard inputs and fixture-path outcomes. Split independent selector/property checks only with fresh equivalent trees. Keep ancestor relationships, state transitions and child-process timeout/kill checks explicit. |
| `tests/unit/utils/unit_size_input_tests.rs` | Named conversion, tolerance, index round-trip and auto-selection tables. Preserve existing numbers, all five units and labels/order assertions; retain straightforward one-off tests. No other pure-value test file is required in this pass. |
| `crates/test-backend/tests/schema.rs`, `physical_state.rs`, `logical_network_state.rs`, `workflow_state.rs`, `contract_surface.rs` | Shared scenario-path/runtime fixtures; replace repeated construction, not backend behavior. Preserve ordered subscriptions, cancellation, operation-inventory and secret-redaction checks. Use an owned temporary directory for schema overlay I/O. |
| `crates/test-backend/tests/unit/control_tests.rs` | Reuse test-only scenario composition while retaining private-module access. Replace PID socket paths with an owned short temporary root. Preserve authentication, serialized requests, checkpoint feature behavior and explicit server shutdown. |
| `tests/ui_scenario_contract.rs`, `tests/scenario_control_runtime.rs` | Root-package scenario fixtures; replace repeated path/runtime setup and PID socket/token paths. Preserve feature gates, no-production-fallback assertions and runtime/control ownership. |
| `tests/application_workflows.rs` | Inject fresh owned `WorkflowHarness` instances with fixture/secret overrides. Tests deliberately comparing two contexts still construct two explicitly. Keep reducer/effect scheduling, cancellation, stale completions and success/failure sequences. |
| `crates/storage-lab-tests/tests/bridge.rs` | Replace the 15 straightforward forwarding wrappers with 15 named `(target, exact_filter)` cases. Retain the dedicated deliberate-failure outer test and host-safe cleanup/exit-status regression. Confirm these starting counts by discovery. |
| `crates/storage-lab-tests/tests/common/mod.rs`, `partitions.rs`, `filesystems.rs`, `images.rs`, `encryption.rs`, `btrfs.rs`, `logical.rs` | Add one owned async lab fixture composed over `lab`; migrate existing callers using their current labels and sizes. Keep whole lifecycles, additional-device ownership, explicit cleanup and `owned` checks. |

Retain without composition rewrites: `tests/storage_lab.rs` (the inner application
target), native `capability.rs` and `network.rs`, lab unit safety tests, UI
coverage/main tests, and other root/storage-crate tests. They remain validation
inputs. Touch their selectors or metadata only if a migrated identity requires it.

Permitted supporting files: the four participating package manifests and root
workspace declaration, `Cargo.lock`, test-only common modules, maintained
composition regressions, exact inventories/evidence consumers, quarantine lock
binding, and documentation. CI/recipes change only where needed to preserve
selection or run this plan's regression gates; do not create a second runner.

## 3. Phase 0 — freeze baseline and assertion inventory

This is a local migration baseline, not a repeat VM/capability prototype.

- [x] Record revision, branch, toolchain/Nextest versions, dependency graph,
  environment/image digests and existing worktree changes.
- [x] Create `rstest-execution-record.md` beside this plan. Store full logs and
  machine-readable listings in a dedicated ignored artifact directory; link them
  from the record with revision, command, exit status and digest. Do not commit
  raw LLVM reports, tokens or machine-specific scratch files.
- [x] Capture Cargo and Nextest discovery for the workspace/all-features build,
  default root build, explicit scenario-feature targets, and outer bridge.
  Record ignored status as well as package, target and exact test name.
- [x] Create `rstest-test-mapping.md` beside this plan. For each changed old test,
  enumerate its independent inputs/assertions, retain/convert decision, expected
  behavior and eventual discovered new identities. Include every negative row;
  one old function mapping to several cases is expected.
- [x] Run the host/feature matrix in section 8, the native bridge, and available
  executed UI cases before edits. Record exact selected native cases, per-case
  container launches and durations. Separate image build time from execution.
- [x] Collect fresh baseline coverage with `just coverage`; preserve the scoped
  report, acceptance failures and provenance before a later run replaces pointers.
  Record per-source covered lines/functions and totals, not just percentages.

The last recorded result was 39.37% lines / 38.51% functions, with seven of eight
mandatory executed UI cases missing. That is historical context, not this phase's
baseline or a passing threshold. Inventory current failures anew.

In particular, `tests/unit/models/helpers_tests.rs` currently has a D-Bus helper
whose failure returns early from tests. Record those bodies as potentially
unexecuted, not proven behavior coverage; leave their redesign to Testing V2.
Do not copy that implicit-skip pattern into new fixtures.

Exit gate: the old assertion inventory and baseline evidence are complete.
Classify existing acceptance failures separately. If infrastructure prevents
valid native/UI profiles, resolve or report that blocker before claiming a
coverage-preserving migration; an old report is not a replacement.

## 4. Phase 1 — dependencies and maintained composition regressions

- [x] Add the workspace rstest declaration and dev dependencies to the root app,
  `test-backend`, and `ui-e2e-runner` as their tests begin using it. Add it to
  `storage-lab-tests` in Phase 4. Reuse existing `tempfile` versions where needed;
  keep default-feature compilation free of optional scenario imports.
- [x] Resolve the lockfile without a broad dependency update. Review every changed
  package and compare normal/build dependency graphs before/after, including
  feature-enabled builds. Reject unrelated changes or production rstest edges.
- [x] Inspect pinned rstest source/API and retain the project's Tokio runtime
  flavor. Do not rely on a later upstream version or the temporary prototype.
- [x] Add maintained `crates/test-backend/tests/fixture_composition.rs` regressions
  and test-only support under `crates/test-backend/tests/common/`. Use existing
  scenario/resource objects rather than introducing a generic harness.

Required regression matrix:

| Regression | Required proof |
| --- | --- |
| Named cases | Every row is separately listed and exactly selectable by Cargo and Nextest; the selected body actually executes. |
| Async overrides | At least two scenario choices are loaded through awaited fixtures on Tokio current-thread, with distinguishable state. |
| Coherent ownership | A client observes changes to its own injected runtime; two independent contexts do not share mutations. |
| Fixture factories | A regression demonstrates that separately requested dependency fixtures are not memoized; actual tests avoid accidental double construction. |
| Normal cleanup | An owned temporary root is removed when its context is released. |
| Partial setup failure | Acquire a temporary resource, then deliberately fail setup in a child test. Parent verifies nonzero failure, body-not-entered evidence, and released resource. |
| Panic cleanup | Deliberately panic after successful fixture setup in a child; parent verifies failure and cleanup after process exit. |

Use child invocations of the built test executable, not recursive Cargo calls.
Deliberate-failure helpers are explicitly ignored and selected only by their
parent using verified exact names. They must not fail normal runs, silently skip
the parent's checks, depend on test order, or require privileged host access.
Exercise the parents under both runners. Store only non-secret diagnostic markers.

Exit gate: maintained host-safe regressions pass under Cargo/Nextest and strict
Clippy; graph/lock review is recorded. Until Phase 5 validates the changed lock
binding, the existing UI quarantine is not assumed valid for the new build.

## 5. Phase 2 — pure tables, scenario and workflow fixtures

- [x] Migrate shutdown and UI input cases from section 2. Prefer descriptive
  mutation functions/explicit values over recreating a numeric dispatch table.
  Construct fresh state for each independent case; preserve all old assertions.
- [x] Migrate size conversions and round-trips, preserving floating-point
  tolerances and unit ordering. Additional boundary cases are welcome when tied
  to the same contract, but must be recorded separately from migrated assertions.
- [x] Add package-local common fixtures for scenario path/runtime composition.
  For the root tests, use `tests/common/`; for backend integration tests, use
  their package's `tests/common/`. Share backend helpers with included unit tests
  using explicit test-only module wiring where appropriate. No production
  dependency on rstest or new cross-workspace support crate is necessary.
- [x] Replace the overlay directory and control socket/token roots with fresh
  owned `TempDir`s. Use short Unix socket paths. Keep runtime/server ownership
  explicit and release/shut down those resources before directory deletion;
  do not pretend that directory removal proves asynchronous shutdown completed.
- [x] Inject owned workflow harnesses with explicit scenario/secret overrides.
  Keep secret construction out of case labels and Debug/trace output. Preserve
  existing workflow/redaction/isolation tests and deliberate dual-context tests.
- [x] Update discovered identities and all exact inventory references in the
  same migration change. Populate the old-assertion-to-new-case mapping.
- [x] Delete replaced path builders, numeric loops and manual temporary cleanup
  immediately. Retain semantic helper methods and operational source code.

Exit gate: affected packages and the feature matrix pass; assertion mapping is
complete for host changes. Inspect discovery and captured output for secret
exposure using a synthetic test secret, never a real credential. Compare host
test counts with the mapping; investigate omissions and unexpected products.

## 6. Phase 3 — exact selection and evidence compatibility

- [x] Audit `tests/ui/required-tests.toml`, `tests/ui/traceability.toml`,
  `tools/ui-testing/assert_tests.py`, `tools/testing/coverage.py`,
  `tools/testing/run_coverage.py`, `tools/testing/test_lab_contract.py`, recipes
  and CI for renamed tests. Search for every old name, not just these paths.
- [x] Keep the required-test validator's exact-match/count checks. Populate
  identities from actual discovery; never infer numeric rstest suffixes or
  replace the required inventory with unchecked automatic discovery.
- [x] Add validator regression coverage under `tools/testing/` as needed:
  a valid generated identity succeeds, a missing/stale one fails, and ambiguous
  selection cannot pass. Exercise the actual validator using controlled test
  inputs without leaving a modified required manifest in the worktree.
- [x] Verify coverage evidence recognizes generated Cargo/Nextest identities
  and still rejects missing required sources. Extend existing reporting tests
  where names are parsed; preserve first-party scoping and zero-count functions.
- [x] Keep all eight mandatory UI case IDs/proof obligations unchanged. Scenario
  files and generated Rust unit cases are not substitute executed UI evidence.

These checks apply during Phase 2 too; this phase is the cross-cutting audit,
not permission to leave earlier commits with stale required inventories.

Exit gate: exact-selection regressions and all Python tests pass. A raw Cargo
command returning success for zero matches is not sufficient evidence; the
maintained wrapper/validator must reject it.

## 7. Phase 4 — native composition and safety revalidation

- [x] Add the lab dev dependency. Convert only the common-lab callers listed
  in section 2 to the owned awaited fixture, preserving label/size overrides,
  error propagation, additional resources, ancestry checks and cleanup order.
- [x] Rebuild the container and discover the actual inner names inside it.
  Update the forwarding table's exact filters atomically with these changes.
  Preserve `#[ignore]` and the explicit `STORAGE_LAB=1` entry-point opt-in.
- [x] Convert the 15 forwarding wrappers into 15 named target/filter cases.
  Preserve the capability target for the six existing capability forwarders;
  preserve the partitions, filesystems, images, encryption, btrfs, network,
  logical (two) and application target mappings for the remaining nine.
  Keep the deliberate-inner-failure outer test separate: normally 16 selected
  native outer tests total, subject to verified baseline discovery.
- [x] Run `STORAGE_LAB=1 just test-lab`. Confirm every expected outer case and
  its exact inner body ran once; verify artifacts, cleanup and failure propagation.
  Retain five-minute hard limits, zero retries and single-slot scheduling.
- [x] Through the isolated outer bridge, send a deliberately missing inner
  filter. Require the existing inner guard's failure (currently exit 64), not
  a zero-test green result, and preserve diagnostics. Add/retain a maintained
  regression and account separately for its additional container launch.
- [x] Re-run native normal cleanup, drop cleanup, failure-after-acquisition and
  deliberate inner assertion failure. Keep the host-safe cleanup-error/unknown
  status regression. Never execute ignored native bodies directly on the host.
- [x] Compare native selection, launches and per-case execution times to Phase 0.
  Migration-only forwarding cases must not increase container launches. Report
  separately any launches from newly added safety regressions. Investigate
  timing regressions under comparable cache/image conditions; do not claim a
  speedup from one noisy sample or reduce safety checks for performance.

Exit gate: all native cases/safety regressions pass with checked resource
cleanup and complete evidence. New regression case counts are explicitly
reconciled, not hidden by weakening the baseline expectation.

## 8. Phase 5 — UI lock revalidation and final acceptance

### 8.1 Quarantine binding

- [x] Review the final lockfile diff and GUI/runtime dependency closure, including
  feature unification. Preserve libcosmic/iced revisions and the diagnosed closure.
- [x] Under spec section 5.3, rebind only the exact reviewed lock hash when the
  change is dev-only, then run the executed, instrumented reload path and inspect
  evidence. Treat the new binding as provisional until validation passes.
  Preserve signature, case/environment hashes, owner and expiry. Do not extend
  expiry, broaden matching or treat an unrecognized crash as acceptable.
- [x] Record functional steps and shutdown outcome separately, with real nonempty
  app/runner profiles and matching ELF hashes. A clean shutdown is acceptable;
  a recognized crash needs the exact authorized evidence. Stop for review if the
  closure changed, policy expired, or classification is different.

### 8.2 Required command matrix

Run from the repository root under its pinned toolchain. Listing steps use the
same package/features as their execution; preserve full outputs and exit codes.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features --all-targets --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo nextest run --workspace --all-features --locked
cargo test -p cosmic-ext-storage --locked
cargo test -p cosmic-ext-storage --locked --test scenario_feature_disabled_contract
cargo test -p cosmic-ext-storage --features test-backend --locked --test ui_scenario_contract
cargo test -p cosmic-ext-storage --features test-backend --locked --test scenario_control_runtime
cargo test -p cosmic-ext-storage --features test-backend --locked --test application_workflows
cargo test -p test-backend --locked
cargo check -p cosmic-ext-storage --no-default-features --all-targets --locked
cargo check -p cosmic-ext-storage --no-default-features --features test-backend --all-targets --locked
cargo tree --workspace --edges normal,build --locked
cargo tree --workspace --all-features --edges normal,build --locked
python3 -m unittest discover -s tools/testing -p 'test_*.py'
python3 tools/ui-testing/assert_tests.py --phase all
STORAGE_LAB=1 just test-lab
just ui-e2e-case tests/ui/cases/live_scenario_reload.toml
just coverage
```

Default-feature and no-default-feature builds are distinct checks. If a baseline
configuration is already broken, record it explicitly; do not silently remove
the row or attribute its success to an all-features run.

Run all other implemented executed UI cases available then, through the existing
container path. `just coverage` supplies the instrumented path; the ordinary
reload command alone does not prove checkpoint/profile durability. No capability
report or approved-looking screenshot substitutes for executed/approved cases.

### 8.3 Coverage, cleanup and hosted evidence

- [x] Collect fresh final coverage after every test/fixture/lock/policy change.
  Compare per-file covered line/function sets with Phase 0, mapping source shifts
  if necessary. Explain any instrumentation differences and re-run nondeterministic
  paths; restore actual lost behavior. Never offset a lost path with a higher
  aggregate percentage, moved source, exclusions or reduced thresholds.
- [x] Keep LLVM report scoping and the existing 98–100% thresholds unchanged.
  Classify final failures as unchanged Testing V2 gaps or migration regressions.
  An unexplained new acceptance failure blocks this adoption's completion.
- [x] Audit removal of superseded helpers/entry points and active stale names;
  inspect the diff for moved lifecycle code, broad dependency changes, fixture
  duplication, new implicit skips, secrets and generated build artifacts.
- [x] Once execution/publication is authorized, validate this branch through
  existing app-repository CI and record the exact tested revision/run URLs.
  Local success does not imply hosted success; do not open a libcosmic PR or
  change branch protection. If publishing authority is absent, report hosted
  validation as pending rather than complete.
- [x] Finish `rstest-test-mapping.md` and `rstest-execution-record.md` with
  commands, case/launch counts, cleanup/failure evidence, coverage comparisons,
  CI links and remaining Testing V2 obligations. Cross-link from the existing
  execution record without rewriting historical evidence.

Local and hosted-evidence exit gates satisfied. CI run `34879591862` passed
all seven jobs at revision `1539e8c0a571ec143c89f9a00f41ce2f26967890`.
Any later commit still requires its own green checks before merge.

Exit gate: every migrated assertion is accounted for; own regression/feature/
native/UI validation passes; exact identities and reporting agree; cleanup is
complete; no unreviewed coverage loss or added production dependency exists;
hosted evidence is recorded. Report rstest adoption and overall Testing V2 status
separately. Seven missing UI cases, coverage thresholds, visual approval and
other pre-existing gates are not waived by completing this plan.

## 9. Suggested implementation changesets

1. Baseline/mapping record, dev dependency and maintained fixture regressions.
2. Pure UI/size cases, including exact identity updates.
3. Scenario/workflow fixtures and temporary-resource cleanup, including selectors.
4. Cross-cutting validator/reporting regressions.
5. Native fixture/forwarder migration and safety/selection regressions.
6. Reviewed quarantine binding, fresh coverage/CI evidence and final cleanup audit.

Keep each changeset's applicable host/selection gates passing. A phase failure
means fix or report the cause, not proceed with a weaker gate. If a particular
conversion offers no clarity or requires broad API changes, retain the plain
test with a documented mapping rationale and resolve any required-scope conflict
before declaring completion.

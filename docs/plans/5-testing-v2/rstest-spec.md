# Specification: adopt rstest for test composition

Status: implementation, local acceptance and hosted CI acceptance complete.
See `rstest-execution-record.md` for the exact tested revision and CI evidence.

Execution sequence and validation gates:
[rstest implementation plan](rstest-implementation-plan.md).

Date: 2026-09-14. Branch: `codex/rstest-adoption`, created from `4-ui-testing`
at `12fb884e2d462b91cacb1b097e8fa35826f84345`.

## 1. Purpose and relationship to Testing V2

Adopt the established Rust `rstest` crate for reusable fixture injection and
parameterized tests. Replace duplicated generic test composition with the
crate's supported mechanisms, while retaining the specialized capabilities
that it does not provide.

This is a scoped addition to the [Testing V2 specification](spec.md) and
[implementation plan](implementation-plan.md), not a replacement for their
unfinished coverage and executed-UI gates. More generated test names do not
constitute additional behavior coverage by themselves.

The future community toolkit should expose resource and driver APIs that work
with `rstest`, not invent its own fixture-injection or case-generation framework.
Publication and extraction remain [future work](../../future.md).

## 2. Investigation evidence

The investigation evaluated `rstest` 0.27.0 with default features disabled.
Its declared minimum Rust version is 1.85; the prototype also passed on this
project's pinned Rust 1.95. A standalone scratch crate using the actual
`storage-types`, `storage-contracts`, and `test-backend` packages passed:

- 11 tests under both Cargo and Nextest;
- strict all-targets Clippy;
- named parameter cases and individually discoverable test identities;
- Tokio current-thread async fixtures, including per-case fixture overrides;
- independent mutable scenario state for separate cases;
- normal temporary-directory cleanup and cleanup during assertion unwinding;
- propagation of a deliberate child-test failure as exit code 101;
- a regression demonstrating that ordinary dependency fixtures are not
  automatically cached once per test.

The deliberately failing helper was excluded from normal selection and
explicitly executed by its parent regression, which checked both failure and
cleanup. This was not a privileged native-lab or UI acceptance run.

Scratch evidence was retained at
`/tmp/cosmic-rstest-investigation-hyam50`, with pinned-toolchain logs at
`/tmp/rstest-investigation-pinned-{test,nextest,clippy}.log`. These temporary
files are background evidence, not a dependency of implementation or CI.
Promote the applicable regressions into maintained repository tests.

Upstream references used in the investigation:

- [rstest project and supported composition features](https://github.com/la10736/rstest)
- [Fixture API and once-fixture limitations](https://docs.rs/rstest/latest/rstest/attr.fixture.html)
- [Parameterized tests, async support and tracing](https://docs.rs/rstest/latest/rstest/attr.rstest.html)
- [Timeout implementation](https://docs.rs/rstest/latest/src/rstest/timeout.rs.html)
- [Nextest's process-per-test execution model](https://nexte.st/docs/design/how-it-works/)

The documentation links track upstream latest; implementation must check the
pinned 0.27.0 source rather than silently adopting later APIs or versions.

## 3. Dependency policy

Declare the version centrally:

```toml
[workspace.dependencies]
rstest = { version = "=0.27.0", default-features = false }
```

Only packages that use it receive:

```toml
[dev-dependencies]
rstest.workspace = true
```

- Keep `rstest` out of normal production dependencies. Do not add it to every
  workspace member speculatively.
- Update and review `Cargo.lock`; keep the Rust toolchain, libcosmic/iced pins,
  and unrelated dependencies unchanged.
- Retain `#[tokio::test(flavor = "current_thread")]` where presently required,
  composed beneath `#[rstest]`. Use `#[future(awt)]` for awaited fixtures.
  Disabling rstest's default features does not remove Tokio fixture support.
- Implementation learning on 0.27.0: composing `#[from(...)]`, `#[future(awt)]`
  and a mutable renamed argument failed fixture resolution in the workflow
  tests. Inject the owned argument without `mut`, then use `let mut harness =
  harness;` in the body. The maintained workflow suite checks this composition.
- Give mutation-closure parameters explicit types in named case attributes;
  the shutdown matrix needed those annotations for macro-expanded inference.
- Do not add `rstest_reuse` initially. Consider it only after identifying a
  concrete case table genuinely shared by multiple tests, with a separate
  dependency justification. Do not create a home-grown equivalent meanwhile.
- Reuse the existing `tempfile` dependency for ordinary temporary directories.

## 4. Replacement scope

| Area | Required change | What remains explicit |
| --- | --- | --- |
| `crates/ui-e2e-runner/tests/unit/shutdown_tests.rs` | Replace numbered mutation loops and repeated policy-status combinations with descriptive named cases and a fresh diagnostic fixture. | Exact stack classification, scope/expiry checks, and expected outcomes. |
| `crates/ui-e2e-runner/tests/unit/cases_tests.rs` | Parameterize independent valid/invalid selectors, keys and property checks where each row expresses one contract. | Stateful action sequences, tree relationships, and semantic assertions. |
| `tests/unit/utils/unit_size_input_tests.rs` and similar pure value tests | Consolidate repetitive conversions and boundary/round-trip checks into named input/expected-output tables. | Every distinct assertion and boundary value; preserve one-off tests that are clearer as plain Rust. |
| `crates/test-backend/tests/{schema,physical_state,logical_network_state,workflow_state,contract_surface}.rs`, related control tests, and root scenario tests | Consolidate repeated fixture-path and scenario setup using shared fixture functions with explicit overrides. | Fixture integrity, schema validation, backend isolation and behavior assertions. |
| `tests/application_workflows.rs` | Inject fresh, owned workflow contexts instead of repeating `WorkflowHarness::from_fixture(...)` construction. | Reducer/effect sequencing, stale-completion tests, secret handling, and trace assertions. |
| `crates/test-backend/tests/schema.rs` | Replace the PID-derived temporary directory and manual deletion in the overlay test with an owned `TempDir` fixture. | Assertions that the immutable fixture is unchanged and overlay replacement is correct. |
| `crates/storage-lab-tests/tests/bridge.rs` | In the later native migration, consolidate straightforward outer wrapper tests into explicit named target/filter cases. | `run_inner_case`, exact inner selection, artifact verification, and dedicated failure/cleanup regressions. |
| `crates/storage-lab-tests/tests/common/mod.rs` and native integration tests | Wrap repeated lab construction in per-test fixtures only where this improves setup composition. | `lab`, `owned`, `LabFixture`, readiness checks, storage operations and checked cleanup. |

Before changing each area, capture the current test inventory and enumerate
the assertions/input rows to preserve. Do not convert loops whose iterations
deliberately share state into independent cases, or split a storage lifecycle
into tests that depend on execution order.

Use named `#[case::description(...)]` rows for audited behavior. Use `#[values]`
only when its generated combinations are intentional and bounded. A Cartesian
product must not accidentally multiply expensive container runs.

## 5. Ownership and fixture design

### 5.1 One coherent context per test

An injected fixture should return an owned context or existing RAII object.
Prefer one `ScenarioContext`, `WorkflowHarness`, or lab context that owns its
runtime, temporary root, secrets, resources and guards as appropriate.
Derive clients from that same context inside the test or through its methods.

Do not independently inject a runtime and a client fixture that constructs
another runtime through a dependency. Ordinary rstest fixtures are factories,
not automatically memoized per-test singletons. Verify coherent identity and
independent state across different tests with explicit regressions.

Fixture functions must do real setup or compose existing constructors. Do not
introduce another registration system, scheduler, pass/fail protocol, or layers
of one-line wrappers solely to make every helper an rstest fixture.

### 5.2 Cleanup and failures

- No `#[once]` fixtures for mutable scenarios, temporary roots, containers,
  loop devices, processes or guards: once-fixture values are never dropped.
  Do not use them as a suite-wide cache under Nextest either.
- Preserve Rust ownership-based cleanup on ordinary return and panic unwind.
  This does not guarantee cleanup after abort, SIGKILL, or process termination;
  retain the existing outer isolation and leak checks.
- Keep explicit fallible cleanup where success is an acceptance condition.
  `Drop` is a fallback, not proof that unmount/detach succeeded. A cleanup
  error must continue to fail the test and preserve diagnostic evidence.
- Async fixture setup is not automatic async teardown. Preserve explicit
  shutdown/awaited cleanup and the existing outer supervision where needed.
- Setup failure must fail its selected test, never return early as a passing
  test or create an implicit skip. Previously acquired resources must unwind
  through their existing owners.
- Do not replace the native lab's ownership-checked backing-file root with a
  generic `TempDir` that deletes files while resources remain attached.
  The simple overlay-test directory is a separate, safe use of `tempfile`.

### 5.3 Supervision and confidentiality

- Keep Nextest's single-slot storage-lab group, zero retries, five-minute hard
  limit, and existing container/process supervision unchanged.
- Do not use rstest's `#[timeout]` as a replacement for those controls. Its
  synchronous timeout panics in the waiting thread without stopping the
  worker thread; external mutations could otherwise continue.
- Do not enable `#[trace]` for secret-bearing fixtures. Passphrase/token values
  must not enter case names, parameter labels, debug output or new artifacts.
  Use descriptive case labels and the existing out-of-band secret mechanism.
- Preserve the exact, expiring shutdown quarantine. Never automatically refresh
  policy hashes, broaden matching, or extend its expiry to make a run pass.
- Adding rstest changes `Cargo.lock`, invalidating the quarantine's exact lock
  binding even if libcosmic/iced themselves are unchanged. Explicitly review
  that diff and the GUI/runtime dependency graph, then revalidate the executed,
  instrumented reload case. A reviewed dev-only lock change may be rebound to
  its new exact hash with recorded evidence; preserve the case, stack signature,
  environment, owner and expiry. If the diagnosed dependency closure changes,
  stop for review rather than treating it as the same known failure.

## 6. Retained infrastructure and non-goals

Retain these mechanisms; rstest supplies test composition, not their behavior:

- Cargo/libtest and Nextest execution, plus Testcontainers lab lifecycle;
- ledgered storage resources, ancestry checks, private services and leak checks;
- scenario schema/model, virtual clock, authenticated control and workflow scheduler;
- Rust UI runner, AT-SPI driver, compositor lifecycle, screenshots and artifacts;
- Python coverage orchestration/checker, planning validation, and optional GDB helper;
- pinned container environments shared by local development and GitHub CI.

This change must not introduce VMs, update libcosmic/iced, publish a crate,
open a libcosmic PR, or port the Python reporting tools. It must not restore
the retired `tools/storage-testing` harness. Community extraction is not part
of this adoption pass.

## 7. Test identities, selection and evidence

Parameterized tests generate names such as
`module::test_name::case_1_description`; case ordering can affect the numeric
part. The old parent function name is not an individually runnable case.
With larger tables the index can be zero-padded (the implemented 15-case
bridge uses `case_01_...`); discovery, not a naming template, is authoritative.

For every renamed or expanded test:

1. Capture actual before/after discovery with `cargo test -- --list` and
   Nextest listing, using the package's required features.
2. Record an old-test/assertion-to-new-case mapping. Preserve every required
   behavior, including negative cases, even when the test count changes.
3. Update applicable `tests/ui/required-tests.toml`, `tests/ui/traceability.toml`,
   exact lab selectors, evidence consumers and documentation in the same change.
4. Select mandatory cases by their discovered identities. Do not weaken
   `tools/ui-testing/assert_tests.py` to accept an arbitrary prefix match or
   relax `tools/storage-lab/run-tests.sh`'s exact-selection/zero-match guard.
5. Prove an intentionally stale/missing selector fails instead of yielding a
   green run with zero executed tests; preserve ignore/opt-in behavior.

Never replace required-case inventory with an unchecked filesystem glob, or
count scenario data files as executed UI tests. The eight mandatory UI case
identities and their semantic proof obligations remain unchanged.

## 8. Cleanup requirements

Delete superseded setup helpers, duplicated path builders, temporary-directory
management, numeric mutation dispatch and wrapper functions in the same
changes that introduce their replacements. Do not leave old and new test
entry points running the same cases during an indefinite transition.

- Keep test-only fixture composition under the relevant `tests/common` or
  `tests/unit` tree, preserving private-module access where necessary.
- Keep operational support code in its existing first-party source boundary.
  Moving lifecycle or ownership logic into excluded test paths to improve
  measured coverage is forbidden.
- Remove unused imports and any genuinely unused dev dependencies after
  checking all features/targets. Retained helpers need a documented purpose,
  not a redundant rstest wrapper around every function.
- Review active recipes, CI, test lists and comments for stale names and
  duplicate routes. Historical execution records stay historical; do not
  rewrite prior test evidence to imply the new cases ran then.
- Do not copy the temporary investigation crate, its lockfile, build output
  or local absolute paths into the maintained test suite.

## 9. Sequencing and acceptance

### Stage A: pure cases and host-safe fixtures

Add the dev dependency, migrate the pure matrices and scenario/workflow setup,
and replace the unsafe-to-reuse PID directory pattern. Preserve or add
maintained regressions for named discovery, Tokio fixture overrides, coherent
context identity, independent state, setup failure, and unwind cleanup.

### Stage B: native test composition

Only after Stage A passes, migrate eligible lab setup and outer wrapper tests.
Retain all native families and dedicated deliberate-failure/cleanup cases.
Re-run them through the existing isolated Testcontainers path, never directly
against the host. Verify single-slot scheduling and selected case identities.

### Stage C: audit, fresh coverage and hosted validation

Required checks include:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features --all-targets --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo nextest run --workspace --all-features --locked
python3 -m unittest discover -s tools/testing -p 'test_*.py'
python3 tools/ui-testing/assert_tests.py --phase all
STORAGE_LAB=1 just test-lab
just ui-e2e-case tests/ui/cases/live_scenario_reload.toml
just coverage
```

Also run all other executed UI cases available at implementation time; a
capability-only run does not substitute for them. Check the production
dependency graph to confirm rstest is dev-only. Validate the resulting tests
on this branch's pinned Rust toolchain and through the existing hosted CI.

Acceptance must establish all of the following:

- No old assertion, required case, safety control or cleanup failure is lost.
- Named generated cases are separately discoverable/runnable in Cargo and
  Nextest; stale filters fail; deliberate inner failure still fails the outer
  test and preserves artifacts.
- Resource setup/teardown, serial native scheduling and privacy requirements
  are unchanged. No new production dependency or unrelated dependency upgrade
  has been introduced.
- Superseded infrastructure is removed and all active identities agree.
- Coverage is freshly collected after test/Cargo/fixture changes. Do not reuse
  old raw-profile provenance or call increased test counts a coverage gain.
  Inspect any lost covered lines/functions and restore lost behavior; do not
  compensate by excluding source or changing thresholds.
- The first-party report scoping/refinement and all 98–100% targets remain
  unchanged. Existing incomplete Testing V2 gates may still make `just coverage`
  nonzero: record exact outstanding failures and distinguish them from adoption
  regressions. This spec does not authorize a green waiver or claim that
  overall Testing V2 is complete.

Record commands, discovered-case mapping, cleanup/failure evidence, report and
CI links, and any remaining gates in the execution record. The adoption is
complete only when its own migration and validation obligations are satisfied;
the separate Testing V2 acceptance status must remain explicit.

# Deterministic implementation plan

> **Revision status:** The Phase-8/9 code currently present is a metadata
> scaffold, not an executed accessibility or visual test system. Before those
> phases can be recorded as complete, implement the mandatory replacement in
> [the E2E revision plan](e2e-revision-plan.md). Its Rust runner, v2 fixtures,
> real AT-SPI actions, PNG/tree goldens, and artifact gates refine the Phase-8
> and Phase-9 requirements below; the safety and composition phases remain in
> force unchanged.

## Execution order and phase protocol

Complete phases strictly in numerical order. A phase is not complete because its
code compiles or a broad workspace command is green: it completes only when
its named gate, scoped manual review, and artifact record have all passed. Do
not start the next phase or combine its paths into the current commit before
the current phase gate passes.

A phase gate must protect durable behaviour: a typed boundary, runtime
selection, state transition, safety property, accessibility interaction, or CI
artifact. Broad filters are supplementary only because Cargo succeeds when a
filter selects no tests. Before running each named test target, list it and
assert that every required test name exists.

`tests/ui/required-tests.toml` is the sole named-test manifest. It groups every
test required by [validation.md](validation.md) by phase and target; `just
ui-assert-tests phase=<n>` reads the target's `--list` output and fails on a
missing, duplicate, renamed, ignored, or wrong-target test. It runs before each
test target in local commands and CI. `tools/ui-testing/assert_tests.py
--plan-only` is a Phase-0a artifact: it validates the schema descriptor,
bootstrap contract inventory, traceability source, and manifest before any Rust
target exists. Use `set -o pipefail` for every pipeline. Every displayed
pipeline is directly copyable and includes its own `bash -o pipefail -c` wrapper;
Just/CI wrappers must not silently add required shell semantics.

Use this pattern for every Rust target:

~~~sh
bash -o pipefail -c "cargo test -p <package> --locked --test <target> -- --list | rg -Fx '<test_name>: test'"
cargo test -p <package> --locked --test <target>
~~~

The phase record in [phase-record.md](phase-record.md) records: commit SHA, exact
files/ranges reviewed, required-test list/output, commands, artifacts, result,
and reviewer. Manual review supplements tests; it cannot waive a missing,
empty, failed, or skipped target.

## Commit sequence and path ownership

Create commits in this order. A commit may touch only the listed paths and
direct tests/fixtures for those paths.

| # | Commit title | Paths |
| --- | --- | --- |
| 1 | `docs(plan): freeze UI scenario testing` | `docs/plans/4-testing/**`, `tests/ui/{required-tests,traceability}.toml`, `tools/ui-testing/assert_tests.py` |
| 2 | `feat(contracts): add typed UI workflow seams` | `crates/storage-types/**`, `crates/storage-contracts/**`, `tests/ui/{required-tests,traceability}.toml`, `tools/ui-testing/assert_tests.py`, `justfile`, and their tests |
| 3 | `feat(testing): scaffold complete safe test backend` | `Cargo.toml`, `Cargo.lock`, `crates/test-backend/**` |
| 4 | `refactor(app): inject selected runtime` | `Cargo.toml`, `Cargo.lock`, `src/{lib,main,app}.rs`, `src/{operations,models,state,message,update,subscriptions}/**`, root runtime tests |
| 5 | `refactor(operations): move host workflows behind adapters` | `src/operations/**`, `src/update/**`, `crates/storage-{contracts,udisks,sys,types}/**`, `Cargo.toml`, `Cargo.lock`, affected tests |
| 6 | `feat(test-backend): implement schema and physical state` | `crates/test-backend/**`, `tests/ui/scenarios/{empty.toml,physical/**}` |
| 7 | `feat(test-backend): implement logical, Btrfs, network, and workflow state` | `crates/test-backend/**`, `tests/ui/scenarios/{logical,network,workflows,accessibility,reload}/**` |
| 8 | `feat(ui): launch and test scenario runtime` | `Cargo.toml`, `Cargo.lock`, `src/**`, `tests/ui/**`, `tests/*ui*_contract.rs`, `justfile` |
| 9 | `test(ui): add isolated accessibility and visual runner` | `tools/ui-testing/**`, `tests/ui/cases/**`, `tests/ui/goldens/**` |
| 10 | `ci(ui): gate scenario and Wayland UI tests` | `.github/**`, `justfile`, CI/image support files |
| 11 | `docs(testing): record validation and developer workflow` | `README.md`, `docs/**` |

No implementation commit may modify `tools/storage-testing/**` unless a
separate real-harness change is explicitly requested. The existing harness is
not a staging area for test-backend code.

## Phase-gate matrix

| Phase | Required target | Named-test authority |
| --- | --- | --- |
| 1 | `crates/storage-contracts/tests/ui_workflow_contract.rs` | `required-tests.toml`, phase `1` |
| 2 | `crates/test-backend/tests/contract_surface.rs` | `required-tests.toml`, phase `2` |
| 3 | `tests/ui_runtime_contract.rs` | `required-tests.toml`, phase `3` |
| 4 | `tests/operation_workflow_contract.rs` | `required-tests.toml`, phase `4` |
| 5 | `crates/test-backend/tests/schema.rs` and `physical_state.rs` | `required-tests.toml`, phase `5` |
| 6 | `crates/test-backend/tests/logical_network_state.rs` and `workflow_state.rs` | `required-tests.toml`, phase `6` |
| 7 | `tests/ui_scenario_contract.rs` and `tests/scenario_feature_disabled_contract.rs` | `required-tests.toml`, phase `7` |
| 8 | `tools/ui-testing/tests/test_runner.py` | `required-tests.toml`, phase `8` |
| 9 | GitHub Actions `ui-scenario-contract` and `ui-e2e` jobs | `required-tests.toml`, phase `9` |

Phase 0 and Phase 10 are documentation/final-acceptance phases; their gates
are specified below. A changed contract method expands the relevant named-test
list before the phase is considered complete.

## Phase 0 — record the pre-implementation baseline

1. Record the current commit SHA, workspace members, root package targets,
   feature list, existing test target list, and CI workflow jobs in the phase
   record.
2. Capture the exact locations of `shared()`, every `*Client::new()` call,
   direct `storage_sys`/`std::fs`/`which` storage workflow call, and the
   existing `LabSpec` schema. Classify each call as: production composition,
   app storage path to inject, app-local desktop service, or harness-only.
3. Confirm that `tools/storage-testing` remains non-published, is not an app
   dependency, and is invoked by `just harness-nondestructive`.
4. Confirm the checked-in UI test fixture root is absent or empty before
   creating it; do not repurpose `resources/lab-specs`.
5. Complete Phase 0a before creating a feature code commit. The phase record
   links the frozen schema descriptor, bootstrap contract inventory,
   traceability matrix, required-test manifest, and E2E environment lock.

**Gate**

~~~sh
git status --short
cargo metadata --no-deps --format-version=1
cargo test --workspace --all-features --locked -- --list
~~~

Review the recorded baseline against `spec.md`. The design-lock diff at this
gate contains only `docs/plans/4-testing/**`, `tests/ui/{required-tests,
traceability}.toml`, and `tools/ui-testing/assert_tests.py`; any pre-existing
worktree change is identified as unrelated and left untouched.

**Manual plan-to-code review**

Verify the recorded commit, package/feature inventory, concrete-call inventory,
existing harness boundary, and CI inventory against the starting tree. Record
the exact baseline files and commands in the Phase 0 record.

## Phase 0a — freeze schema, composition, and coverage contracts

Before feature implementation code, complete the v1 descriptor and bootstrap
operation inventory in [scenario-schema-v1.md](scenario-schema-v1.md), the
generated-method coverage source in [traceability.md](traceability.md), and the
complete `tests/ui/{required-tests,traceability}.toml` manifests. This phase
also records the precise portable `RuntimeAdapters`/`DesktopServices` API, the
scenario-safe image-attachment replacement for `OwnedFd` methods, the retained
Tokio-bootstrap lifecycle, and the pinned E2E environment-lock format. The
concrete OCI/package/font lock is created as the first Phase-8 action before
any golden exists. No
state/behaviour field, case selector, operation ID, or test name may be left as
prose such as “typed representation” or “small tolerance”.

The descriptor is complete only after its `dto_catalog` contains a closed
record for every referenced domain/fixture DTO and its transition catalog maps
every active `ScenarioOperation` to closed request, selector, outcome, event,
and trace records. The checked-in declarative source is the source from which
Phase 1 generates loader DTO code and the descriptor; do not let a domain type
name, `serde_json::Value`, or a future state phase fill a missing field. This
work is part of Phase 0a even though the resulting type declarations land in
the Phase-1 contracts commit.

**Gate**

~~~sh
git diff --check
python3 tools/ui-testing/assert_tests.py --plan-only
~~~

The Phase 0a reviewer checks schema/operation/traceability/planned-test-manifest
cross-references, completed DTO/transition catalogs, fixture-path grammar and
scheduled ownership, source-order rules, generated operation stability, and
required E2E environment-lock fields. Phase 1 extends
`ui-plan-check` from the Phase-0a checker to inspect generated contract metadata
and every later phase invokes it.

## Phase 1 — define typed workflow contracts

### Files and exports

1. Add typed values needed by the new seams in `storage-types`. At minimum
   define stable, serde-capable request/progress/result/error values for image
   workflow, usage workflow, synthetic image assets/attachments, and desktop
   service requests/responses. Do not expose `OwnedFd`, a command, arbitrary
   JSON, a callback, a process handle, or a host-only path capability. Migrate
   `storage-contracts::OperationId` in this phase to the locked validated string
   form with separate production-random and scenario-deterministic constructors;
   update its existing serialization tests in the same commit.
2. Add `FilesystemToolDiscovery`, `UsageOperations`, and
   `ImageWorkflowOperations` to `storage-contracts/src/traits` and re-export
   them from both traits/mod.rs and lib.rs. Retain object safety and use
   `async_trait` because implementations are held behind `dyn` trait objects.
3. Define portable `DesktopServices`, `ScenarioControl`, `ScenarioRuntime`, and
   `RuntimeAdapters` in
   `storage-contracts`, while keeping real desktop-service implementation in
   the application crate. `DesktopServices` has app-local semantics and is not
   a storage supertrait; its typed requests select a synthetic image asset or
   cancel, reveal/open a display reference, or open a URL. It must not offer
   arbitrary command execution. `RuntimeAdapters::validate()` owns duplicate
   network-ID and fixed-logical-order validation. `ScenarioRuntime` is the
   scenario-only cross-crate factory result containing adapters and control; the
   root maps adapters to its private `BackendRegistry` and retains control only
   in feature-gated scenario mode.
4. Define the typed image attachment/workflow API and document ownership:
   private production workflow internals own any `OwnedFd`, workflow traits own
   app-facing progress/results, and desktop services own non-storage
   integrations. Keep the existing descriptor-returning
   `ImageDeviceOperations` as a temporary `native_only` bridge until the atomic
   Phase-4 migration. `test-backend` returns typed `Unsupported` from every
   bridge method and never creates an FD. Do not add workflow methods as
   supertraits of `BlockStorageBackend`.
5. Define the contract-surface manifest/macro that generates the public
   storage-trait method declarations, metadata, and `ScenarioOperation`
   inventory from the same declaration. Migrate every existing storage trait
   declaration to that generated source in this commit. `ScenarioControl` is
   the documented fixed-semantics control-plane exception and has no
   `ScenarioOperation`. It must be impossible to add a storage contract method
   without an operation ID and mapping obligation.
6. Write contract tests against fake implementations proving each trait is
   object-safe, values round-trip under JSON/TOML as promised, and
   `DesktopServices` remains outside the storage backend supertrait.
7. Extend the Phase-0a `tests/ui/required-tests.toml`,
   `tests/ui/traceability.toml`, and `tools/ui-testing/assert_tests.py` with
   generated-contract validation, including the completed closed DTO catalog,
   operation-to-selector binding, and operation-to-transition binding. Add `just ui-plan-check` and
   `just ui-assert-tests phase=<n>`. The tool validates the frozen planning
   artifacts and lists every named test from its declared target before later
   phase gates run.

### Gate

~~~sh
just ui-plan-check
just ui-assert-tests phase=1
cargo test -p storage-contracts --locked --test ui_workflow_contract
cargo fmt --all -- --check
~~~

**Manual plan-to-code review**

Inspect every new public type, serde form, trait export, and trait-object use.
Trace one usage-scan, one image-progress, and one desktop action value from
construction to error/result. Confirm a consumer cannot pass a shell fragment,
raw descriptor, callback, or unknown structured payload. Record reviewed
ranges and test output.

## Phase 2 — scaffold the complete safe `test-backend` package

1. Add `crates/test-backend` as a non-published workspace member with package
   name `test-backend` and library crate name `test_backend`. It depends only
   on `storage-types`, `storage-contracts`, Serde/TOML, and deterministic
   async/stream support. It must not depend on `storage-udisks`,
   `storage-sys`, `storage-btrfs`, D-Bus, `which`, or a process-spawning crate.
2. Consume the Phase-1 generated `ScenarioOperation` inventory with one entry
   for every public contract method. The generated contract-surface metadata is
   the implementation checklist; no independently maintained string list or
   source-text grep is permitted.
3. Provide an internal `UnsupportedScenarioBackend` that implements every
   required trait and returns a typed `Unsupported` error naming the operation.
   It exists only to prove completeness and injection before state semantics
   are added. It must never return success, invoke a host API, or initiate a
   device event. During the Phase-1/4 compatibility bridge its legacy
   `ImageDeviceOperations` methods also return `Unsupported`; they never create
   or expose an FD.
4. Add dependency-boundary tests that inspect Cargo metadata, a source-boundary
   check that permits host-I/O APIs only in `store.rs`, and a recording-store
   test using hostile state values. The runtime may receive only private startup
   I/O capabilities, so the test proves every attempted I/O has such a
   capability; it does not falsely claim that Cargo metadata alone proves host
   safety. Add generated-inventory tests that fail when a contract method lacks
   an implementation/mapping. Do not add scenario launch CLI or a fixture parser
   in this phase.
5. Define the actor-handle lifecycle now: construction allocates a 256-command
   queue without spawning; first async dispatch on COSMIC's executor starts one
   actor; no-executor dispatch returns `Unavailable`; and shutdown resolves
   waiters and closes streams. The Phase-2 unsupported backend exercises this
   lifecycle without a fixture parser, so later state code cannot invent a
   second runtime or actor protocol.

### Gate

~~~sh
just ui-assert-tests phase=2
cargo test -p test-backend --locked --test contract_surface
bash -o pipefail -c "! cargo tree -p test-backend --locked -e normal | rg -F 'storage-udisks'"
bash -o pipefail -c "! cargo tree -p test-backend --locked -e normal | rg -F 'cosmic-ext-storage-storage-sys'"
bash -o pipefail -c "! cargo tree -p test-backend --locked -e normal | rg -F 'storage-btrfs'"
cargo fmt --all -- --check
~~~

**Manual plan-to-code review**

Review the package manifest and resolved normal dependency tree. Compare every
contract method to `ScenarioOperation` and its implementation. Exercise an
unsupported operation and verify the trace/error contains only typed/redacted
data. Record the inventory source, dependency tree output, and reviewed ranges.

## Phase 3 — make runtime selection explicit

1. Move application modules into `src/lib.rs` and retain a thin `main.rs`.
   Expose only the app/runtime construction API needed by integration tests.
2. Define `AppRuntime` containing one `Arc<StorageOperations>`,
   `Arc<dyn DesktopServices>`, optional `Arc<dyn ScenarioControl>`, and the
   retained dedicated multi-thread Tokio bootstrap runtime for production mode.
   Add synchronous `AppRuntime::production()`,
   `AppRuntime::from_adapters(RuntimeAdapters)`, and
   `StorageOperations::from_adapters(RuntimeAdapters)` constructors. Production
   construction creates a runtime first, calls private adapter construction with
   `block_on`, clones `desktop`, and moves that same runtime into `AppRuntime`.
   Add a
   root `ui-test` feature containing only the public testing facade; Phase 7's
   `test-backend` feature depends on `ui-test`.
3. Construct only explicit production or test `AppRuntime` values in this
   phase; do not add scenario CLI parsing or fixture loading yet. Create the
   production runtime with the retained bootstrap runtime before
   `cosmic::app::run`. Change application flags from `()` to the ready
   `AppRuntime` and pass it through initialization, models, update helpers,
   background tasks, and subscriptions. Phase 7 owns launch-argument parsing,
   fixture loading, and scenario-mode failure handling.
4. Convert every `*Client::new()` call to a constructor receiving selected
   operations. Convert `UiDrive`, `UiVolume`, `load_all_drives`,
   `build_drive_timed`, logical task closures, image task closures, and device
   subscriptions in the same phase; a partially converted call graph is not
   allowed.
5. Delete `SHARED_OPERATIONS` and `shared()` only after all callers are
   converted. The production factory remains the sole production adapter
   construction site. Use a narrow root-test `RuntimeAdapters` fake to prove
   test runtime construction does not create UDisks; Phase 7 repeats that proof
   with the actual scenario factory.
6. Expose a narrowly public, `ui-test`-gated `ui_test` facade that constructs a
   `Core` and test runtime, dispatches real messages, drains returned tasks,
   and returns read-only state snapshots. It must also expose the ordered task
   drain protocol used by the current `cosmic::app::Task` API; if that API cannot
   be drained without a desktop server, Phase 3 stops and records a design
   decision before application internals are widened. Scenario-clock advancement
   is added only with the concrete Phase-5 `ScenarioControl`. Do not make views,
   states, or mutable internals public just for integration tests.
7. Add a runtime-local factory probe/counter. It proves construction once per
   launch without requiring a live system D-Bus in CI; it is not a global
   production test hook. A bootstrap smoke test proves the retained runtime can
   receive one device event after `cosmic::app::run` has started.

### Gate

~~~sh
just ui-assert-tests phase=3
cargo test -p cosmic-ext-storage --locked --features ui-test --test ui_runtime_contract
cargo fmt --all -- --check
cargo clippy -p cosmic-ext-storage --all-targets --all-features --locked -- -D warnings
~~~

**Manual plan-to-code review**

Trace startup, one device event, and one image completion from app flags through
the selected runtime to the task/subscription. Confirm the test probe selects
only the narrow root-test adapter and production construction is isolated to
the composition root. Confirm no production adapter is imported from a view,
state, model, update, client, or subscription module. Record reviewed ranges.

## Phase 4 — extract host workflows into production adapters

1. Implement Phase-1 interfaces with production adapters preserving current
   filesystem-tool detection, usage validation/scan/delete policy, image file
   creation/copy/progress/cancellation semantics, and desktop chooser/open
   behaviour. Keep system calls and private descriptor handling inside
   production adapter modules only.
2. Replace `StorageOperations.filesystem_tools` and concrete
   `ImageOperationManager` ownership with registered interfaces. Replace
   direct usage/image host calls in `src/operations/**` and
   `src/update/image/**` with typed operation calls.
3. Perform the atomic image-boundary migration: remove the legacy
   descriptor-returning `ImageDeviceOperations` methods from
   `BlockStorageBackend`, update `storage-udisks`, `storage-sys`, root clients,
   and tests in the same commit, and replace all app call sites
   with the typed image workflow/attachment boundary. `loop_setup`/attach
   accepts an image asset reference and the production desktop service resolves
   that reference only inside the production adapter. The scenario
   implementation creates the declared synthetic device and never receives an
   FD or a host image path. A workspace build is mandatory at this gate.
4. Preserve cancellation semantics exactly: current image copy remains
   non-interruptible while blocking; cancellation is checked before and after
   the copy. Do not claim immediate device I/O interruption.
5. Keep protected-path/current-user scanning policy and existing image input
   validation. The scenario backend must not inherit that host implementation.
6. Route every picker/reveal/URL action through `DesktopServices`. The real
   service preserves user-visible production behaviour; a test service records
   calls without opening external programs.

### Gate

~~~sh
just ui-assert-tests phase=4
cargo test -p cosmic-ext-storage --locked --test operation_workflow_contract
cargo test -p cosmic-ext-storage-storage-sys --locked
cargo test --workspace --locked
cargo fmt --all -- --check
~~~

**Manual plan-to-code review**

Trace a valid and rejected usage delete, image backup cancel, and file-picker
request from UI operation facade to the correct adapter. Confirm app code no
longer directly invokes host storage/path APIs and test desktop services cannot
launch an external process. Record behaviour comparison to the baseline.

## Phase 5 — implement TOML schema and physical scenario state

1. Implement the Phase-0a-frozen `schema_version = 1` DTOs in
   `crates/test-backend/src/schema.rs`; do not design fields during this phase.
   Run the frozen byte-level first-key pre-parser before TOML deserialization,
   then use `deny_unknown_fields` at stable levels. Validate all cross references,
   enum tags, IDs, sizes, partition parents, filesystem/mount relationships,
   network IDs, logical keys, fault matchers, event declarations, and synthetic
   path/asset rules before creating runtime state.
2. Implement immutable fixture loading, optional atomic state overlay,
   exact-byte fixture hash/version, virtual clock, FIFO request queue,
   deterministic rule precedence, per-subscriber event queues, redacted
   canonical JSON trace, and `validate`, `materialize-state`, `print-schema`
   utility commands. Fixtures are never written. Overlay replacement follows
   the schema's temp/fsync/rename/parent-fsync/self-write/stale-reload protocol;
   an invalid reload preserves the old runtime. For an overlay-backed mutation,
   write a cloned next state before publishing it; a write failure returns typed
   `Other` with no state, generation, event, or trace-success change. A valid
   restarted overlay resumes at `last_sequence + 1`.
3. Implement metadata, discovery, device event stream, drive, partition,
   filesystem, encryption, filesystem-tool, and desktop-recorder methods over
   `Arc<RwLock<ScenarioRuntime>>` via the FIFO queue. Each success applies a
   declared transition, increments generation once where topology changes, then
   emits events in declaration order. Each fault follows virtual tick/error/
   result semantics. All state-derived I/O attempts must be rejected by
   `ScenarioStore` before reaching a host API.
4. Implement the concrete `ScenarioControl`: `advance_to` and
   `reload_overlay` are actor commands that return a receipt, or for reload an
   applied/rejected outcome containing its receipt, plus read-only diagnostics.
   Phase 7 exposes only these commands as scenario-mode semantic actions; the
   normal runtime receives `None`. This gives the in-process facade and later
   AT-SPI runner one deterministic clock/reload path without a socket or watcher
   timing assertion.
5. Add minimal, separately valid fixtures for empty, physical partition/format,
   busy-unmount, LUKS, and unavailable states. Use `/dev/ui-*`, `/mnt/ui-*`,
   and `asset:<id>` values only; fixture values must not point at a real device
   or host file.
6. Do not implement a watch loop or application CLI yet. Loader/runtime tests
   call reload directly and prove atomicity.

### Gate

~~~sh
just ui-assert-tests phase=5
cargo test -p test-backend --locked --test schema
cargo test -p test-backend --locked --test physical_state
cargo run -p test-backend --locked --bin ui-scenario -- validate tests/ui/scenarios/physical/partition-format.toml
cargo fmt --all -- --check
~~~

**Manual plan-to-code review**

Read the fixture and overlay after a mutation; verify only overlay/trace
changed. Walk one success and one busy fault through matcher selection,
state transition, generation, event, and trace. Confirm parser failure occurs
before runtime construction and all synthetic paths are safe. Record files,
trace, and event order.

## Phase 6 — implement logical, Btrfs, network, usage, and image state

1. Add per-mount Btrfs utility state and every `BtrfsOperations` method. Each
   subvolume/default mutation changes only scenario state; it must never call
   `btrfsutil` or a command.
2. Add logical sources, candidate captures, preflights, confirmed-action key
   validation, action transitions, affected entities, and configured conflicts.
   Preserve existing `LogicalAction::validate` semantics. A stale key returns
   typed conflict before state mutation. A successful logical action publishes
   exactly its declared topology/device changes once. Always expose the fixed
   `[Udisks, LocalTools]` source pair; unavailable source state is data, not a
   registration omission.
3. Add network backend registration, provider schema, config CRUD, test,
   mount status, and mount-on-login state. Reject undeclared backend/config ID
   with typed errors; do not silently create arbitrary providers.
4. Add deterministic usage/image operation state, finite progress schedules,
   wait/cancel/forget lifecycle, and desktop-service picker responses. Use
   synthetic sequence-derived IDs and `asset:<id>` data only. Race tests advance
   `ScenarioClock` and assert queue sequence; they never sleep. A cancel queued
   before completion always wins, while one queued after a completed transition
   returns the documented terminal result without changing it.
5. Add Btrfs/logical/network/image/usage fixtures that exercise success,
   unsupported, permission, stale, progress, cancellation, and blocked-reason
   branches.

### Gate

~~~sh
just ui-assert-tests phase=6
cargo test -p test-backend --locked --test logical_network_state
cargo test -p test-backend --locked --test workflow_state
cargo fmt --all -- --check
cargo clippy -p test-backend --all-targets --locked -- -D warnings
~~~

**Manual plan-to-code review**

Replay a stale logical confirmation, a Btrfs mutation, a network mount, and a
cancelled image operation from fixture rule to trace. Confirm all paths stay
inside `test-backend` state and every event/progress ordering is deterministic.
Record trace extracts with redacted secrets.

## Phase 7 — launch scenario mode and prove the real UI consumes it

1. Add optional root `test-backend` Cargo feature depending on package
   `test-backend` and the existing `ui-test` feature. Default/release/package
   builds must not enable either feature. Parse
   `--backend real|scenario`, `--scenario`, `--scenario-state`,
   `--watch-scenario`, and `--scenario-trace` before runtime construction.
2. In a build without the feature, `--backend scenario` fails before any app or
   real backend construction. In a feature build, invalid scenario/overlay
   fails before opening the window; scenario factory failure has no real
   fallback.
3. Add test-only scenario mode indication, diagnostic state, feature-gated
   semantic `Advance scenario clock`/`Reload scenario` controls, controlled
   watch reload, atomic trace-on-exit, and test `DesktopServices`. Watch events
   merely request a reload; E2E invokes the semantic reload control after an
   atomic replacement and waits for its trace sequence. The normal desktop entry
   point must not show scenario controls.
4. Add library integration tests that drive real `AppModel` messages/tasks over
   scenario fixtures through the `ui_test` facade. They assert UI state,
   dialog/error/progress result, virtual-time/event sequence, and trace rather
   than a mocked view. Do not use `#[path]` copies of app source, a desktop
   server, unbounded timing, or manual error injection.
5. Add `just ui-scenario-check` and `just ui-test scenario=<file>`. The first
   validates all recursive checked-in fixtures, schema descriptor,
   traceability, and required-test manifest; the second requires the feature
   and defaults to no overlay/watch. Add `just ui-assert-tests phase=<n>` and
   make both CI and every phase gate call it.
6. Add `just package-check`, which builds the root release package without
   `test-backend` and proves the binary's help/resources contain no scenario
   surface.

### Gate

~~~sh
just ui-assert-tests phase=7
cargo test -p cosmic-ext-storage --locked --test scenario_feature_disabled_contract
cargo test -p cosmic-ext-storage --locked --features test-backend --test ui_scenario_contract
just ui-scenario-check
just package-check
cargo fmt --all -- --check
~~~

**Manual plan-to-code review**

Launch a copied fixture with a writable overlay, confirm the scenario indicator,
perform a mutation, and inspect overlay/trace. Use the named semantic clock and
reload controls and assert their trace sequences. Launch the independently built
feature-disabled binary with normal arguments and with `--backend scenario`;
confirm there is no scenario surface and that the latter fails before the
production factory is selected. Verify invalid fixture failure in the feature
build. Record command outputs and screenshot.

## Phase 8 — build the isolated accessibility and visual runner

1. Add `tools/ui-testing` case manifests containing scenario reference,
   locale/theme/viewport, semantic AT-SPI selectors/actions/assertions, and
   screenshot checkpoints. Reject a case whose only interaction selector is a
   coordinate.
2. Implement the `pyatspi` runner with semantic-ready wait, action/type/focus
   primitives, keyboard navigation, accessible tree dump, application-bound
   screenshot crop, exact schema comparison rule, and process/session cleanup.
   Fixed viewport, font set, locale, scale, theme, and semantic-ready selector
   are input to every case; wall-clock time is only a diagnostic watchdog.
   Case manifests represent delayed work with explicit `advance_clock` semantic
   actions and live reload with an atomic overlay replacement followed by
   `reload_scenario`; the runner waits for the returned trace sequence, never a
   filesystem-watch delay. Convert AT-SPI logical output bounds to screenshot
   pixels with the locked scale and floor/ceil edge rule.
3. Materialize the Phase-0a environment-lock contract as
   `tools/ui-testing/environment.lock.toml` before creating a case or golden.
   Add `tools/ui-testing/Containerfile`, a digest-pinned base image reference,
   package/font version lock, and the exact Weston-headless/AT-SPI/D-Bus/software
   rendering launch command recorded in that lock. Start with private
   `XDG_RUNTIME_DIR`, `HOME`, `XDG_CONFIG_HOME`, and `XDG_CACHE_HOME`, then run
   `dbus-run-session`; set locked `LANG`, `LC_ALL`, `TZ`, Fontconfig, and COSMIC
   theme values. Do not expose the system bus or inherit host configuration.
4. Add initial cases and reviewed goldens: `physical_partition_format`,
   `busy_unmount`, `luks_unlock`, `logical_preflight_confirmation`,
   `network_mount`, `image_usage_progress`, `keyboard_accessibility`, and
   `live_scenario_reload`.
5. Implement `just ui-e2e` and `just ui-e2e-update`. Baseline update requires
   an explicit flag, writes a review manifest/diff, and cannot run in CI.
6. Implement `tools/ui-testing/run.py --list-cases <root>`. It emits the
   sorted complete case IDs and exits non-zero for an invalid case manifest or
   duplicate ID. `just ui-e2e` must select that exact list; a test case may not
   be silently skipped because a fixture, semantic selector, or screenshot is
   missing.
7. Add a validated case-manifest schema and a unit test that the runner rejects
   a missing image digest, package/font lock, semantic-ready selector, golden,
   case ID, or unsupported image-comparison setting.

### Gate

~~~sh
just ui-assert-tests phase=8
python3 -m unittest tools/ui-testing/tests/test_runner.py -v
tools/ui-testing/run.py --list-cases tests/ui/cases
just ui-e2e
~~~

**Manual plan-to-code review**

Inspect a successful and deliberately failed artifact bundle. Confirm each has
the copied scenario, overlay, trace, log, a11y dump/action log, actual,
expected, and diff image. Replay keyboard-only dialog traversal and inspect
the AT-SPI labels; verify selectors are semantic and screenshot bounds are the
application window, not the desktop.

## Phase 9 — make CI gates enforce the layer

1. Add required `ui-scenario-contract` job to `.github/workflows/ci.yml`. It
   runs `just ui-assert-tests phase=all`, fixed contract/runtime/scenario tests,
   validates every recursive fixture, and does not need a display.
2. Add required `ui-e2e` job using the pinned image. It builds with
   `test-backend`, creates private runtime/D-Bus/AT-SPI/Wayland session,
   executes all named cases, and uploads `ui-artifacts/` on success and
   failure.
3. Preserve existing build, clippy, fmt, and `harness-nondestructive` jobs.
   Do not add the destructive harness to a normal GitHub-hosted runner.
4. Make test reports visible in the GitHub summary and fail the job on a
   missing case, missing baseline, semantic failure, pixel mismatch, leaked
   process, or missing required artifact. Do not hide failures behind retries
   or `continue-on-error`.
5. Remove the broad `paths-ignore: "**/*.md"` policy, or replace it with an
   equivalent include policy, so changes to `docs/plans/4-testing/{spec,
   scenario-schema-v1,traceability,validation}.md`, scenario fixtures, runner,
   manifest, and CI files trigger `ui-scenario-contract`. Unrelated prose may
   remain exempt only through an explicit, tested filter.
6. A repository administrator applies branch protection requiring
   `ui-scenario-contract` and `ui-e2e`, then records the rule, owner, and run
   URLs in `phase-record.md`. This is an external acceptance task, not a YAML
   side effect.

### Gate

On an authorized maintainer test PR, run one deliberate semantic assertion
failure and one expected-image mismatch, then restore the reviewed fixtures.
Confirm both runs fail and upload complete artifacts. Run a clean branch and
confirm branch protection requires and GitHub reports these exact jobs green:

~~~text
ui-scenario-contract
ui-e2e
Rust tests on ubuntu-latest
Clippy on ubuntu-latest
Rustfmt
~~~

Review workflow YAML, container pin, command logs, artifact listing, and job
permissions. Verify the UI jobs use no system bus, privileged device, secret,
or destructive environment variable. Record workflow run URLs and artifact
names in the phase record.

**Manual plan-to-code review**

Compare the clean and deliberately failing workflow runs to the exact declared
case list. Verify a case cannot disappear through a path filter, test filter,
missing fixture, retry, or `continue-on-error`; inspect the uploaded artifact
listing for every required evidence file. Record reviewed workflow ranges and
run URLs.

## Phase 10 — final acceptance and documentation

1. Update README and developer documentation with supported scenario commands,
   feature boundary, fixture/overlay safety, E2E requirements, artifact
   location, and the explicit distinction from `storage-testing`.
2. Cross-check every specification acceptance criterion, every phase gate, and
   every required validation test against the implemented diff. Remove stale
   planned test names or add their missing implementation; do not mark an
   unimplemented case as optional.
3. Run final focused gates plus workspace format/clippy/test and the existing
   safe harness. The real destructive harness remains a separate disposable
   VM acceptance activity.

### Gate

~~~sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
just ui-scenario-check
just harness-nondestructive
just ui-e2e
just package-check
git diff --check
~~~

**Manual plan-to-code review**

Review the final diff against `spec.md` and `validation.md` phase by phase.
Confirm package path/name is `crates/test-backend`/`test-backend` everywhere;
release/package commands omit the optional feature; no app/scenario dependency
reaches `tools/storage-testing`; and all required artifacts exist. Resolve
every unmet criterion before accepting the work.

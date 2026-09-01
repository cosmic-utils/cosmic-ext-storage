# Application-workflow testing v2 revision plan

**Status:** implemented as a feature-gated, headless workflow-test layer on
`4-ui-testing` as of 2026-08-13. The implementation record below supersedes
the earlier proposed production-adapter wording where they differ.

**Authority:** This is the implementation authority for deterministic,
headless application-workflow integration tests. It complements the scenario
backend plan and the Rust/Sway accessibility plan; it does not relax either
one. It may be implemented before editable-field accessibility support lands
in iced.

## Implementation record (2026-08-13)

The implemented layer is deliberately the logical-testing layer requested for
the current accessibility gap. It builds a real `AppModel` shell and real
selected `AppRuntime`/`StorageOperations` adapters, but it does **not** drive
the normal widget `Message` router or claim that it proves widget-to-message
wiring. That proof remains owned by the future AT-SPI/E2E layer.

| Decision | Implemented, deterministic rule |
| --- | --- |
| Build boundary | `src/workflows/**` and `src/testing/**` are compiled only by `feature = "test-backend"`; release builds contain neither the harness nor its test-only state. `application_workflows` has `required-features = ["test-backend"]`, so a normal `cargo test` skips it. |
| Application ownership | `WorkflowHarness::from_fixture` constructs `AppModel::for_workflow_test` with the normal model's concrete state types and no boot tasks. The workflow reducer state is owned by that `AppModel`; the harness never substitutes a mock model or `StorageOperations`. |
| UI boundary | Public tests dispatch typed intents only. `workflow_facade_covers_every_migrated_path` verifies the closed, five-slice facade; it is intentionally not a `Message`-adapter claim. Existing production `Message` paths retain their established behaviour. |
| Runtime isolation | Every effect receives capabilities cloned from that harness's selected `AppRuntime`. A test-only thread-local guard rejects `operations::shared()` while any harness exists; no global cell is installed or reset. |
| Scheduler | The harness executes one FIFO effect, records a redacted completion status in its temporary `trace.json`, then delivers the private completion. Each reducer tracks generation and rejects stale logical, physical, network, image/usage, and reload completions. |
| Fixtures and reload | Primary fixtures are fixed relative paths below `tests/ui/scenarios`. Overlay staging accepts exactly `scenario:<relative.toml>` or `overlay:<relative.toml>`; the latter is rooted at `tests/ui/workflow-overlays` so deliberately malformed reload input is not treated as a runnable scenario. Both roots reject absolute paths, traversal, non-ASCII values, symlink escapes, and non-TOML files. Fixed lower-case SHA-256 verification precedes atomic staging. |
| Trace and secrets | The harness writes only `TraceProjection { sequence, virtual_tick, operation, generation, status }`; it never serializes an effect or completion payload. `SecretInput` has redacted debug output, is zeroed on drop, and the LUKS test scans the populated trace for plain, hex, Base64, and SHA-256 representations. |

The earlier proposed production `Message` adapter and shared production/test
executor remain a separate future migration. They are not prerequisites for
the deterministic logical coverage implemented here and must not be implied by
a green `app-workflow-check` job.

## 1. Decision and outcome

Add a third testing layer between contract/backend tests and graphical E2E:

```text
typed workflow intent
  -> application reducer
  -> ordered typed effect
  -> selected scenario RuntimeAdapters
  -> typed completion/error
  -> same application reducer
  -> view-neutral application snapshot + scenario diagnostics/trace
```

The test runner is an ordinary Rust integration-test binary. It starts no
window, compositor, D-Bus session, AT-SPI service, renderer, virtual keyboard,
or screenshot helper. It runs the real application workflow logic and real
`test-backend` adapters in process.

This layer is called **application-workflow integration testing**. It proves
that the application makes the correct state transition and invokes the
selected backend exactly as its UI intends. It does not prove that a widget is
rendered, discoverable, reachable by keyboard, labelled, or connected to the
right message. Those remain distinct view-level and accessibility/E2E
obligations.

## 2. Why it is needed now

The existing layers establish useful but incomplete facts:

| Existing layer | What it proves | What it bypasses |
| --- | --- | --- |
| `crates/test-backend/tests/**` | Scenario schemas, backend transitions, trace redaction, and no production adapter use. | `AppModel`, dialogs, loading state, confirmation state, and application message routing. |
| `tests/ui_scenario_contract.rs` | `AppRuntime::scenario` selects scenario adapters and a caller can invoke contracts. | The actual application reducer and workflow message adapter; it calls contract methods directly. |
| `ui-e2e-runner capability` | Sway, AT-SPI transport, the app process, the selected marker, and a non-empty widget tree are live. | Every declared workflow action and application state assertion. |
| Future AT-SPI E2E | User-visible semantic actions, visual/tree golden evidence, and keyboard accessibility. | Fast, broad diagnosis of workflow logic independent of toolkit support. |

The current UI path also uses `operations::shared()` from asynchronous update
closures. It is a process-global `OnceCell`; it cannot be reset safely and a
second selected runtime fails installation. That is acceptable for the one
production app process, but it makes a parallel, in-process scenario test
harness unreliable. V2 removes that dependency from every workflow it tests.

## 3. Research record and resulting decisions

Research was performed on 2026-08-01 against this branch's locked
libcosmic/iced source, `stoorps/libcosmic` commit
`3d5fdb087534fb4944e64da2346f3bb673cc0215`, and upstream iced sources. That
revision, rather than an unpinned latest iced release, is the basis for every
toolkit decision below.

| Evidence | Observation | V2 decision |
| --- | --- | --- |
| [iced `Task` API](https://docs.rs/iced/0.14.0/iced/struct.Task.html) | A task is a private, concurrent runtime action that may emit one or more messages. | Tests do not inspect, discard, or attempt to drain toolkit `Task` internals. The effect executor is a normal async function shared by the app adapter and the test harness. |
| [iced headless testing PR #2698](https://github.com/iced-rs/iced/pull/2698) | `iced_test::Simulator` can select widgets and simulate interactions. The checked-out simulator turns a selected bounded target into a centre-point click. | Do not use it for V2's workflow truth. It is view interaction, not a coordinate-free semantic boundary. |
| [iced test framework PR #3059](https://github.com/iced-rs/iced/pull/3059) | The emulator executes real tasks/subscriptions, but its `.ice` format is explicitly described as likely to change. | Do not add `iced_test`, `iced_tester`, `.ice`, or test-recorder dependencies in V2. Re-evaluate only as an optional view smoke layer after an exact compatibility spike. |
| [semantic selector PR #3324](https://github.com/iced-rs/iced/pull/3324) | Upstream is adding role/label/test-ID selectors to the headless framework. | Do not depend on an open/upstream-only selector API. Stable application IDs remain an AT-SPI/E2E concern. |
| [accessibility issue #552](https://github.com/iced-rs/iced/issues/552) and [AccessKit PR #3111](https://github.com/iced-rs/iced/pull/3111) | Persistent widget identity, widget action support, and platform accessibility are still evolving. | Do not delay logical workflow coverage on accessibility-roadmap timing, and do not treat this layer as an accessibility substitute. |
| This branch's `src/update/mod.rs`, `src/app.rs`, `src/operations/mod.rs` | `AppModel` owns the relevant state, update code creates `Task`s, and many operation paths still call the global selected context. | Extract workflow reducers/effect executors in vertical slices. The app and tests call the same reducer/executor code; target slices receive operations explicitly. |
| `test-backend` and `ScenarioControl` | Fixtures are typed, operations are deterministic, virtual time is explicit, and `advance_to`, `reload_overlay`, and `diagnostics` are the only semantic control operations. | The harness calls the in-process `ScenarioControl` trait directly. It never starts the Unix-socket control server and never invents time. |

### Rejected alternatives

1. **AT-SPI-only tests now:** blocked for editable fields and does not give
   fast, local diagnostic coverage of state/error paths.
2. **A fake `AppModel` or mocked `StorageOperations`:** rejected. It could
   make reducer tests green while the scenario adapter wiring is broken.
3. **Calling backend traits directly and calling that application coverage:**
   rejected. That is the existing gap.
4. **Constructing a `Task`, then ignoring it in a test:** rejected. A passing
   test would not establish that the effect was invoked or that its completion
   was reduced.
5. **Running one scenario per subprocess or serialising all tests to work
   around `OnceCell`:** rejected. It hides rather than removes the selected
   runtime leak and makes the normal test target unnecessarily slow.
6. **Using sleeps, Tokio paused time, or a test timeout as workflow state:**
   rejected. Scenario time advances only through an expected virtual-clock
   receipt.

## 4. Scope

### In scope

- View-neutral reducers for the workflow slices named in section 10.
- Ordered, typed effects and shared async executors using an explicit
  `Arc<StorageOperations>`/`AppRuntime` selected at construction.
- A feature-gated, in-process `WorkflowHarness` that builds an `AppModel`
  without starting normal boot tasks or installing the global operations cell.
- Exact state snapshots, effect logs, scenario diagnostics, and redacted trace
  assertions.
- Test registration in `tests/ui/required-tests.toml`, traceability updates,
  a narrow CI target, and reviewable named-test gates.
- Migration of the selected workflow paths away from `operations::shared()`.

### Explicitly out of scope

- Widgets, render output, fonts, viewport sizing, screenshot goldens, Sway,
  Wayland, AT-SPI, keyboard navigation, mouse coordinates, and accessibility
  IDs/actions.
- Replacing the Rust AT-SPI runner, its capability gate, or its future v2 case
  executor.
- A generic event-sourcing framework, a global `Effect` enum for all 91
  `ScenarioOperation`s, or a second app state model.
- A production feature flag, test-only CLI subcommand, test server, shell,
  Python runtime, mocked backend, direct UDisks/rclone/host-filesystem access,
  or a real storage-testing harness invocation.
- Testing the `keyboard_accessibility` flow here. That flow is intentionally
  owned by accessibility E2E.

## 5. Fixed terminology and contracts

| Term | Meaning | Allowed contents |
| --- | --- | --- |
| **Intent** | A user-meaningful request entering one workflow reducer. | Typed IDs, typed selections, and non-secret form values. Never a widget ID, point, key sequence, or raw JSON. |
| **Reducer** | Synchronous state transition from an intent or completion to a new state plus ordered effects. | State mutations, validation, stale-result rejection, dialog/pending/error transitions. No I/O, clock reads, task creation, or global lookup. |
| **Effect** | A closed request to an injected application capability. | A typed contract request plus enough correlation data to create its completion. No closure, trait object, shell fragment, host path, or a `String`/`&str` secret. A sensitive request may own the non-debuggable `SecretInput` defined below. |
| **Executor** | Async implementation of a workflow effect using the selected runtime. | The real `StorageOperations` contract objects, `DesktopServices`, and (for the reload slice only) `ScenarioControl`; no mock implementation in V2 tests. |
| **Completion** | Typed result returned by the executor to the same reducer. | Success/result or a display-safe `OperationError` classification converted at the app boundary. It carries the effect correlation key. |
| **Snapshot** | A stable, view-neutral projection asserted by a test. | Public identifiers, workflow phase, pending/error kind, selected item, dialog kind, and redacted display-safe text. No `Element`, renderer, `Core`, task, or secret. |

The test API must not expose or accept `Message` as its public vocabulary.
`Message` remains the internal toolkit/application routing type. Each migrated
update arm adapts a `Message` to one typed workflow intent and adapts a typed
completion back to its existing `Message` variant. The public harness begins
at the typed intent boundary and cannot create a hand-written
`*Loaded`/`*Finished` message that the executor would never emit. A separate
private adapter-contract verifier, invoked by the required section-10 test
through a zero-`Message` test-facade method, exercises every migrated
inbound/outbound `Message` route without exposing `Message` through the
harness API.

### 5.1 Secret boundary

V2 introduces one production-owned `SecretInput(Vec<u8>)` type for workflows
that submit LUKS input. It intentionally is not `Clone`, `Copy`, `Serialize`,
`Deserialize`, or `Eq`, and does not derive `Debug`; its manual `Debug`
implementation emits only `SecretInput(<redacted>)`. Construction consumes the existing dialog `String`
with `into_bytes`; the dialog field is cleared with `mem::take` before an
effect is created. The executor borrows a validated UTF-8 view only for the
backend call, retains the `SecretInput` across that await, and overwrites its
buffer with zero bytes in `Drop`. It never turns the value back into `String`.

An effect record is a separate, display-safe `EffectRecord` projection. It
records `operation`, correlation key, and `has_secret: bool`; it never formats
the effect itself. `FixtureSecrets` is likewise a non-debuggable, consuming
test input. It binds fixture secret IDs to test values in memory when creating
a scenario runtime. The test creates a second `SecretInput` for the user
intent; neither input is written to an overlay, trace, diagnostics, snapshot,
or assertion failure. The test-only backend's existing in-memory expected
secret map is permitted to retain this fixture value only for the lifetime of
that harness. Fixture TOML continues to contain secret IDs only.

## 6. Target architecture

### 6.1 Module ownership

Create these modules; do not create a new workspace crate:

```text
src/
  workflows/
    mod.rs                    # shared private workflow traits/types
    logical.rs                # logical reducer, effect, completion, snapshot
    physical.rs               # partition/format, unmount, LUKS slices
    network.rs                # configuration/mount slice
    image_usage.rs            # image and usage lifecycle slices
    reload.rs                 # scenario-control reload/virtual-time slice
  testing/
    mod.rs                    # compiled only with feature = "test-backend"
    workflow_harness.rs       # public test-only facade and diagnostics
tests/
  application_workflows.rs    # integration tests only; no desktop session
```

In the implemented revision, both `src/workflows/**` and `src/testing/**` are
compiled only by `test-backend`; `src/testing/**` is the only public test
facade. `src/lib.rs` exports it exactly as:

```rust
#[cfg(feature = "test-backend")]
pub mod testing;
```

No type from `src/testing/**` may appear in a default-feature public signature.
The workflow modules themselves remain private to the crate. Their intent and
snapshot types use `pub`, rather than `pub(crate)`, solely so `testing` can
re-export the exact same types to the external integration-test crate; they
are unreachable without `feature = "test-backend"`.

### 6.2 Vertical-slice reducer shape

Use one small, non-serialized type family per workflow module. Do not build a
single dynamic dispatcher.

```rust
pub enum LogicalIntent { /* closed, typed inputs */ }
pub(crate) enum LogicalEffect { /* closed, typed contract requests */ }
pub(crate) enum LogicalCompletion { /* effect-keyed success/error */ }
pub struct LogicalSnapshot { /* Eq + Debug, display-safe */ }

pub(crate) fn reduce_intent(
    state: &mut LogicalState,
    intent: LogicalIntent,
) -> Vec<LogicalEffect>;

pub(crate) fn reduce_completion(
    state: &mut LogicalState,
    completion: LogicalCompletion,
) -> Vec<LogicalEffect>;

pub(crate) async fn execute(
    capabilities: &WorkflowCapabilities,
    effect: LogicalEffect,
) -> LogicalCompletion;
```

Physical, network, image/usage, and reload modules follow the same shape but
only with types they actually need. Reducers return effects in declaration
order. V2 effects for a single workflow are executed FIFO; a reducer may not
return two effects whose outcomes race to determine user-visible state. A
genuinely concurrent production workflow is out of scope until it has an
explicit correlation and ordering contract.

Each effect has a stable internal correlation key:

```text
EffectKey { workflow: WorkflowKind, operation: ScenarioOperationId,
            generation: u64, issue: u64 }
```

`issue` is the per-workflow queue's monotonically increasing issue number; it
starts at one for every fresh `AppModel` and increments exactly once when an
effect envelope is appended. A completion with a wrong generation, operation,
issue number, or stale preflight key is ignored/rejected according to the
existing workflow semantics and is asserted in a named test.

`workflows::mod` owns a small generic `EffectQueue<E>`, held inside the
corresponding workflow state. It contains `next_issue: u64`,
`pending: VecDeque<Envelope<E>>`, and `in_flight: Option<EffectKey>`. The
shared scheduling rule is fixed:

1. A reducer appends its enveloped effects to `pending` in returned order.
2. If `in_flight` is empty, schedule and remove exactly the queue head, then
   set `in_flight` to its key.
3. Only a completion with that exact key is reduced. It clears `in_flight`.
4. The completion reducer either emits no further work (including terminal
   error/cancel) or appends new work; then the scheduler starts the next head.

The app adapter and harness use this same queue/scheduling helper. V2 never
uses `Task::batch` for workflow effects: its declaration order is not a
completion-order guarantee. An error clears any pending dependent work unless
the reducer returns a separately reviewed cleanup effect; V2 has no such
cleanup exception initially.

`workflows::mod` also owns this crate-private capability bundle:

```rust
pub(crate) struct WorkflowCapabilities {
    operations: Arc<StorageOperations>,
    desktop: Arc<dyn DesktopServices>,
    scenario_control: Option<Arc<dyn ScenarioControl>>,
}
```

It is constructed only from `&AppRuntime`, cloning the selected capability
arcs. Every executor receives `&WorkflowCapabilities`; a reload effect fails
with typed `OperationError::Unsupported` when `scenario_control` is absent.
The harness obtains the same bundle from its selected scenario `AppRuntime`.
It cannot construct a bundle, substitute a capability, or observe its fields.

### 6.3 Implemented harness adapter

The originally proposed production-message adapter is deferred. The implemented
adapter follows these rules inside the harness instead:

1. Dispatch a typed intent against the harness `AppModel` workflow state.
2. Enqueue the resulting closed effects FIFO.
3. Await exactly one effect against `WorkflowCapabilities::from(&AppRuntime)`
   and put its private completion in a one-item mailbox.
4. Deliver only that completion to the same reducer. A later queued effect is
   not executed until delivery succeeds.

The executor is the sole place that invokes storage traits. It receives the
selected operations explicitly; a migrated path must not call `shared()`, a
`*Client::new()`, `StorageOperations::new()`, `std::fs`, `storage_sys`, or a
desktop/global service directly.

`verify_workflow_facade_contract()` checks that the five typed entry points are
non-empty and unique. It does not inspect, construct, or claim coverage of a
widget or `Message` route.

### 6.4 Test harness API

`cosmic_ext_storage::testing::WorkflowHarness` is the entire external test
surface. Its precise construction and methods are:

```rust
let mut harness = WorkflowHarness::from_fixture(
    "logical/preflight.toml",
    FixtureSecrets::none(),
).await?;

harness.dispatch_logical(LogicalIntent::Open { device_path: None })?;
harness.drive_until_idle().await?;
assert_eq!(harness.logical_snapshot(), expected);
assert_eq!(harness.diagnostics().await?, expected_diagnostics);
```

Fixed requirements:

- Every fallible facade method returns only
  `WorkflowHarnessError::{InvalidFixtureReference, FixtureIntegrityMismatch,
  TemporaryRootUnavailable, ScenarioBootstrapFailed, ScenarioControlUnavailable,
  SchedulerProtocolViolation, TraceContainsSensitiveInput}`. These variants
  carry no path, operation payload, source error, secret, or formatted
  backend message. Backend success/failure remains a typed workflow
  completion/snapshot, not a harness-construction error.
- `from_fixture(relative_fixture, fixture_secrets)` accepts a non-empty,
  forward-slash relative path below `tests/ui/scenarios`. It rejects an
  absolute path, `.`/`..` component, non-UTF-8 component, symlink escape, or
  extension other than `.toml`, then resolves the validated path from
  `CARGO_MANIFEST_DIR/tests/ui/scenarios`. It is not given a general host
  path and has no separate `fixture()` helper.
- `from_fixture` uses a new private
  `AppRuntime::scenario_with_fixture_secrets` that shares
  `AppRuntime::scenario`'s `ScenarioRuntime -> RuntimeAdapters ->
  StorageOperations` composition path, but calls `ScenarioRuntime::load_with_secrets`
  when `FixtureSecrets` is non-empty. It never calls `Application::init` or
  `AppRuntime::install`.
- A private `AppModel::for_workflow_test(runtime)` creates exactly
  `Core::default()`, `ContextPage::default()`, `nav_bar::Model::default()`,
  `SidebarState::default()` (without calling `set_network_loading`), `None`
  dialog/image operation ID, empty filesystem tools, `NetworkState::new()`,
  `LogicalState::default()`, and `Config::default()`. It starts no
  drive/tools/network boot task, calls no `update_title`, and reads no user
  configuration.
- `FixtureSecrets` has only `none()` and consuming `insert(secret_id,
  SecretInput)` constructors. It rejects an empty ID and duplicate ID. It has
  no iterator, getter, `Debug`, `Clone`, `Default`, or path/environment field.
- The root package adds `tempfile = "=3.27.0"`, `base64 = "0.22.1"`,
  `sha2 = "0.10.9"`, and `serde_json = "1.0.140"` as **optional normal
  dependencies** enabled by `test-backend` (not root dev-dependencies,
  because `src/testing` is feature-compiled library code). These versions
  reuse the workspace/lockfile versions already used by the scenario tooling.
  The harness owns a fresh mode-0700 temporary root per test, writes only
  `overlay.toml` and `trace.json` beneath it, and removes the root on drop.
  V2 deliberately has no artifact-retention environment variable or option.
- `stage_overlay_from_fixture(source, expected_sha256)` is the only overlay-write
  API. `source` must be exactly `scenario:<relative.toml>` or
  `overlay:<relative.toml>`; these resolve respectively beneath
  `tests/ui/scenarios` and `tests/ui/workflow-overlays` under the same strict
  repository-relative rules. It verifies the supplied lower-case 64-hex
  SHA-256 before atomically writing the bytes to the harness-owned
  `overlay.toml`. `reload_overlay` then calls only the in-process
  `ScenarioControl::reload_overlay`. The harness never starts or speaks the
  Unix control protocol.
- `dispatch_logical` (and each other typed dispatch method) is synchronous and performs only the
  shared intent adapter/reducer/scheduler transition. It does not invoke I/O
  or manufacture a completion. `execute_scheduled` takes the one scheduled
  real effect, awaits the shared executor, and puts that executor-produced
  completion in a private one-item mailbox. `deliver_completion` takes that
  private completion through the shared completion adapter/reducer/scheduler.
  `drive_until_idle` repeats those two calls in order until no scheduled
  effect, mailbox completion, or pending effect remains.
- The mailbox is the only deliberate delivery boundary. It is required for
  stale-completion tests: dispatch intent A, dispatch superseding intent B,
  execute A, then deliver A's real completion. The test never builds a
  completion, calls a `*Finished` message, or changes backend internals.
  `execute_scheduled` fails if the mailbox is occupied, and
  `deliver_completion` fails if it is empty; no execution is concurrent in a
  V2 harness.
- The harness has no `Task`, `Stream`, Tokio runtime handle, window, D-Bus
  socket, or renderer access.
- `advance_to`, `reload_overlay`, and `diagnostics` call the selected
  in-process `ScenarioControl` trait. The Unix control protocol is not
  constructed, authenticated, or tested here; it is covered separately.
- The harness exposes only immutable snapshots, ordered effect records, and
  redacted trace/diagnostic projections. It never lends mutable `AppModel`,
  `StorageOperations`, or raw scenario backend state to a test. Its exact
  public dispatch/snapshot pairs are `dispatch_logical`/`logical_snapshot`,
  `dispatch_physical`/`physical_snapshot`, `dispatch_network`/`network_snapshot`,
  `dispatch_image_usage`/`image_usage_snapshot`, and
  `dispatch_reload`/`reload_snapshot`; it has no generic dynamic dispatcher.
  `execute_scheduled`, `deliver_completion`, `drive_until_idle`,
  `effect_records`, `trace`, `assert_trace_redacted_for`, `diagnostics`, and
  `stage_overlay_from_fixture` are the only cross-slice methods. The module's
  one non-harness facade function is `verify_workflow_facade_contract()`;
  it accepts no input and returns only a pass/fail `WorkflowHarnessError`.
- `trace()` parses the harness-owned canonical trace JSON and returns only
  `TraceProjection { sequence, virtual_tick, operation, generation, status }`.
  `assert_trace_redacted_for(secret)` consumes its `SecretInput` and scans the
  raw trace bytes for the UTF-8 secret, lower-case hex, standard Base64, and
  lower-case SHA-256 encodings. Its only failure text is the fixed
  `"trace contains sensitive input"`; it never returns or prints the matched
  bytes. This is how the LUKS test proves redaction without exposing a secret
  through an assertion failure.

### 6.5 Global-context migration rule

V2 migrates every `shared()` use reached by its named workflows. A test may
run with multiple harnesses concurrently; each must use only its own selected
`Arc<StorageOperations>`. The implementation must not add a reset method,
`unsafe` mutation, thread-local replacement, test mutex, or `serial_test`
workaround for `SHARED_OPERATIONS`.

The legacy global remains a production compatibility bridge for update paths
outside V2. A new `operations::with_operations` constructor is allowed only
when it accepts an explicit `Arc<StorageOperations>` and is used by the effect
executor. Each migrated `*Client` gets such a constructor before its global
`new()` constructor is removed from that workflow.

## 7. Determinism, isolation, and safety contract

Every V2 test must satisfy all of the following:

| Concern | Fixed rule |
| --- | --- |
| Runtime selection | Load exactly one named scenario fixture into one harness. Assert the selected block backend ID is `ui-scenario`. |
| I/O | Only fixture reads and harness-owned overlay/trace writes are permitted. No UDisks, system D-Bus, rclone config, host mount scan, host browser, subprocess, socket server, or external network. |
| Time | No `sleep`, timeout-as-success, wall-clock assertion, `tokio::time`, random delay, or polling loop. Scenario progress occurs only after `advance_to(expected_tick)` and exact receipts are asserted. |
| Ordering | Intent/effect/completion records have a monotonically increasing harness sequence. Effects run FIFO. Tests assert the complete ordered effect list for a workflow, not merely its final state. |
| Errors | Convert `OperationError` only at the defined workflow boundary. Preserve its stable kind and exact display-safe reason; do not assert debug strings or log timing. |
| Secrets | Pass test LUKS secrets through a dedicated in-memory `SecretInput` value. Snapshots, effect logs, diagnostics, overlays, and traces must contain neither the secret nor its base64/hex/hash encoding. |
| Fixture mutation | Read-only fixture files are never changed. A reload test stages a checked-in overlay through the harness-only staging method, checks its SHA-256, then calls `reload_overlay`. |
| Test parallelism | All `application_workflows` tests use `#[tokio::test(flavor = "current_thread")]` and pass under Cargo's default parallel test execution. The guard is thread-local; tests require no shared process lock. |
| Failure evidence | Test failure includes `WorkflowSnapshot`, ordered effect/completion list, and redacted `ScenarioDiagnostics`. No success artifact is required. |
| Reproducibility | No random scenario ID. Operation IDs, virtual ticks, and expected diagnostics are fixture-derived and exact. |

## 8. Snapshot contract

Snapshots are manually defined, `Debug + Clone + Eq + PartialEq` structs.
They are not serialized as long-lived goldens in V2. A snapshot is a small
domain projection, for example:

```text
LogicalSnapshot {
  phase: AwaitingConfirmation,
  selected_device: Some("/dev/ui-disk0"),
  selected_entity: Some("vg/ui-vg"),
  preflight_generation: 4,
  confirmation: DestructiveReview,
  error: None,
}
```

Rules:

1. A snapshot names workflow phase/domain state, not an `Element`, widget
   label, layout value, internal task, or implementation pointer.
2. A snapshot may contain display-safe identifiers already present in a
   fixture. It may not contain a password, token, arbitrary host path, raw
   result object, or backend trait object.
3. The snapshot includes a pending operation/generation only when that is an
   observable correctness condition, such as stale-preflight prevention.
4. New snapshot fields require a test that asserts both the success path and
   the configured error/cancellation/stale path affected by that field.
5. Snapshot equality is asserted inline in Rust. PNG, text, JSON, TOML, or
   `insta` snapshots are deliberately not introduced by this plan.

## 9. Effect and completion contract

Each module owns a closed, reviewed mapping:

| Slice | Intent | Effects | Completion conditions |
| --- | --- | --- | --- |
| Logical | open/select/preflight/confirm/cancel | capture candidate, load entities, preflight, execute confirmed action | candidate resolution, matching preflight key, operation outcome, one refresh generation |
| Physical | request/validate/confirm format; request unmount; request unlock | partition/filesystem mutation, unmount, LUKS unlock | created device/event sequence, retained busy reason, unlock success/error, no secret leak |
| Network | edit/create/test/mount/unmount/toggle login | typed network backend calls | configuration/mount status and actionable configured error |
| Image and usage | create/attach/start/cancel/forget; start scan/delete | typed image/usage workflow calls | operation ID, virtual-tick progress, terminal-state legality, redacted delete result |
| Reload | advance virtual clock; reload staged overlay | `ScenarioControl` calls only | exact receipt sequence/generation/tick and atomic accepted/rejected reload state |

`DesktopServices` actions in V2 are tested only as typed effect construction
and completion mapping. The harness's scenario desktop service records a
display-safe request; it does not open a URL, reveal a path, or select a host
file.

An executor converts each effect to exactly one completion. A completion must
include the originating effect key. It must not read current mutable app state
to reconstruct its key. This makes a stale result detectable after a new
intent has invalidated an earlier generation.

## 10. Required coverage matrix

The following named tests are the minimum V2 acceptance set. They are one
`phase = "workflow-v2"`, `kind = "rust-integration"`,
`package = "cosmic-ext-storage"`, `name = "application_workflows"`,
`features = ["test-backend"]` target in `tests/ui/required-tests.toml`. The
names are globally unique and must also appear verbatim in
`docs/plans/4-testing/validation.md`.

| Test name | Fixture | Exact proof |
| --- | --- | --- |
| `workflow_harness_uses_only_selected_scenario_runtime` | `empty.toml` | The harness has no boot task, no global install, and the block backend ID is `ui-scenario`. Two harnesses coexist. |
| `logical_open_preflight_confirm_executes_once_and_refreshes_once` | `logical/preflight.toml` | Intent → capture/load/preflight → confirmation → execute produces one expected operation and exactly one refresh generation. |
| `logical_stale_preflight_completion_is_rejected` | `logical/preflight.toml` | A completion carrying an old key cannot overwrite a newer selection or execute an action. |
| `partition_format_validation_and_completion_preserve_effect_order` | `physical/partition-format.toml` | Invalid form emits no effect; valid confirmation executes the typed partition/filesystem effects in declared order and observes the fixture event. |
| `busy_unmount_keeps_actionable_error_and_does_not_refresh` | `physical/busy-unmount.toml` | Configured busy failure keeps the expected dialog/state and reason; no success refresh is emitted. |
| `luks_unlock_uses_secret_input_and_redacts_every_projection` | `physical/luks.toml` | Success and configured failure use the typed secret boundary; snapshot, trace, diagnostics, and effect history contain no secret representation. |
| `network_create_mount_and_status_are_reduced_from_one_flow` | `network/mount.toml` | Create/test/mount status transitions use the scenario network backend and leave deterministic state. |
| `image_progress_cancel_and_terminal_state_are_virtual_clock_driven` | `workflows/image-usage.toml` | Progress follows exact `advance_to` receipts; cancel prevents later successful completion. |
| `image_usage_stale_completion_cannot_replace_newer_workflow_state` | `workflows/image-usage.toml` | A completed image-start effect cannot overwrite a newer usage-scan intent. |
| `usage_scan_and_delete_map_results_without_host_file_access` | `workflows/image-usage.toml` | Scan/delete effects use the selected usage contract; result/error transitions are reflected in the snapshot. |
| `scenario_reload_is_atomic_and_generation_checked_by_the_application` | `reload/live.toml` | Valid reload applies once, invalid overlay is rejected without partial state, and generation/sequence values match expected receipts. |
| `reload_stale_completion_cannot_move_virtual_time_backwards` | `reload/live.toml` | A completed earlier advance cannot overwrite a superseding later advance. |
| `workflow_effects_do_not_call_global_operations_context` | all migrated fixtures | A test-only instrumentation guard makes any V2-path call to `shared()` fail deterministically. |
| `workflow_facade_covers_every_migrated_path` | none | The zero-`Message` facade verifies that the five typed workflow entry points remain closed and distinct. It makes no widget/message-routing claim. |

`keyboard_accessibility` has no V2 test entry. Its traceability row remains
E2E-only and must keep a future AT-SPI assertion. No V2 test may claim it.

For every success test above, add the matching configured error, stale, cancel,
or invalid-input assertion in the same named test unless the row already names
that negative branch separately. Test names describe the end-user invariant,
not the helper function.

The two foundation tests (`workflow_harness_uses_only_selected_scenario_runtime`
and `workflow_effects_do_not_call_global_operations_context`) enter the target
in W1. Each W2–W5 commit appends only the test or tests it implements to the
same target and adds the same names to validation and traceability in that
commit. This makes `just ui-assert-tests phase=workflow-v2` green at every
commit; it never declares a future test before that test exists.
`workflow_facade_covers_every_migrated_path` is appended in W5, once the final
reload slice exists.

`tests/ui/traceability.toml` gains an optional
`application_workflow_tests = ["..."]` field on a flow. The logical,
physical, network, image/usage, and reload flow rows list their corresponding
V2 names from this table. The two foundation tests are declared once in a
top-level `workflow_v2_cross_cutting_tests` array; W5 appends the facade test
to that same array. W1 extends `assert_tests.py` to require that
the current `workflow-v2` target equals the deduplicated union of those two
sources, and that `keyboard_accessibility` does not name an
application-workflow test. This is the authoritative mapping; the existing
`in_process_test` field retains its earlier direct-contract meaning and is not
repurposed.

## 11. Test-only instrumentation guard

V2 adds a narrow, feature-gated guard around global context lookup:

```rust
#[cfg(feature = "test-backend")]
pub(crate) fn reject_global_operations_for_workflow_tests() -> Guard;
```

While a guard is live on the current test thread, `operations::shared()` must
return `OperationError::Failed("workflow test attempted global operations context")`.
The guard is a `thread_local! Cell<u32>` nesting count. Its `Guard` owns a
`PhantomData<Rc<()>>` so it is `!Send`; it is held by the harness for the
whole test. `shared()` checks the count before reading `SHARED_OPERATIONS`.
Drop decrements the count and asserts that it was nonzero. The guard is
thread-local, nestable, and restores its prior value on drop. It does not
reset, overwrite, or inspect `SHARED_OPERATIONS`.

`WorkflowHarness::from_fixture` enables this guard before dispatching an
intent. Therefore any unmigrated target path fails as soon as it tries to use
the global context. Ordinary existing runtime tests do not enable the guard and
retain their current behaviour until their own path is migrated.

The guard is a migration detector, not a production safety mechanism; it is
compiled out of default builds.

## 12. Fixture, trace, and scenario-control rules

1. The existing fixture name remains the only scenario selector. V2 creates no
   second workflow-fixture format.
2. Every test gets a harness-owned `overlay.toml` and `trace.json`; a reload
   test stages a checked-in overlay fixture while all other tests leave the
   overlay absent. It states the exact lower-case SHA-256 in test source. The
   harness, rather than a test, is the only V2 code allowed to write that
   path.
3. The in-process `ScenarioControl` is called directly. The test asserts the
   returned `ScenarioReceipt`/`ScenarioReload` immediately; it never polls a
   socket or waits for app redraw.
4. Each virtual-time test begins at the fixture's declared tick (zero unless
   fixture schema later changes), advances to named absolute ticks, and
   asserts the returned `{ sequence, generation, virtual_tick }` exactly.
5. Trace assertions match the typed operation/status and verify explicit
   absence of secret strings. They do not compare source file paths, process
   IDs, elapsed time, logging prefixes, or randomized temp paths.
6. Failure injection remains in the fixture/overlay behaviour rules. A test
   must not modify adapter internals or introduce a mock-only error channel.

## 13. Implementation sequence and commit ownership

Do the following commits in order. Do not combine a later slice with the
foundation commit.

| Commit | Title | Permitted paths | Completion gate |
| --- | --- | --- | --- |
| W0 | `docs(testing): define deterministic application workflow plan` | `docs/plans/4-testing/**` | `just ui-plan-check`, markdown/link review |
| W1 | `refactor(workflows): add explicit effect foundation` | `src/{lib,app,runtime,operations,state}/**`, `src/workflows/**`, `src/testing/**`, `Cargo.toml`, `Cargo.lock`, `tests/application_workflows.rs`, `tests/ui/{required-tests,traceability}.toml`, `tools/ui-testing/assert_tests.py`, validation docs | Harness coexistence/global-guard test plus formatter/clippy |
| W2 | `test(workflows): cover physical and encryption flows` | W1 paths plus `src/update/**`, physical fixtures/traceability/validation | partition, busy-unmount, and LUKS named tests |
| W3 | `test(workflows): cover logical preflight flow` | W1 paths plus logical state/update/fixture/traceability/validation | both logical named tests |
| W4 | `test(workflows): cover network image and usage flows` | W1 paths plus network/image/usage updates and fixtures/traceability/validation | network, image, and usage named tests |
| W5 | `test(workflows): cover scenario reload at app boundary` | W1 paths plus reload/control tests/fixture/traceability/validation | reload and facade named tests plus existing control/schema tests |
| W6 | `ci(testing): gate application workflow integration` | `.github/workflows/**`, `justfile`, required-test/validation docs | fresh CI command, no desktop dependency, all named tests selected |

W1 must leave existing production launch behaviour unchanged. A commit that
migrates one operation path must add its named test in the same commit. No
commit in this sequence may modify `crates/ui-e2e-runner/**` or UI goldens
except a documentation cross-reference.

## 14. CI and developer commands

Add these recipes without changing the meaning of `just ui-e2e`:

```just
app-workflow-check:
    just ui-assert-tests phase='workflow-v2'
    cargo test -p cosmic-ext-storage --features test-backend --locked --test application_workflows
```

The CI job is named **Application workflow integration**. It installs the
normal Rust/native build prerequisites already required by the project; it does
not install Sway, grim, wtype, at-spi2-core, a browser, Python packages, or
Docker. It runs, in order:

```sh
just ui-plan-check
just app-workflow-check
cargo test -p test-backend --locked
cargo test -p cosmic-ext-storage --features test-backend --locked --test ui_scenario_contract
```

The required-test manifest checker must list
`application_workflows` first and verify each name before executing its target.
The checker already accepts non-numeric phase strings; update its command-line
help from `number|all` to `name|all` in W1.

The E2E CI job remains capability-only until the E2E revision plan's R4 case
executor is complete. A green workflow integration job must not be displayed
or described as an accessibility/visual E2E result.

## 15. Review checklist

Every W1–W6 review answers yes to all applicable checks:

1. Does the reducer contain no I/O, task creation, global lookup, clock read,
   secret formatting, or renderer/toolkit type?
2. Does the executor receive explicit selected operations/runtime, and is the
   exact same executor called by production task wiring and the harness?
3. Does every effect have a closed typed request, correlation key, success,
   and typed-error completion?
4. Can a stale completion change state? If not, is the rejection test present?
5. Does each operation touch only the selected scenario adapter and preserve
   fixture/overlay/trace safety?
6. Are virtual ticks, receipt sequences, effect order, state projection, and
   configured error assertions exact rather than time-based?
7. Are all secrets absent from `Debug`, snapshots, effect records, traces,
   diagnostics, assertion messages, and failure output?
8. Does the test pass with Cargo's default parallelism and with the global
   context guard active?
9. Is every required test registered in the manifest, validation document, and
   traceability matrix, and does `ui-assert-tests` find it once?
10. Is it clear in the PR description whether the change proves backend,
    application-workflow, view wiring, or accessibility behaviour?

## 16. Completion criteria

V2 is complete only when all are true:

- All W0–W6 gates pass on a clean checkout.
- Every named test in section 10 is registered and passes under default Cargo
  parallelism.
- The target workflow paths no longer call the global selected operations
  context; the guard test proves this.
- At least two scenario harnesses can execute concurrently with separate
  fixtures and no state/trace cross-talk.
- Each fixture mutation test proves fixture immutability, overlay isolation,
  exact virtual-time receipts, and trace redaction.
- The application uses the same reducers/effect executors as the tests; no
  parallel test-only workflow implementation exists.
- The existing backend, scenario-control, capability, and package tests remain
  green.
- A review explicitly records that no V2 result was used to claim widget,
  keyboard, AT-SPI, screenshot, or visual coverage.

## 17. Follow-on compatibility spike (not a V2 gate)

After V2 is complete, a separate, one-commit experiment may answer whether
the exact pinned libcosmic/iced source can expose an `iced_test` headless
`Program` without a window and without adding a second iced version. It must:

1. Pin `iced_test` to the exact same git revision/submodule graph as
   libcosmic.
2. Compile one isolated view smoke test using a stable author-provided widget
   ID, not visible text or a coordinate.
3. Prove no production feature or test recorder is linked into the release
   binary.
4. Run no storage operation other than a scenario fixture.
5. Be deleted if it requires an upstream-only patch, incompatible renderer,
   coordinate target, unstable `.ice` file, or unreviewed toolkit API.

Success permits a new plan for a **view wiring smoke** layer. It does not
replace AT-SPI E2E, and failure has no effect on this V2 plan.

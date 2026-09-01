# UI scenario-backend specification

## Status and scope

This is the design for a new UI-test layer. It is not an implementation record.

The layer must make it possible to launch the normal COSMIC Storage UI against
an editable false storage system and exercise successful and failed UI flows.
It covers physical disks, partitions, filesystems, encryption, SMART, images,
Btrfs utility data, logical storage, network drives, usage scans, and the
app-owned file/action services needed to drive those screens safely.

It does not replace the disposable-fixture integration harness, emulate
UDisks/D-Bus, test kernel storage drivers, grant privileged access, or add a
production storage service.

## Baseline and problem statement

The production composition root in `src/operations/mod.rs` already owns a
`BackendRegistry` of `storage-contracts` trait objects. That is the correct
seam, but it is currently hidden by a process-global `shared()` constructor.
Many UI update paths and model constructors call `*Client::new()` and therefore
construct the real registry. `AppModel::init` has no supplied runtime
dependency.

There are storage-adjacent UI paths that bypass the registry:

- filesystem-tool probing, local usage discovery/scanning/deletion, and host
  path checks live in `src/operations/filesystems.rs`;
- image copy/progress and part of image creation use `std::fs` and
  `storage_sys` directly in `src/operations/image.rs` and
  `src/update/image/dialogs.rs`;
- desktop file chooser and URL/path launchers are not injectable.

The existing `tools/storage-testing::spec::LabSpec` is intentionally a
loop-image fixture description. It describes image sizes, partition layout,
and mounts for a destructive or nondestructive real-backend harness run. It
cannot describe dialogs, logical preflights, network schemas, errors, delayed
completions, or a mutable simulated system. Reusing it would couple UI tests to
fixture commands and make the harness a dependency of the desktop application,
which is prohibited.

The current GitHub CI runs Rust tests and the harness's safe profile. It has no
desktop session, scenario fixture gate, accessibility interaction, screenshot
baseline, or UI-test artifacts.

## Required architecture

~~~mermaid
flowchart LR
    Scenario[Versioned TOML scenario] --> Loader[test-backend]
    Loader --> Fake[ScenarioBackend: Arc RwLock RuntimeState]
    Real[Production factory: UDisks + local tools + rclone] --> Registry
    Fake --> Registry[BackendRegistry + app services]
    Registry --> Runtime[AppRuntime]
    Runtime --> App[Normal COSMIC AppModel]
    App --> A11y[AT-SPI UI runner]
    App --> Manual[Manual COSMIC session]

    Harness[storage-testing LabSpec] --> RealAdapters[Disposable loops + real adapters]
    RealAdapters --> HarnessReport[Ledgered run report]
~~~

The UI sees one `AppRuntime`, irrespective of mode. It never branches on a
"mock" boolean and no view/update code imports the scenario crate. Only the
composition root selects the factory.

### Application dependency injection

1. Move reusable application modules out of `main.rs` into `src/lib.rs`. Leave
   `src/main.rs` as argument parsing, logging/i18n setup, runtime construction,
   and `cosmic::app::run`. Keep implementation modules private unless a
   test-facing API needs to be public.
2. Replace `Application::Flags = ()` with an `AppRuntime`/`AppFlags` value
   containing `Arc<StorageOperations>` and injected app services. Production
   constructs this once via the synchronous `AppRuntime::production()` builder;
   tests construct it from explicit `RuntimeAdapters`. `AppRuntime` also retains the dedicated
   multi-thread Tokio bootstrap runtime that constructed production adapters;
   it remains alive for the whole COSMIC session. The flags must not contain a
   global factory.
3. Remove `SHARED_OPERATIONS` and `shared()`. Convert every `*Client::new()`
   call, model constructor, background task, and subscription to receive a
   clone of the current `Arc<StorageOperations>` (or a narrow client created
   from it). `with_operations` becomes the normal constructor, not a testing
   escape hatch. `UiDrive`, `UiVolume`, and `load_*` helpers accept the same
   runtime dependency.
4. Make subscriptions capture the app's runtime. Device events subscribe to
   `runtime.operations.registry.block` and image progress uses the injected
   image workflow. A background task retains the runtime selected at launch;
   it must never resolve a new backend later.
5. `main` parses arguments and validates/loading scenario files before calling
   `cosmic::app::run`. `AppRuntime::production()` creates the bootstrap runtime,
   uses it to construct production adapters, then returns the same retained
   runtime in the fully built flags; an error is rendered to stderr and exits
   before a window. The scenario factory is synchronous after file
   validation, so malformed scenarios also exit before a window. The
   `ui_test` library facade owns construction of a test `Core`, task draining,
   explicit virtual-clock advancement, message dispatch, and read-only state
   snapshots; root integration tests use it instead of private-module access or
   a desktop server.
6. Preserve current production behaviour and the single production UDisks
   connection. This is a dependency-flow refactor, not a second adapter path
   or a change in authorization semantics.

### Complete contract surface

All storage-facing UI behaviour must cross a trait boundary. Retain the
current `BlockStorageBackend`, `BtrfsBackend`, `LogicalTopologySource`,
`LogicalOperations`, and `NetworkDriveBackend` contracts. Add the missing
narrow interfaces below to `storage-contracts`:

| New interface | Owns | Production implementation | Scenario implementation |
| --- | --- | --- | --- |
| `FilesystemToolDiscovery` | `list_filesystem_tools` | current feature/`which` discovery | scenario-declared availability |
| `UsageOperations` | selected-mount validation, scan result/progress, delete result | current `storage_sys::usage` policy and file operations | declared scan snapshots, progress script, in-memory delete effects |
| `ImageWorkflowOperations` | create image, start backup/restore, status, wait, cancel, forget | current file/FD copy manager | synthetic operation state/progress and declared result |
| `DesktopServices` (app-local semantics) | file selection, reveal/open path, open URL | current COSMIC chooser and `open` behaviour | recorded/no-op or scenario-selected response |
| `ScenarioControl` (scenario-only) | explicit virtual-clock advance, overlay reload, diagnostics | not constructed | actor commands and read-only snapshot |

`DesktopServices` and `RuntimeAdapters` are declared in `storage-contracts`
as portable runtime contracts, but `DesktopServices` is not a storage backend
trait and is not a supertrait of `BlockStorageBackend`. This keeps its
app-local semantics without making `test-backend` depend on the application
crate. `RuntimeAdapters` is the complete adapter set from which the root crate
builds `BackendRegistry`; `test-backend` returns it without importing root
types.

`ScenarioControl` is a test-runtime control plane, not a storage operation. It
does not receive a `ScenarioOperation`, cannot be selected by a behaviour rule,
and has no production implementation. Its three methods have fixed semantics
and are covered by dedicated lifecycle/control tests rather than the generated
storage-method inventory.

`ImageDeviceOperations` is refactored to the typed, scenario-safe image
attachment boundary. Its former `OwnedFd` methods become private implementation
details of the production `ImageWorkflowOperations` adapter and are removed
from `BlockStorageBackend`. The higher-level image workflow means UI code never
copies a host file itself. `UsageOperations` similarly owns the current
protected-path and current-user policies instead of allowing the UI facade to
access the host. `OperationId` is migrated in Phase 1 before `test-backend`
needs it. The later public-trait removal, UDisks implementation change, and
application call-site migration are one atomic compatibility change; no
intermediate commit removes a method that a checked-in application or production
adapter still implements.

`StorageOperations` becomes a thin registry of these interfaces and contains
no host-specific mutable manager or filesystem-tool cache. Production adapters
may still use `storage_sys` internally; application and scenario-backend code
may not reach it directly. All public operation inputs and outputs remain typed
domain values, never commands, paths embedded in an action string, or untyped
JSON.

`DesktopServices` is intentionally separate: it prevents a UI scenario from
opening a real browser, file picker, or path while avoiding the false claim
that desktop integrations are storage backend operations.

### Frozen portable runtime API (Phase 0a)

This subsection is the Phase-0a API lock. Phase 1 implements these names and
signatures verbatim; changing one is a design change that reopens Phase 0a.
All listed values are owned, `Send + Sync`, and have an explicitly tagged
Serde form. String fields named `device`, `mount`, `id`, `label`, or `url` are
validated domain values, not capabilities.

```rust
pub trait FilesystemToolDiscovery: Send + Sync {
    async fn list_filesystem_tools(&self) -> Result<Vec<FilesystemToolInfo>, StorageError>;
}

pub trait UsageOperations: Send + Sync {
    async fn list_mounts(&self) -> Result<Vec<UsageMount>, StorageError>;
    async fn authorize_show_all_files(&self) -> Result<bool, StorageError>;
    async fn start_scan(&self, request: UsageScanRequest) -> Result<OperationId, StorageError>;
    async fn scan_status(&self, id: &OperationId) -> Result<UsageScanStatus, StorageError>;
    async fn wait_for_scan(&self, id: &OperationId) -> Result<UsageScanResult, StorageError>;
    async fn delete_usage_files(&self, request: UsageDeleteRequest)
        -> Result<UsageDeleteResult, StorageError>;
}

pub trait ImageWorkflowOperations: Send + Sync {
    async fn create_image(&self, request: CreateImageRequest)
        -> Result<ImageAssetRef, StorageError>;
    async fn attach_image(&self, asset: &ImageAssetRef)
        -> Result<ImageAttachment, StorageError>;
    async fn start_copy(&self, request: ImageCopyRequest) -> Result<OperationId, StorageError>;
    async fn copy_status(&self, id: &OperationId) -> Result<ImageCopyStatus, StorageError>;
    async fn wait_for_copy(&self, id: &OperationId) -> Result<ImageCopyResult, StorageError>;
    async fn cancel_copy(&self, id: &OperationId) -> Result<(), StorageError>;
    async fn forget_copy(&self, id: &OperationId) -> Result<(), StorageError>;
}

pub trait DesktopServices: Send + Sync {
    async fn select_image(&self, request: ImageSelectionRequest)
        -> Result<ImageSelection, StorageError>;
    async fn reveal(&self, target: DisplayReference) -> Result<(), StorageError>;
    async fn open_url(&self, url: SafeUrl) -> Result<(), StorageError>;
}

pub trait ScenarioControl: Send + Sync {
    async fn advance_to(&self, tick: u64)
        -> Result<ScenarioControlReceipt, StorageError>;
    async fn reload_overlay(&self)
        -> Result<ScenarioReloadOutcome, StorageError>;
    async fn diagnostics(&self) -> Result<ScenarioDiagnostics, StorageError>;
}
```

`ImageAssetRef` is an opaque, validated identifier (`asset:<safe-id>`). A
production desktop service and production image workflow share a private
asset-token store owned by the composition root; only that store knows a host
path. A scenario implementation accepts only declared asset IDs. `SafeUrl`
accepts only an absolute `https` URL or a documented project URL scheme, and
`DisplayReference` is a display-only `asset:<id>`, `/dev/ui-*`, or `/mnt/ui-*`
reference. None exposes a `Path`, descriptor, command, or callback.

`OperationId` is a validated transparent string, not an opaque UUID. Its
constructors are `OperationId::production_random()` and
`OperationId::scenario(fixture_name, operation, sequence)`. The latter emits
exactly `<fixture-name>:<operation>:<sequence>` and is the only constructor
that `test-backend` may call; parsing rejects any other form in scenario state.
`production_random()` emits exactly `prod:<uuid-lowercase>`; that random text
never enters a scenario fixture or trace.

`RuntimeAdapters` has exactly these fields: `block`, optional `btrfs`,
`network: Vec<NetworkAdapterRegistration>`,
`logical_sources: [LogicalSourceRegistration; 2]`, `logical_operations`,
`filesystem_tools`, `usage`, `images`, and `desktop`.
`NetworkAdapterRegistration` contains its typed ID, availability, and optional
backend; `LogicalSourceRegistration` contains the fixed source enum,
availability, and source object. The logical registrations are always
`[Udisks, LocalTools]`. `RuntimeAdapters::validate()` is defined in
`storage-contracts` and rejects duplicate network IDs or a wrong logical order;
`StorageOperations::from_adapters` calls it before consuming the value. This
makes registration-order validation testable in the contracts crate.

The scenario clock/control is deliberately outside `RuntimeAdapters`.
`test-backend` returns a `ScenarioRuntime { adapters, control }`, where
`control: Arc<dyn ScenarioControl>` has only `advance_to`, `reload_overlay`,
and read-only `diagnostics` methods. `AppRuntime` retains this control only in
scenario mode. The `ui_test` facade calls it directly; the feature-gated
scenario diagnostic page exposes the two mutating commands as semantic actions
and renders the diagnostics read-only for the black-box runner. The normal
desktop entry point has neither the control nor its accessibility nodes. The
root feature `test-backend` implies `ui-test`;
Phase-3 integration tests enable `ui-test` directly without adding scenario
launch support.

`ScenarioRuntime` is an owned, non-cloneable factory result. Its factory
validates the fixture and optional overlay synchronously, constructs the
unstarted actor handle, calls `RuntimeAdapters::validate()`, and returns no
production adapter. `AppRuntime::from_scenario` clones only the adapter arcs
needed to retain `desktop`, consumes the adapters once, and stores the paired
control. No downcast from `StorageOperations` to a scenario type is permitted.
`advance_to` rejects a lower tick with `InvalidInput` and returns its actor
sequence only after all due work has materialized. `reload_overlay` returns
`Unsupported` without an overlay, and otherwise returns an `applied` or
`rejected` outcome containing its actor sequence; invalid input never replaces
the prior state. `diagnostics` is a snapshot at its own actor sequence and
never changes generation.

`AppRuntime` is exactly `operations: Arc<StorageOperations>`,
`desktop: Arc<dyn DesktopServices>`,
`bootstrap: Option<Arc<tokio::runtime::Runtime>>`, and
`scenario_control: Option<Arc<dyn ScenarioControl>>`. Production construction
creates one multi-thread runtime, uses it to construct the real adapters, and
retains it as `Some` until the COSMIC process exits. `AppRuntime::production()`
is synchronous: it creates the runtime, calls a private
`build_production_adapters` with `Runtime::block_on`, clones `desktop` before
passing adapters to `StorageOperations::from_adapters`, and then stores the
runtime. `AppRuntime::from_adapters` performs the same desktop clone for test
adapters and sets `bootstrap` to `None`. `AppRuntime::from_scenario` accepts a
validated `ScenarioRuntime` and stores its control. Scenario and in-process
futures run on COSMIC's executor. `Application::Flags` is this fully-built
`AppRuntime`, not a factory, command-line value, or global cell.

| DTO | Required fields / legal tags |
| --- | --- |
| `UsageMount` | `id`, `mount`, `total_bytes`, `free_bytes`, `selectable` |
| `UsageScanRequest` | `mount_ids: [safe-id]`, `top_files: u32`, `show_all_files: bool`, `parallelism: low\|balanced\|high` |
| `UsageScanStatus` | `operation_id`, `phase`, `bytes_processed`, `bytes_total`, `percent`, `terminal: pending\|completed\|cancelled\|failed` |
| `UsageDeleteRequest` | `mount_id`, `file_ids: [safe-id]`; adapters resolve IDs, never a UI path |
| `CreateImageRequest` | `asset: ImageAssetRef`, `size_bytes` |
| `ImageCopyRequest` | `kind: backup\|restore`, `device: BlockDeviceRef`, `asset: ImageAssetRef` |
| `ImageCopyStatus` | the `UsageScanStatus` progress/terminal fields |
| `ImageCopyResult` | `operation_id`, `kind`, `terminal: completed\|cancelled`, optional `attachment` |
| `ImageAttachment` | `asset`, `device: BlockDeviceRef`, `display_path: /dev/ui-*` in scenario mode |
| `ImageSelectionRequest` / `ImageSelection` | `purpose: create\|attach\|backup\|restore`; `selected(asset)` or `cancelled` |

`DesktopServices::select_image` reserves and returns an opaque asset token.
For `create` and `backup` it reserves a destination selected with save
semantics; for `attach` and `restore` it reserves an existing regular file
selected with open semantics. The production desktop service stores the private
path in the composition-root token store. `create_image` consumes a reserved
create token and leaves it usable as an image asset on success; a failed create
invalidates it. `attach_image` and `start_copy` consume only purpose-compatible
tokens and retain them until `forget_copy` or process exit. Scenario tokens are
declared assets with the same state machine. This gives the UI one typed flow
without leaking host paths across a contract.

### Frozen workflow value encoding

The API table above is not shorthand for an implementation-defined Serde
layout. Phase 1 defines the following closed values verbatim in
`storage-types`, and JSON/TOML round-trip tests assert these tag and field
names:

```text
OperationId        = validated transparent string
                    production: `prod:<uuid-lowercase>`
                    scenario: `<fixture-name>:<operation>:<sequence>`
ImageAssetRef      = validated transparent `asset:<safe-id>`
WorkflowPhase      = queued | running | finalizing
WorkflowTerminal   = pending | completed | cancelled | failed
ImageSelection     = { tag = "selected", asset = ImageAssetRef }
                   | { tag = "cancelled" }
UsageScanStatus    = { operation_id, phase: WorkflowPhase,
                       bytes_processed: u64, bytes_total: Option<u64>,
                       percent: Option<u8>, terminal: WorkflowTerminal }
ImageCopyStatus    = UsageScanStatus fields
ScenarioControlReceipt = { sequence: u64, virtual_tick: u64, generation: u64 }
ScenarioReloadOutcome = { tag = "applied", receipt: ScenarioControlReceipt }
                      | { tag = "rejected", receipt: ScenarioControlReceipt,
                          error: StorageError }
ScenarioDiagnostics = { virtual_tick: u64, generation: u64,
                        last_sequence: u64, last_reload_error: Option<StorageError> }
```

`percent`, when present, is `0..=100`; it is present only when
`bytes_total = Some(total)` and `total > 0`, and equals
`min(100, floor(bytes_processed * 100 / total))`. A pending status has no
terminal result. A failed terminal result is returned as `StorageError` from
`wait_*`, never as a successful `*Result`. `UsageMount.mount` is a validated
display reference, not an OS path capability. `ImageSelectionRequest` has
exactly `purpose: create|attach|backup|restore`; its purpose selects the save
or open semantics above. `CreateImageRequest.asset` must have purpose `create`.
`ImageCopyRequest.asset` must have purpose `backup` or `restore` matching its
kind. No value has an implicit default except `percent = None` and
`bytes_total = None`.

## Scenario backend

### Packaging and safety boundary

Add this non-published workspace member:

~~~text
crates/test-backend/
  src/lib.rs                 # loader, validation, factory
  src/schema.rs              # versioned TOML DTOs
  src/state.rs               # runtime mutation/state queries
  src/block.rs               # all BlockStorageBackend component traits
  src/btrfs.rs
  src/logical.rs
  src/network.rs
  src/usage.rs
  src/image.rs
  tests/{schema,contract,mutation}.rs
~~~

It may depend only on typed contracts/types, Serde/TOML, async/runtime support,
and narrowly scoped utilities for deterministic state and streams. It must not
depend on the root application crate, `storage-udisks`, `storage-sys`,
`storage-btrfs`, shell execution, D-Bus, `which`, host mount discovery, or
fixture commands. The only host I/O code is the explicit `ScenarioStore` facade
for the selected fixture, overlay, trace, and runner artifacts; no state-derived
value reaches that facade. Set `publish = false`.

This boundary is enforced rather than inferred from a dependency tree.
`ScenarioStore` owns private `FixturePath`, `OverlayPath`, `TracePath`, and
`ArtifactRoot` capability types. Only argument parsing and the runner construct
them from user-provided paths; runtime state, fixture DTOs, and operation
requests cannot construct or convert to them. All `test-backend` filesystem I/O
lives in `store.rs`; the crate has `#![deny(unsafe_code)]`, and the planning
checker rejects direct uses of `std::fs`, `tokio::fs`, `std::process`,
`tokio::process`, `Command`, `OpenOptions`, D-Bus crates, and network-socket
crates outside `store.rs` and its explicitly listed test fixtures. The runtime
tests use a recording store with hostile state values and assert that every I/O
attempt used one of the startup capabilities. Cargo-metadata checks remain a
second, narrower dependency check; neither check is treated as a proof by
itself.

The application has an optional `test-backend` Cargo feature whose only new
dependency is this crate. Release/package recipes must invoke the root package
without this feature and a CI artifact inspection must prove the resulting
binary has neither scenario CLI help nor scenario UI resources. In a build
without the feature,
`--backend scenario` exits before opening a window with a clear rebuild
instruction. In a build with it, malformed or missing scenarios exit before
constructing any production adapter. A scenario launch never falls back to
UDisks, rclone, local Btrfs, or a host storage target.

### Runtime model

`ScenarioBackend` owns one FIFO `ScenarioCommand` actor, an
`Arc<RwLock<ScenarioRuntime>>` used only to publish completed snapshots, and a
virtual `ScenarioClock`. The runtime contains validated typed initial state,
mutable current state, monotonic topology generation, per-subscriber ordered
event queues, in-flight synthetic image/usage operations, and an append-only
typed `ScenarioTrace`.

Construction creates the bounded command channel but does not spawn a task.
The first async backend/control call made on COSMIC's Tokio executor starts one
actor under a `start_once` lock, then enqueues its command. A call made without
an executor fails with typed `Unavailable`; it never creates a private fallback
runtime. Shutdown enqueues a terminal command, rejects new calls, resolves
waiters with `Unavailable`, closes subscriber queues, and joins the actor.
Dropping a subscriber enqueues an ordered unsubscribe command; it cannot mutate
topology or generation. Channel capacity is 256 commands; back-pressure awaits
capacity and is represented by command enqueue order in the trace.

For concurrently initiated calls, the deterministic order is the order in
which they are successfully enqueued; the assigned actor sequence is the
observable tie-breaker. Tests that require a particular race must enqueue
commands through `ScenarioControl` in that order and assert the recorded
sequences. The specification makes no claim that unsynchronised host task
scheduling has a repeatable source order.

Every public backend call—including metadata reads, discovery reads, status
reads, event-stream subscription, clock advancement, and overlay reload—is a
command. The actor assigns a monotonically increasing sequence and executes it
after all earlier commands. A read returns the immutable snapshot captured at
its own sequence and never increments generation. A subscription command first
registers its queue and then returns a stream; it receives only events emitted
by later sequences. The `RwLock` never decides ordering, and no backend future
may mutate state outside the actor.

`advance_to(tick)` is also an actor command. It rejects a lower tick, moves the
clock once, and materializes all due synthetic steps in ascending
`(due_tick, operation_start_sequence, schedule_index)` order before the next
command begins. Synthetic work does not progress autonomously. Therefore a
cancel enqueued before `advance_to` that would complete the operation wins; a
cancel enqueued after that clock command observes the documented terminal
result. A `wait_*` command registers a waiter at its sequence without blocking
the actor and resolves only when a later synthetic terminal step is materialized.

Every implemented method must perform this deterministic sequence:

1. Record a redacted typed request in the trace.
2. Select an exact fault/latency rule, if configured, using the closed rule
   comparator in [scenario-schema-v1.md](scenario-schema-v1.md).
3. Return the configured `StorageError` or perform the documented in-memory
   transition.
4. Increment the appropriate generation exactly once and publish declared
   events after a successful topology-changing transition.
5. Record the typed response or error.

For a read, steps 3–4 mean “capture snapshot and emit no topology event.” For a
subscription, the response records the start sequence but never serializes the
stream handle. For a reload, the command is queued after prior work; it cancels
affected synthetic operations, applies the validated snapshot, increments
generation once, and emits the explicitly sorted diff. The sort key is
`(device_path, event_kind)`, where `Removed < Added`; duplicate events are a
schema error.

No method may silently be a successful no-op. A method not configured for a
fixture follows its documented modelled default or returns
`StorageErrorKind::Unsupported` with `Scenario does not configure
<operation>`. There is no `todo!`, panic, host fallback, arbitrary command, or
arbitrary callback escape hatch. The backend validates `LogicalAction` and the
same type-level invariants before applying a logical transition. Synthetic
operation IDs, trace timestamps, and progress are sequence/tick-derived, never
UUID- or wall-clock-derived.

The implementation covers every method of these existing contracts:

| Contract family | Required scenario behaviour |
| --- | --- |
| metadata, discovery, events, drive, partition, filesystem, encryption, typed image attachment | read/mutate corresponding typed state; derive display paths/IDs only from scenario state; provide an ordered stream that remains pending until a change or shutdown, then closes |
| `BtrfsOperations` | query/mutate per-mount typed subvolume/default/usage/deleted state without `btrfsutil` |
| logical topology and operations | return declared source topology/candidate captures/preflights; validate confirmation keys and apply configured action transitions or errors |
| network drives | read/update per-backend configs and mount state using declared provider schemas/capabilities |
| usage/image/tool interfaces | drive simulated progress/results declared in the scenario; never inspect or create a host path |

The scenario factory returns `RuntimeAdapters` containing the scenario as the
one block backend, Btrfs backend, logical source/executor, and every declared
network backend, plus the test `DesktopServices` recorder. It always registers
logical sources in the production order `[Udisks, LocalTools]`; a fixture that
omits a source yields an explicitly unavailable empty source rather than
changing registration order.

### Scenario file format

The canonical format is TOML and is defined by
[scenario-schema-v1.md](scenario-schema-v1.md), not by Rust's private
serialization layout. It defines the exact root, DTO, safe identifier, rule,
overlay, event, virtual-time, and trace forms. The loader uses dedicated DTOs
with `deny_unknown_fields` at every stable level, validates all cross references
before runtime construction, and converts them into domain values. A schema
version is never inferred.

Fixtures live recursively under `tests/ui/scenarios/**/*.toml`; ad-hoc manual
scenarios may live anywhere. The first non-comment key of every file is
`schema_version = 1`. The `ui-scenario print-schema --version 1` output and its
checked-in descriptor are part of the compatibility contract. A fixture or
behaviour rule cannot contain `OwnedFd`, a real process, a host-path image,
raw secret, native object path, arbitrary JSON, command line, or callback.

### Editable use

A scenario has an immutable fixture file and, optionally, a writable state
overlay. The production-safe default is read-only and starts from the fixture
on every launch. The developer flow is:

~~~text
cargo run --features test-backend -- \
  --backend scenario \
  --scenario tests/ui/scenarios/physical/partition-format.toml \
  --scenario-state /tmp/storage-ui-state.toml \
  --watch-scenario
~~~

`--scenario-state` is atomically rewritten after successful simulated
mutations and includes schema version, immutable-fixture hash, typed state, and
last sequence; it never rewrites a repository fixture. An overlay-backed
transition is transactional: the actor applies it to a clone, writes and fsyncs
the new overlay, and only then publishes the clone, generation, events, and a
success response. A write failure leaves runtime state, generation, events, and
the visible response unchanged and returns typed `Other`. In read-only mode the
same transition is committed in memory without a write. A restarted overlay
sets the next actor sequence to `last_sequence + 1` after validating its exact
fixture hash.

`--watch-scenario` watches this overlay only, not the immutable fixture. File
notifications are merely a request to enqueue `reload_overlay`; duplicate and
self-write notifications are coalesced by exact overlay bytes plus acknowledged
sequence. The deterministic test path never waits for a notification: after an
atomic external replacement it invokes the semantic `Reload scenario` control,
waits for its trace sequence, and then asserts the result. The atomic
replacement, self-write acknowledgement, invalid/stale overlay handling,
in-flight-operation policy, and sorted device-diff algorithm are defined in
[scenario-schema-v1.md](scenario-schema-v1.md). Automated runs leave watching
and writeback off, copy a fixture into the run-artifact directory, and assert
against that copy and its trace.

Provide a `ui-scenario` utility with `validate`, `materialize-state`, and
`print-schema` subcommands. It is the supported way to validate an editable
fixture before launching the app; it must not be necessary for ordinary TOML
editing.

### Launch and observability

The feature-gated binary accepts:

~~~text
--backend real|scenario              # real is the default
--scenario <path>                    # required for scenario
--scenario-state <path>              # writable overlay, opt-in
--watch-scenario                      # opt-in live reload of --scenario-state
--scenario-trace <path>              # JSON trace written atomically on exit
~~~

Scenario mode adds a clearly non-production window/subtitle indicator such as
`Test scenario: <name>` and a safe local diagnostic page showing schema
version, source path, last reload error, generation, and a redacted trace
summary. It also exposes feature-gated, named semantic actions `Advance
scenario clock` and `Reload scenario`; the former accepts an explicit unsigned
tick and the latter returns the queued sequence/result. They are the only
black-box control mechanism and are absent from normal builds. The indicator
prevents a user confusing simulated device paths with host devices.

Argument rules are fail-closed: `--scenario` is required only for
`--backend scenario`; `--scenario-state` and `--watch-scenario` require that
mode, and watching requires an overlay; real mode rejects scenario-only flags.
`--scenario-trace` is allowed only in scenario mode. These checks run before
any production factory is considered.

The UI test runner obtains structured artifacts through files, not a new
application socket: copied scenario, optional state overlay, scenario trace,
application log, accessibility-tree dump, screenshots, and image diffs.

## Test strategy

### Contract and scenario tests

Add unit/integration tests in `test-backend` for schema, runtime,
and every trait family. The `storage-contracts` contract-surface manifest is
the one source for trait method identifiers and generated `ScenarioOperation`
values. It generates the public trait method declarations as well as their
metadata, test-backend mapping requirement, schema operation tag, and trace
operation tag. Existing handwritten trait modules are migrated to generated
declarations in Phase 1; they may not be independently extended. Adding a
method therefore requires its modelled default, validation case, fault case,
trace-redaction rule, and [traceability row](traceability.md) in the same
change. Removing a public method retains its versioned operation as an explicit
`native_only` tombstone until the schema version retires it. The completeness
test compares active generated sets plus declared tombstones; independently
maintained string lists are prohibited.

Required fixture classes include:

- empty/permission/unavailable startup states;
- internal, removable, loop, optical, and failed-SMART disks;
- GPT/MBR/empty tables, nested volumes, filesystem format/mount/busy unmount,
  mount settings, and filesystem-tool unavailable;
- LUKS unlock/lock/options/error; image create/attach/backup/restore/progress/
  cancellation; and usage progress/delete failure;
- Btrfs utility data plus logical Btrfs, LVM, and MD entities, preflights,
  blocked reasons, stale confirmation conflict, and topology events;
- network provider form, validation, connection test, mount status, and
  unavailable backend.

Test state transitions, virtual-time progression, queue ordering, and event
ordering rather than merely checking that a method returns `Ok(())`. Dependency
tests prove `test-backend` has no prohibited production dependencies; factory
tests separately prove scenario selection does not construct a production
adapter. They do not make the false claim that the feature-gated root binary is
unlinked from its normal production dependencies.

### In-process UI tests

After dependency injection, add root integration tests that instantiate the
real `AppModel` with `AppRuntime::from_scenario(...)` (or explicit
`AppRuntime::from_adapters(...)` for a narrow non-scenario fake), use the public `ui_test`
facade to drive typed messages and drain controlled tasks, explicitly advance
the scenario clock, and assert state/dialog/view-model snapshots. They use
scenario fixtures through the real operations facade, not hand-written mock
clients.

Cover startup, incremental loading, selected-volume navigation, every dialog
submit/cancel/validation branch, operation pending/progress/completion,
configured errors, device events, logical generation/preflight freshness,
network CRUD/mount flow, and regression of the normal real-runtime factory.
Keep pure state tests close to their state module; integration tests must not
use `#[path]` copies of application source because those bypass composition.

### Black-box accessibility and visual tests

Add `tools/ui-testing/` with a scenario case runner. Cases reference a
scenario by relative path and declare stable accessibility-name/role actions,
expected accessible states/text, and screenshot checkpoints. Use the app's
AccessKit/AT-SPI support and a Python `pyatspi` driver; do not use coordinate
clicks as the primary selector. Add stable localized accessibility names and
descriptions to controls that currently rely only on icon tooltips.

Each case runs the actual feature-gated binary in an isolated Wayland session,
waits for a semantic ready condition, invokes AT-SPI actions/types text, and
captures the app window by accessible bounds. Screenshot comparison uses a
fixed viewport, scale, locale, font set, light/dark theme fixture, and the
following exact comparison rule: normalize to sRGB RGBA, crop with floor(left,
top)/ceil(right,bottom) accessible bounds, permit an absolute per-channel delta
of at most 2 for no more than 0.05% of pixels, and apply no masks. Golden-image
updates require an explicit `just ui-e2e-update` command and review of generated
diffs; CI never accepts a new baseline automatically.

The runner advances delayed scenario work only through the named `Advance
scenario clock` accessibility action and waits for the returned trace sequence;
it never infers completion from elapsed time. It invokes `Reload scenario` after
an overlay replacement for the same reason. Accessibility bounds are logical
output coordinates with origin `(0, 0)` at the single Weston output; the runner
converts each edge to pixels by `floor(edge * scale)`/`ceil(edge * scale)`
before cropping. The locked launch environment sets `HOME`, `XDG_CONFIG_HOME`,
`XDG_CACHE_HOME`, `LANG`, `LC_ALL`, `TZ`, Fontconfig configuration, and the
COSMIC theme fixture to checked-in values, so no host or prior-user setting can
affect a golden.

`tools/ui-testing/Containerfile` is the sole E2E environment definition. It
uses a digest-pinned Debian base, a checked-in package/font version lock, the
Weston headless backend on a private socket, `dbus-run-session`, the AT-SPI bus,
software GL (`LIBGL_ALWAYS_SOFTWARE=1`), and a checked-in screenshot utility.
The case-manifest schema fixes locale, theme, viewport, scale, scenario, named
semantic-ready selector, semantic actions/assertions, and checkpoints. It
rejects coordinate-only interactions, missing/duplicate IDs, unknown selector
roles, and a checkpoint without a golden. `just ui-e2e` always runs the image
identified by the checked-in digest; it never selects a host compositor, font,
config directory, locale, or theme.

The initial required black-box cases are:

1. physical disk list → partition page → create/format success and refresh;
2. busy unmount → blocking-process dialog → cancellation/error remains
   actionable;
3. encrypted volume unlock error and success state;
4. logical Btrfs/LVM/MD selection, blocked-action explanation, preflight,
   confirmation, and post-action event refresh;
5. network remote create/validation/test/mount status;
6. image progress/cancel and usage-scan progress/failed delete;
7. keyboard-only dialog traversal and accessible name/disabled-reason checks;
8. scenario reload while UI is open, proving a valid edit refreshes and an
   invalid edit preserves prior state.

## Harness relationship

The layers share `storage-types`, `storage-contracts`, action validation, and
the application's operations facade. They do not share a fixture schema or
execution engine:

| Concern | UI scenario backend | `storage-testing` harness |
| --- | --- | --- |
| System under test | normal UI against simulated traits | real UDisks/local adapters against disposable loops |
| State source | TOML typed application state + behaviour | `LabSpec` loop images/partitions/mounts |
| Mutation | in-memory/optional overlay only | ledger-validated fixture commands plus typed real operations |
| Safety | cannot open D-Bus, run commands, or touch host storage | fail-closed destructive profile in dedicated VM |
| CI evidence | trace, a11y dump, screenshots/diffs | fixture ledger and run report |

Do not make `storage-testing` a dependency of the app or scenario backend, and
do not make the scenario backend a dependency of the destructive harness. The
harness may record a scenario filename/hash in a future combined report, or a
developer may manually derive a visual fixture from a known lab topology, but
there is no automatic schema conversion and no claim that a scenario proves
native backend correctness.

## CI and developer commands

Retain current Rust and safe-harness jobs. Add these independent, required
pull-request checks once bootstrap fixtures are committed:

| Job | Environment | Required work |
| --- | --- | --- |
| `ui-scenario-contract` | ordinary Ubuntu runner, no display | format/check scenario crate; schema/trait-completeness/mutation tests; application injection/in-process UI tests; validate every checked-in scenario |
| `ui-e2e` | pinned container on Ubuntu runner with software rendering, nested Wayland compositor, D-Bus, AT-SPI, fixed fonts/theme | build with `test-backend`; execute accessibility/visual cases; upload artifacts; fail on semantic or approved pixel-baseline mismatch |
| `harness-nondestructive` | existing Ubuntu job | continue `just harness-nondestructive`; it remains a real-adapter check |

The Phase-0a environment-lock format is defined in
[e2e-environment-v1.md](e2e-environment-v1.md). Phase 8 materializes the
digest-pinned UI image, package/font lock, and Weston command in the repository
before creating any golden. The runner creates a private `XDG_RUNTIME_DIR` and
`dbus-run-session`, starts the AT-SPI registry and nested compositor, launches
the app with copied scenario/artifact directory, and always tears the session
down. It must not expose the host system bus or grant privileged devices.
Software rendering is explicit so visual baselines do not depend on runner GPUs.

Add developer recipes:

~~~text
just ui-scenario-check       # validate scenarios and run scenario tests
just ui-test scenario=<file> # local test-backend desktop launch
just ui-e2e                  # containerized a11y/visual suite
just ui-e2e-update           # deliberate local golden refresh after review
~~~

`ci.yml` uploads `ui-artifacts/` on success and failure: run-case manifest,
copied scenario/state, trace, app log, accessibility dump, actual screenshots,
expected images, and diffs. It publishes JUnit/step summaries with test case
names and never suppresses a failed UI case as `Skipped`. Flaky failures are
fixed through deterministic readiness/fixture behaviour, not retries that hide
a failure.

Workflow YAML cannot make a check required. Phase 9 includes a repository
administrator-owned branch-protection change that requires `ui-scenario-contract`
and `ui-e2e`; its owner and evidence are recorded in
[phase-record.md](phase-record.md). Deliberately failing semantic and pixel
runs are a maintainer acceptance exercise on an authorized test PR, not a
command an ordinary implementation branch must be able to push.

## Acceptance criteria

The work is complete only when all statements are true:

- a developer can edit a validated TOML scenario, launch the normal app in
  clearly marked scenario mode, and observe only simulated state changes;
- every app storage path and all current/new public storage trait methods are
  served by the selected runtime, with no global real-backend construction or
  state-derived host storage operation in UI/application code;
- scenario mode is non-production, feature-gated, host-safe, traceable, and
  cannot silently fall back to a real backend;
- in-process and black-box tests drive the real UI against scenario fixtures,
  including error, async, refresh, keyboard, and accessibility states;
- CI gates scenario contract/injection coverage and the isolated Wayland
  accessibility/visual suite while retaining the real-backend safe harness;
- the destructive harness remains isolated and its `LabSpec`, ledger, profile
  gates, and real-adapter purpose are unchanged.

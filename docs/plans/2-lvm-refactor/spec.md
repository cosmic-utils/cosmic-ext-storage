# LVM Refactor Specification

**Status:** Planned
**Authority:** This specification defines the intended merged behaviour. If an
old `079-lvm-support` implementation conflicts with it, this document and the
current `main` architecture govern.

## Outcome

Add a first-class `Logical` topology to the in-process application. Logical
roots represent LVM volume groups and logical volumes, MD RAID arrays, and
multi-device Btrfs filesystems. Physical disks retain their own tree; they link
to logical membership but never own cross-device aggregates.

The UI preserves every logical action, dialog, topology control, status update,
and sidebar behaviour from `079-lvm-support`. A permission failure returned by
UDisks2 is a typed user-visible result, not a reason to resurrect the removed
privileged service.

## Source-fidelity requirement

The target is a literal transplant of feature-branch additions wherever they
can compile and run in the current-main layout. Before a transplant, reconcile
the common ancestor, current-main, and feature changes under
`reconciliation.md`; an independently changed main range must not be
overwritten merely because the source branch also changed its pre-move path.
For feature-only ranges, copy the feature-branch logical controls, state,
update routing, views, dialog forms, sidebar behaviour, and harness files into
their mapped current-main locations before making adaptations. Preserve their
control hierarchy, labels, action availability, field validation,
message/state transitions, ordering, default values, confirmation/cancel
behaviour, and test scenarios exactly.

Permitted changes are limited to:

- mechanical path/module/import changes caused by the move from
  `storage-app/` to `src/` and libraries to `crates/`;
- replacing a removed `LogicalClient` or service signal call with the
  equivalent typed operations-facade call; and
- adaptation required by the upgraded current-main Rust, libcosmic, or UDisks
  API, when no source-compatible form exists.
- a listed identity/native-semantics compatibility conversion: resolving an old
  path/CSV form value to an opaque domain ID, retaining a source form whose
  native equivalent is unavailable as a disabled action with its specific
  reason, or moving a fixture-only cleanup command behind the constrained lab
  executor described below.
- a current-main asynchronous lifecycle guard—load generation or refresh
  coalescing—needed because the current app runs physical, network, and logical
  work concurrently. It must preserve the source-visible selection, ordering,
  defaults, and completion behaviour.
- a completed three-way reconciliation which retains a current-main range and
  names separate evidence for the retained main and feature behaviours.
- converting a source harness `Skipped` path into deterministic profile
  exclusion, Blocked, or Failed under the required-execution contract, while
  retaining the case, fixture lifecycle, and test intent.

A permitted adaptation must preserve the source's visible and state-machine
behaviour. It may not redesign a screen, consolidate/split a workflow, rename
or remove a message/state transition, alter validation/defaults/order, reduce
the harness, or substitute an approximately similar UI. A source form that
UDisks cannot represent is retained, including its default and validation
presentation, but its Apply control is disabled with the precise native
limitation; it is not silently reinterpreted as another operation. Every
non-verbatim line range must be recorded with its reason and equivalence
evidence in `source-fidelity.md`; an unrecorded divergence fails this
specification.

## Target dependency and call flow

~~~text
storage-types
      ^
storage-contracts
      ^
storage-udisks     storage-sys
      ^                  ^
      +---- application composition root ----+
                       |
                 src/operations/logical.rs
                       |
            state/messages/views/sidebar/detail
~~~

`storage-udisks` owns UDisks2-backed logical discovery, device identity
normalisation, and every logical mutation. `storage-sys` owns local read-only
command probing that enriches topology; it has no logical mutation API, does
not depend on UDisks, and never constructs a D-Bus connection. The application
composition root constructs and registers one UDisks logical source, one local
logical source, and one UDisks logical action executor. `BackendRegistry` owns
those contract objects separately from its `BlockStorageBackend`: logical
discovery has multiple sources, while mutation has exactly one native executor.
All remaining application code sees contracts only.

The operation façade merges discovery sources deterministically by stable ID:
the fixed registration order is UDisks then LocalTools; UDisks identity,
parentage, state, health, capability, and member data win when present; and the
local-tools source fills only absent optional display/usage fields and absent
metadata keys. It must not merge entities merely by display name or overwrite
an empty-but-present UDisks value. Local-only entities remain displayable with
every mutation blocked. Source status and entity output are deterministically
ordered and must be covered by fixtures.

## Determinism and operation lifetime

The logical domain exposes a `LogicalTopology` containing uniquely identified,
ordered entities and exactly one status for each registered source. Duplicate
entity IDs or source statuses are a construction error, not last-writer-wins
behaviour. Roots/children use case-sensitive name then stable ID order; UDisks
member order is retained and only missing local members may be appended in the
local parser's documented deterministic order.

The UDisks backend owns a monotonic object-manager epoch which changes only
after it applies an ObjectManager add/remove event. A block mutation reference
contains the observed epoch plus a versioned fingerprint. Each operation
performs local validation, resolves a snapshot, evaluates capability, then
re-resolves the target and every block reference in a fresh snapshot just
before the native call. Invalid input, missing targets, identity conflicts,
missing interfaces, and stale epochs use the fixed error precedence in the
implementation plan. An identity conflict always prevents a proxy call.

The application permits one pending logical action at a time. Every dispatched
action receives a monotonically increasing action generation; late progress or
completion messages from an older generation are ignored, including their
refresh and dialog-close effects. Logical-load generation independently guards
topology refreshes. This makes async behaviour independent of task scheduling
order without changing the source-visible forms, selection, or status flow.

## Domain and contract requirements

Create `crates/storage-types/src/logical.rs` and export it from `lib.rs`.
Keep the existing narrow `lvm.rs` structures source-compatible where possible,
but do not represent the same concept twice in UI-facing APIs.

The logical domain must include:

- newtypes for stable entity and member IDs, with a documented source and
  canonical form. A `BlockDeviceId` is an opaque live
  `block:<major>:<minor>` lookup key made from UDisks `Block.DeviceNumber`; it
  is not a stable authorization token or a D-Bus object path. A mutating
  `BlockDeviceRef` carries that key plus an opaque immutable fingerprint and
  observed object-manager generation. The adapter resolves and rechecks the
  complete reference in a fresh snapshot immediately before every operation;
  a recycled device number is `Conflict`, never an eligible target;
- entity kind, health/state, member role/state, capability, blocked-reason,
  and progress types; use enums where the values are finite;
- root/detail and member models with deterministic ordering;
- `LogicalTopology`, `LogicalSource`, and source-availability/status values;
  duplicate IDs/statuses are rejected at construction rather than silently
  merged;
- typed request types for LVM, MD RAID, and multi-device Btrfs actions. Inputs
  are validated names, device references, sizes, levels, and mount points—not
  JSON blobs, command strings, or raw option fragments;
- error/result values that fit the existing `StorageError` / `OperationError`
  mapping.

Add a `logical` trait module in `crates/storage-contracts`.

- `LogicalTopologySource` lists typed topology and reports source availability.
- `LogicalOperations` exposes every branch operation through typed requests:
  LVM VG/LV/PV lifecycle, MD RAID lifecycle/membership/sync, and Btrfs
  device/resize/label/default-subvolume management.
- `LogicalTopologySource` and `LogicalOperations` are independent contracts;
  they are not supertraits of `BlockStorageBackend`. `src/operations/logical.rs`
  is the only application module allowed to merge registered sources and route
  actions to the registered logical executor.

`UdisksBackend` implements the UDisks logical source and the logical executor
using its existing single `DiskManager` connection. It calls UDisks2's native
Manager.LVM2, VolumeGroup, LogicalVolume, Manager, MDRaid, and
Filesystem.BTRFS interfaces, using their Polkit flow. `storage-sys` implements
only the read-only local-tools source with an injected command runner. The
feature-branch project-service handler, proxy client, JSON transport, caller
identity, authorization annotations, and project D-Bus signal code are not
ported.

## Logical UI requirements

- `SidebarState` gets a distinct logical entity key and selection. The current
  `Logical` section must be populated from logical roots, not from `UiDrive`.
- Selecting a logical root renders a dedicated detail surface; do not reuse the
  disk header or partition-segment control. Reuse generic cards/buttons only.
- Detail renders overview, members, and operations. Member links navigate to a
  physical device when that device is currently known; stale links remain
  inert and readable. A stale device reference may never be submitted as an
  action.
- Mutations launch typed application tasks, display progress/result state, and
  schedule one logical/physical refresh only for the current action generation
  on completion. There is no project D-Bus topology signal or polling proxy.
- Port async sidebar loading only after this state/routing works. It must load
  through the shared operations context and preserve current selection/loading
  semantics.

## Scope and privilege gate

This merge preserves the complete `079-lvm-support` action matrix. Each action
is implemented by an UDisks2-native method under the desktop user's existing
UDisks2/Polkit authorization flow. The application and production crates must
not add a project helper, root process, project policy, `sudo`, or `pkexec`
fallback.

The separate host-only test lab is not a production privilege path. Its
fixture setup and cleanup may use only the audited, ledger-validated command
executor in `tools/storage-testing`, running in an explicitly disposable VM or
CI runner with the privileges supplied by that environment. It may never
elevate itself, target an unallocated device, or be called by application code.
Logical actions under test still go through typed UDisks contracts; direct
fixture commands are limited to allocation, teardown, and reset.

For destructive operations, the typed native call and its side effects are the
public contract. The implementation plan records the source command behaviour,
native `wipe`/`tear-down` choice, changed semantic where no exact native
equivalent exists, confirmation text, and fixture assertion. No action may be
implemented with an implicit native default that changes how member metadata,
physical-volume labels, or mounted configuration are treated. Preserve-only
typed policies require `wipe=false` for LVM/MD member removal and
`tear-down=false` for VG/LV/MD deletion; Btrfs member removal uses its empty
native options map. Destructive VG/LV/MD requests also carry an exact,
freshly-revalidated collateral scope; if dependent volumes or member devices
change after confirmation, the operation returns Conflict and the UI requires
the user to review the new scope.

Stratis, ZFS, dm-cache/dm-writecache, VDO, and bcache/bcachefs remain out of
scope. Update the root README with the final scope and prerequisites; do not
restore obsolete service installation instructions.

## Harness boundary

Recreate the historical `storage-testing` package as a non-published workspace
member at `tools/storage-testing/`, preserving its `harness`, `lab`, fixture,
ledger, and integration-suite entry points. Test cases call typed adapters and
operations directly—never a project D-Bus service—and destructive cases remain
explicitly opt-in disposable-fixture tests. The lab's constrained fixture
executor is the only test-only exception to the application's no-command
mutation rule. Release packaging and publishing continue to select only the
root application and five published libraries. The runner uses a static case
catalog and a versioned report: profiles exclude cases before selection; every
selected case reaches Passed, Failed, or Blocked; timeout is Failed; and
`--require-executed` fails unless every selected case Passed. There is no
success-like skipped result.

## Verification contract

The implementation provides durable named test targets for the logical domain,
contracts, native adapter, read-only local tools, application state, UI,
asynchronous sidebar, and harness runner. Those tests assert product behaviour
and safety invariants rather than branch/task completion. Every phase validates
that its named target and required behavioural test names exist before running
the target; an umbrella Cargo filter is not an acceptance gate because it can
select no tests. Each phase also records a manual trace from this specification
through the changed code and a normal plus failure/edge path in validation.md.
The full test-target inventory, commands, and phase-record schema are part of
the implementation plan and validation record.

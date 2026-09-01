# Implementation Plan

## Delivery order

### 0. Establish the executable contracts and test seams

Implement the normative [execution contracts in spec.md](spec.md#execution-contracts)
before changing a renderer or form. This is a code-bearing phase, not a
research task left to later commits.

- replace all renderer-visible generic entity/member fields and ambiguous
  zero-valued display fields with `LogicalEntityDetails`, `LogicalDisplay`,
  typed LVM VG/LV/PV, MD array/member, and Btrfs filesystem/member/subvolume
  details, `BtrfsRelativePath`, `BtrfsSubvolumeRowKey`, and
  `BtrfsSubvolumeRef`. Migrate the selected-subvolume/default/delete/snapshot
  `LogicalAction` payloads at the same boundary. `metadata` is diagnostic-only;
  no logical renderer/form/action resolver may read it;
- introduce `LogicalCandidateAnchor`, `LogicalLoadRequest`,
  `LogicalLoadResult`, `LogicalCandidateResolution`, `LogicalActionKind`,
  `LogicalPreflightRequestKey`, `LogicalPreflightKey`,
  `LogicalDeviceCandidate`, `LogicalInputConstraints`, `LogicalReviewData`,
  and `ConfirmedLogicalAction` with the exact ownership and freshness fields
  specified in `spec.md`. Extend the contracts so preflight and execution are
  explicit facade operations, not UI conventions;
- split UDisks collection from mapping. One captured
  `ResolvedLogicalSnapshot` must provide the epoch, blocks, logical objects,
  captured Btrfs method observations, canonical Btrfs groups, and
  comparator-selected internal primary target to discovery, candidate
  resolution, preflight, and execution. Make its pure mapper fixture-driven;
- implement the typed per-block fingerprint hierarchy: partition UUID bound to
  containing-drive WWN/serial, drive WWN/serial, then loop backing identity. A
  partition UUID alone, filesystem `IdUUID`/`IdType`, object path, device
  number, or display path must never create a Btrfs member ref. An
  identity-less item is readable but cannot be selected or mutated;
- add explicit candidate eligibility and constraint/profile providers. A
  ready device has a current strong ref, is not already a target member, and
  has no detected structured-data signature. The versioned MD profile catalog,
  LVM/Btrfs open bounds, and absent-source blocking behaviour are those in the
  spec; do not infer them in views;
- introduce an injectable completed-snapshot/fixture boundary so adapter tests
  do not need a live D-Bus ObjectManager. Fixtures must contain captured
  `GetSubvolumes`/`GetDefaultSubvolumeID` outcomes, assert the private primary
  proxy/native target selected by the executor, and prove the pure mapper makes
  no live native call rather than only comparing values in a constructed Rust
  action.

The phase gate is a compile-clean migration of all action consumers and named
domain/contract/adapter tests for every `LogicalEntityDetails` variant, display
unknowns, Btrfs member read-only state, refs, request/result keys, ready versus
blocked candidates, review variants, captured observations, and deterministic
mapping. No old string/path subvolume action constructor or metadata renderer
may remain after this phase.

### 1. Treat the approved mock as a composition reference

Use [mock.html](mock.html) as the acceptance reference for the Btrfs page's
single-page information order: existing app header, logical filesystem header,
Devices, then Subvolumes. It deliberately does not specify production CSS or
icons. Before adding a logical control, check the component map in
[spec.md](spec.md#component-icon-and-theme-contract) and reuse or extract from
the named implementation source. Do not create a Btrfs-local font, spacing,
button, tooltip, colour, or icon convention.

### 2. Lock down topology, candidate resolution, and preflight data

Build on the phase-0 contracts with fixture-driven adapter tests that
demonstrate a two-device Btrfs filesystem, the same fixtures in permuted
ObjectManager order, a Btrfs filesystem with subvolumes/default ID,
malformed/duplicate hierarchy, a missing Btrfs module, and every documented
unknown/disagreement value. Then implement the snapshot mapper and facade:

- extend `ResolvedBlock` with the exact documented data in
  [baseline.md](baseline.md#verified-udisks-field-inventory): `Block.IdLabel`,
  `Block.Size`, partition UUID, drive WWN/serial, and loop backing identity.
  Do not add a display field without a source/normalisation/absence row in
  that table;
- group Btrfs objects by canonical `uuid::Uuid` from the single completed
  snapshot, retaining every member, public primary-member state, private
  primary target, and its captured default/subvolume observations. Sort
  `None` fingerprints before canonical `Some` fingerprints and never expose
  the selected object path. Resolve `BtrfsSubvolumeRef` from a fresh completed
  snapshot only, derive the native relative path there, and return `Conflict`
  for changed ID/path/parent/descendant state or a diagnostic-blocked row;
- implement `capture_logical_candidate` and `load_logical_topology` so the
  sidebar's initial path merely captures an anchor. Resolve that anchor by
  block identity in the load snapshot, emit `Resolved`, `Missing`, or precise
  UDisks `Unavailable`, and never select an unrelated root;
- implement `preflight_logical_action` and confirmed execution through
  `storage-types`, `storage-contracts`, `src/operations/logical.rs`, and the
  UDisks implementation. The executor captures one fresh completed snapshot,
  compares its epoch to the returned preflight key, then checks
  ownership/eligibility/scope from that same snapshot before the native call.
  Local tools remain read-only display sources.

Do not expose data whose native meaning cannot be verified. In that case use
an explicit unknown/unavailable value and document it in the adapter test.

### 3. Establish the page state machine and refresh coordinator

Extend `src/state/logical.rs` with a typed page/draft model:

- anchored selection resolution and an `unresolved selected candidate` state;
- selected page entity/parent context, ordered applicable sections, and
  status/banner data;
- a discriminated logical action draft, draft validation error, preflight
  key/generation/loading/error, and final confirmation payload;
- transient action feedback derived from the existing pending action, action
  generation, progress, and latest result.

Represent the anchor, candidate resolution, preflight request key, returned
preflight key, and draft revision as typed state rather than optional
paths/strings. Increment draft revision for every input, target, action-kind,
cancellation, or retry change; accept a preflight only when its request key
matches current state, then store its returned epoch-bound key. A confirmation
stores that returned key and is invalidated before dispatch by any
draft/topology change.

Implement the `RefreshCoordinator` and its two `DomainRefreshState`s exactly
as specified in [spec.md](spec.md#refresh-coordinator). Keep the one monotonic
clock in the coordinator; drive refresh only through `request`,
`action_succeeded`, `start_next`, and `complete`. Model action success as a
strict start barrier, attach causes to requests/runs, queue a successor for a
pre-success run, and allocate both logical and physical run IDs only in the
coordinator. Add transition tests for a success during a load, a manual/event
request before and after success, two action successes before a successor, an
idle-domain immediate successor, and a stale completion. Assert exact
`requested_at`, `must_start_after`, `started_at`, run ID, and cause sets; no
test may simulate coalescing with a local counter.

Update `src/message/app.rs` and `src/update/mod.rs` with explicit messages for
opening/updating/cancelling/submitting each draft and for preflight results.
Keep generation checks at every asynchronous completion. A stale result may
not close a dialog, mutate a new draft, trigger a refresh, or overwrite the
currently selected entity's activity.

### 4. Build reusable logical controls and render the shared shell

Replace the placeholder renderer in `src/views/logical.rs` with small
logical-specific adapters under `src/controls/logical/`. Prefer extraction to
duplication: existing controls remain the owners of shared presentation.

- compose the logical header from the existing `src/views/disk.rs` header
  pattern and `src/controls/usage_pie.rs`; retain the app-level
  `Volume`/`Usage` treatment in `src/views/app.rs`, but remove the legacy
  Btrfs tab rather than adding logical-page tabs;
- use `src/controls/actions.rs::{icon_tooltip_action,trailing_actions_row}`
  for header and device actions. Bind the icon names and semantics defined in
  [spec.md](spec.md#component-icon-and-theme-contract), including
  `edit-delete-symbolic` for every destructive control;
- use active COSMIC spacing, typography, palette, and corner-radius tokens.
  Add only a minimal theme-aware writable-LED helper, with text/tooltip state
  for assistive technology;
- type/health/status header and transient action feedback;
- summary information with accessible text and unknown handling;
- member/device row with a labelled status LED, physical-navigation action,
  and trailing More/Remove icon controls;
- extract the reusable hierarchy presentation from `src/views/btrfs.rs` for
  the logical Subvolumes section; the legacy physical Btrfs view delegates to
  it while consolidation is in progress;
- flattened section headers and technical disclosure only where it is useful;
- operation menu/list item that presents a disabled reason;
- Btrfs subvolume tree/table.

Do not copy the mock's HTML/CSS or introduce custom styling to imitate it.
Render flat, applicable sections for all relevant logical entities. For Btrfs,
render the standard header followed by Devices and Subvolumes; do not render native
support, capacity-card, details, sources, duplicate-member, or Activity
sections. Wire Refresh and all empty, loading, unresolved, and source-failure
states before adding mutation forms.

Update `src/views/sidebar.rs` only where needed to preserve the selected
candidate/root relationship and to make member-to-physical navigation correct.
The global sidebar remains the sole topology navigator.

### 5. Implement the typed forms and reviews

Create focused logical dialog modules and state in `src/state/dialogs.rs`,
`src/views/dialogs/logical.rs`, and the corresponding message/update paths.
Implement the forms in [action-matrix.md](action-matrix.md) in this order:

1. Btrfs subvolume, snapshot, default-subvolume, label, absolute resize, and
   member-device flows;
2. LVM VG/LV/PV flows;
3. MD RAID creation/membership/control flows.

Each form opens a current preflight, validates locally, constructs one complete
`LogicalAction`, wraps it in `ConfirmedLogicalAction` with the accepted key,
and then uses the existing confirmation UI. Delete/stop/remove/repair styling
and review language must make destructive intent clear.
Preserve backend-specific blocked reasons verbatim enough to remain actionable.
The MD form renders the exact audited profile returned for the selected level;
if several profiles are eligible it requires an explicit labelled choice. Do
not infer a profile from level alone. Btrfs selected subvolumes/defaults use
`BtrfsSubvolumeRef`, never a selected relative-path string. Implement the
confirmation classes and revalidation rules in
[action-matrix.md](action-matrix.md#confirmation-and-freshness-binding)
verbatim.

### 6. Consolidate Btrfs entry points

Move the physical-volume Btrfs subvolume/snapshot surface onto the logical
Btrfs page/draft implementation. The physical page may retain a compact link
that selects the logical filesystem, but it must not own its own Btrfs form,
completion state, or mutation calls. Remove redundant messages/state/dialogs
only after callers have migrated. Keep read-only usage display only when it is
clearly identified as mount-point usage rather than filesystem allocation.

### 7. Localise, execute fixtures, and polish

Add Fluent keys for every new label, status, tooltip, validation message, and
empty state. Complete the automated and manual gates in
[validation.md](validation.md), run formatting/clippy, and record any
intentional UDisks data limitation in this plan's baseline or validation log.

Implement the targeted disposable-fixture logical suite before claiming the
manual or full-lab gate. Add a `harness-logical` recipe that invokes the
existing `full-lab` profile with `--suite logical --require-executed`; it must
exercise `logical.list_entities.schema_integrity`,
`logical.lvm.create_resize_delete_lv`, `logical.mdraid.create_start_stop_delete`,
`logical.btrfs.add_remove_member`, and new cases
`logical.btrfs.primary_ordering` and
`logical.btrfs.subvolume_ref_conflict`. Each Btrfs case uses the same
three-loop-device fixture. Add a narrowly-whitelisted
`FixtureCommand::FormatBtrfs { loop_device }`: it accepts only a ledger-owned
loop device and executes one fixed `mkfs.btrfs` argument vector—never an
arbitrary command or user argument. Setup creates marker-owned images, attaches
three 1-GiB loops, formats the first loop through that fixture-only command,
and adds the two additional members through the typed UDisks logical-operation
test boundary. The conflict case obtains a ref through the adapter, performs
an adapter-created temporary subvolume, deletes it through the out-of-band
fixture Btrfs proxy, waits for the ObjectManager epoch to advance, and proves
the stale ref is rejected by the next typed adapter action. Every
target/command is recorded in the fixture ledger; teardown unmounts, detaches
loops, removes marker-owned artifacts, and records successful cleanup before
its result is `Passed`.
`FullLabExecutor` must set up, execute, verify, and clean up those selected
cases through the fixture ledger. Before setting up a logical fixture it checks
for three attachable loops, the fixed `mkfs.btrfs` executable, and the UDisks
Btrfs module; a missing prerequisite is a failed fixture-environment result,
never a passing/optional or silently blocked logical case. Fixture setup may
use the reviewed fixture command executor; desktop production code may not
invoke it, `mkfs.btrfs`, or the `btrfs` CLI.

## Expected change map

| Area | Likely files |
| --- | --- |
| Domain/preflight | `crates/storage-types/src/logical.rs`, `crates/storage-contracts/src/traits/logical.rs`, contract tests |
| Native snapshot/discovery | `crates/storage-udisks/src/logical/{discover,resolve,proxy,operations}.rs`, new snapshot mapper/test support, adapter tests |
| Operations facade | `src/operations/logical.rs`, `src/operations/mod.rs` as required |
| App state/routing/refresh | `src/state/{app,logical}.rs`, `src/state/dialogs.rs`, `src/message/app.rs`, `src/update/{mod,logical}.rs` |
| UI | `src/views/logical.rs`, `src/controls/logical/*`, `src/controls/{actions,layout,usage_pie}.rs`, `src/views/dialogs/logical.rs`, `src/views/{app,disk,sidebar,btrfs}.rs` |
| Legacy Btrfs handoff | `src/state/btrfs.rs`, `src/update/{btrfs,volumes/btrfs}.rs`, Btrfs volume/dialog message modules |
| Verification | `tests/logical_*`, `crates/storage-{types,contracts,udisks}/tests/*`, `tools/storage-testing/{src,tests}/**`, `justfile`, required logical-suite multi-device Btrfs cases |

## Commit boundaries

Use reviewable commits in the same order: (1) typed domain/action migration,
(2) snapshot/identity/candidate/preflight mapper and fixtures, (3) refresh
coordinator/state/messages, (4) read-only page shell, (5) Btrfs forms,
(6) LVM forms, (7) MD forms, (8) Btrfs consolidation/localisation, and (9)
targeted logical fixture execution. Each commit must compile and its focused
test target must pass; do not land a visual control that emits an incomplete,
path-derived, or un-keyed action.

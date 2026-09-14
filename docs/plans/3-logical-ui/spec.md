# Logical Detail UI Specification

## Outcome

Selecting an LVM, MD RAID, or Btrfs logical root opens a complete detail page.
The page is readable without knowledge of opaque logical IDs, works with the
existing global sidebar, and gives each supported native capability a safe,
contextual workflow. It must remain correct while topology refreshes and
actions complete asynchronously.

## Execution contracts

This section is normative. Phase 0 implements these contracts, their pure
snapshot mappers, and their tests before the page shell or a form is changed.
It removes the need for a view/update implementation to infer identity,
availability, eligibility, or asynchronous ownership.

### Typed topology and display model

`LogicalEntity::metadata` remains diagnostic-only. It is never read by a page,
form, confirmation, or action resolver. `LogicalEntity` gains a
`LogicalEntityDetails` discriminant. The displayable fields currently carried
by generic entity fields migrate into its matching variant; after phase 0 a
renderer may read only `id`, `kind`, `parent_id`, `capabilities`, and `details`.
It may not infer a display value from `name`, `uuid`, size fields, member
strings, or `metadata`. The variants and their field ownership are the
following (equivalent names are acceptable only when they preserve these
fields and invariants):

```text
LogicalDisplay<T> = Known(T) | Unknown { reason: String }
UdisksEpoch = u64
DeviceNumber { major: u32, minor: u32 }
NonEmptyString = validated UTF-8 string with no leading/trailing ASCII whitespace

LogicalEntityDetails =
  LvmVolumeGroup(LvmVolumeGroupDetails)
  | LvmLogicalVolume(LvmLogicalVolumeDetails)
  | LvmPhysicalVolume(LvmPhysicalVolumeDetails)
  | MdRaidArray(MdRaidArrayDetails)
  | MdRaidMember(MdRaidMemberDetails)
  | BtrfsFilesystem(BtrfsFilesystemDetails)
  | BtrfsDevice(BtrfsDeviceDetails)
  | BtrfsSubvolume(BtrfsSubvolumeEntityDetails)

LvmVolumeGroupDetails {
  name: String,
  uuid: LogicalDisplay<String>,
  size: LogicalDisplay<u64>,
  used: LogicalDisplay<u64>,
  free: LogicalDisplay<u64>,
  logical_volumes: Vec<LvmLogicalVolumeSummary>,
  physical_volumes: Vec<LvmPhysicalVolumeSummary>,
}

LvmLogicalVolumeDetails {
  name: String,
  volume_group: LogicalEntityId,
  device_path: LogicalDisplay<String>,
  size: LogicalDisplay<u64>,
  activation: LogicalDisplay<LvmActivationState>,
}

LvmPhysicalVolumeDetails {
  block: Option<BlockDeviceRef>,
  display_path: LogicalDisplay<String>,
  size: LogicalDisplay<u64>,
  state: LogicalDisplay<LvmPhysicalVolumeState>,
}

MdRaidArrayDetails {
  name: String,
  uuid: LogicalDisplay<String>,
  level: LogicalDisplay<MdRaidLevelName>,
  size: LogicalDisplay<u64>,
  running: LogicalDisplay<bool>,
  health: LogicalDisplay<MdRaidHealth>,
  sync_progress: LogicalDisplay<ProgressRatio>,
  members: Vec<MdRaidMemberDetails>,
}

MdRaidMemberDetails {
  member_id: LogicalMemberId,
  block: Option<BlockDeviceRef>,
  display_path: LogicalDisplay<String>,
  size: LogicalDisplay<u64>,
  role: LogicalDisplay<MdRaidMemberRole>,
  state: LogicalDisplay<MdRaidMemberState>,
}

LvmLogicalVolumeSummary { entity_id: LogicalEntityId, name: String, size: LogicalDisplay<u64>, activation: LogicalDisplay<LvmActivationState> }
LvmPhysicalVolumeSummary { entity_id: LogicalEntityId, member: LvmPhysicalVolumeDetails }
LvmActivationState = Active | Inactive
LvmPhysicalVolumeState = Available
MdRaidHealth = Healthy | Degraded
MdRaidMemberRole = Active | Spare
MdRaidMemberState = Native { labels: Vec<String> }
MdRaidLevelName = validated, non-empty lower-case native level token

BtrfsFilesystemDetails {
  filesystem_uuid: uuid::Uuid,
  label: LogicalDisplay<String>,
  allocation: LogicalDisplay<BtrfsAllocation>,
  mount_usage: Option<MountPointUsage>,
  default_subvolume: LogicalDisplay<Option<NonZeroU64>>,
  primary_member: BtrfsPrimaryMember,
  members: Vec<BtrfsMember>,
  subvolumes: Vec<BtrfsSubvolumeDetails>,
  diagnostics: Vec<BtrfsTopologyDiagnostic>,
}

BtrfsMember {
  member_id: LogicalMemberId,
  block: Option<BlockDeviceRef>,
  display_path: LogicalDisplay<String>,
  label: LogicalDisplay<String>,
  size: LogicalDisplay<u64>,
  state: BtrfsMemberState,
}

BtrfsSubvolumeDetails {
  row_key: BtrfsSubvolumeRowKey,
  id: NonZeroU64,
  relative_path: BtrfsRelativePath,
  parent_id: Option<NonZeroU64>,
  hierarchy: BtrfsSubvolumeHierarchy,
  is_default: bool,
}

BtrfsDeviceDetails { filesystem: LogicalEntityId, member: BtrfsMember }
BtrfsSubvolumeEntityDetails { filesystem: LogicalEntityId, subvolume: BtrfsSubvolumeDetails }

BtrfsMemberState = Writable | ReadOnly | Unknown { reason: String }
BtrfsPrimaryMember = Selected { member_id: LogicalMemberId } | Unavailable { reason: String }
BtrfsSubvolumeHierarchy = Attached | Unparented { diagnostics: Vec<BtrfsTopologyDiagnostic> }
BtrfsAllocation { total: u64, used: u64, free: u64 }
MountPointUsage { mount_point: String, total: u64, used: u64, free: u64 }

BtrfsTopologyDiagnostic =
  DuplicateSubvolumeId { id: NonZeroU64, rows: Vec<BtrfsSubvolumeRowKey> }
  | DuplicateRelativePath { path: BtrfsRelativePath, rows: Vec<BtrfsSubvolumeRowKey> }
  | SelfParent { row: BtrfsSubvolumeRowKey, id: NonZeroU64 }
  | Cycle { rows: Vec<BtrfsSubvolumeRowKey> }
  | MissingParent { row: BtrfsSubvolumeRowKey, parent_id: NonZeroU64 }
  | PrimaryUnavailable { member_id: Option<LogicalMemberId>, reason: String }
  | FilesystemValueDisagreement { field: Label | Allocation, member_ids: Vec<LogicalMemberId> }

BtrfsSubvolumeRowKey { id: NonZeroU64, relative_path: BtrfsRelativePath, occurrence: NonZeroU32 }
```

`LvmPhysicalVolumeState::Available` means only that the source identified the
PV; it does not assert allocation or health. `MdRaidMemberState::Native` retains
the non-empty, sorted native state labels verbatim as typed data rather than a
comma-delimited display string. `MdRaidMemberRole` is `Active` or `Spare` only
when UDisks supplies that classification; otherwise its `LogicalDisplay` is
`Unknown`. `Unknown` is used for an unreadable property; the `LogicalDisplay`
reason names that property/source. `MdRaidHealth` is `Healthy` only when
`MDRaid.Degraded == 0`, `Degraded` only when it is greater than zero, and
otherwise `Unknown`. `BtrfsMemberState` is derived solely from
`Block.ReadOnly`: `false` is `Writable`, `true` is `ReadOnly`, and a missing or
failed read is `Unknown`. The writable indicator always has this text state as
well as its themed LED.

`BtrfsSubvolumeRowKey` exists only to retain and render malformed duplicate
rows. Its `occurrence` is assigned after sorting raw rows by `(id,
relative_path UTF-8 bytes, parent_id)` and counting identical `(id, path)`
pairs from one. It is not mutation identity. A `BtrfsSubvolumeRef` may be made
only from an `Attached` row with no diagnostic naming that row; otherwise its
operation is blocked with that diagnostic. Diagnostic row/member lists are
strictly sorted by their documented keys. A duplicate ID/path, self-parent,
cycle, or missing parent marks every implicated row `Unparented`; unaffected
rows remain attached. `PrimaryUnavailable` blocks native Btrfs controls but
does not discard the readable member rows.

The Btrfs root arrays are authoritative. If the sidebar retains
`BtrfsDevice`/`BtrfsSubvolume` child `LogicalEntity` rows, the snapshot mapper
emits them solely as projections of those arrays with the root `parent_id`.
They must carry the same typed values and are never populated by a second
UDisks call or by metadata parsing. Adapter tests compare the root and child
projections exactly.

`LogicalDisplay<BtrfsAllocation>` is `Known { total, used, free }` only when
every value has a documented filesystem-wide source and the reported values
agree after normalisation. The current adapter has no such source, so phase 0
emits `Unknown` rather than summing member sizes. `MountPointUsage` is a read-only
`statvfs` value, is optional, and is labelled *mount-point usage*; it is never
shown as Btrfs filesystem allocation. Existing generic zero-valued size fields
are migrated to `LogicalDisplay<u64>` (or an equivalently explicit optional
display type) before the renderer is allowed to use them.

`BtrfsRelativePath` is a validated, non-absolute, non-traversing path newtype.
Parent ID `0` from UDisks normalises to `None`; a subvolume ID of `0` is
invalid. The mapper retains every valid row. Duplicate IDs, duplicate paths
with a different ID, self-parenting, cycles, and missing non-root parents mark
only the affected rows `Unparented` and add a deterministic diagnostic.

### Identity, snapshot, and candidate resolution

All native Btrfs discovery, preflight, and execution begin by capturing one
crate-private snapshot. Its complete shape is:

```text
ResolvedLogicalSnapshot {
  epoch: UdisksEpoch,
  blocks: Vec<ResolvedBlock>,
  logical_objects: Vec<ResolvedLogicalObject>,
  btrfs_observations: Vec<BtrfsObservation>,
}

BtrfsObservation {
  filesystem_uuid: uuid::Uuid,
  primary: InternalBtrfsPrimaryTarget,
  default_subvolume: BtrfsObservationValue<Option<u64>>,
  subvolumes: BtrfsObservationValue<Vec<RawBtrfsSubvolume>>,
}

InternalBtrfsPrimaryTarget {
  member_id: LogicalMemberId,
  block_id: BlockDeviceId,
  fingerprint: Option<BlockDeviceFingerprint>,
  object_path: zbus::zvariant::OwnedObjectPath,
}

RawBtrfsSubvolume { id: u64, parent_id: u64, relative_path: String }
BtrfsObservationValue<T> = Observed(T) | Unavailable { source: String, reason: String }
```

The adapter captures ObjectManager data, derives candidate Btrfs groups,
selects one internal primary, and calls `GetSubvolumes` and
`GetDefaultSubvolumeID` on that primary exactly once before constructing the
snapshot. The pure mapper receives only this completed snapshot; it never
enumerates ObjectManager paths, reads a block property, or makes a D-Bus call.
`InternalBtrfsPrimaryTarget` is adapter-private and never appears in
`LogicalEntity`, a preflight response, an action, a log intended for the UI, or
an error returned to the UI.

For the default observation, `Observed(None)` maps to `Known(None)` and
`Observed(Some(id))` maps to `Known(Some(NonZeroU64))` only when `id` is
non-zero and within the native `u32` range; zero or an out-of-range value maps
to `Unknown` with an invalid-native-value reason. An unavailable observation
maps to `Unknown` with its source/reason. `RawBtrfsSubvolume` values are
validated independently under the row-key/hierarchy rules; an unavailable
subvolume observation keeps the filesystem/member data readable but blocks
subvolume-derived controls.

A snapshot's Btrfs members sort by `(major, minor, fingerprint, object_path)`.
`major` then `minor` are ascending unsigned values. `fingerprint` orders
`None` before `Some`; `Some` uses the canonical byte encoding of the typed
fingerprint below. `object_path` orders by its UTF-8 bytes ascending. The first
sorted member with the Btrfs interface is its sole primary. The public
`BtrfsPrimaryMember` exposes only the selected member ID or unavailability,
never the object path. Discovery, captured Btrfs observations, preflight, and
mutation all use that same selection rule. A failed or missing primary returns
unavailable/conflict as applicable; it never falls back to another member.

`Block.IdUUID` is a Btrfs filesystem grouping key, not a member identity.
`ResolvedBlock` obtains a strong fingerprint only from the following typed
values, in this precedence order:

```text
BlockDeviceFingerprint =
  Partition { partition_uuid: uuid::Uuid, drive: DriveFingerprint }
  | Drive(DriveFingerprint)
  | Loop { backing_device: DeviceNumber, backing_inode: u64 }

DriveFingerprint { wwn: NonEmptyString, serial: NonEmptyString }
```

1. `Partition.UUID` bound to the containing drive's non-empty WWN and serial;
2. the block's containing drive non-empty WWN and serial;
3. loop backing device and inode.

A partition UUID without a `DriveFingerprint` is weak display data, not a
fingerprint, because it can be cloned. A fingerprint's canonical byte encoding
is its discriminant followed by length-prefixed UTF-8 values or fixed-width
big-endian numeric values; this encoding is the comparator value above and is
the equality value used by a `BlockDeviceRef`.

If none is available, the member remains readable but has no `BlockDeviceRef`.
The picker cannot offer it and member mutation is inert with that exact reason.
`Block.IdUUID`/`Block.IdType`, a partition UUID without its drive binding, an
object path, a device number, and a display path are never sufficient fallbacks
for a `BlockDeviceRef`.

The physical sidebar may send a display path once to
`capture_logical_candidate`. That read-only facade resolves the current block
and returns the following anchor. The state stores the anchor, not the path.

```text
LogicalCandidateKind = Btrfs | LvmPhysicalVolume | RaidMember

LogicalCandidateAnchor {
  kind: LogicalCandidateKind,
  block_id: BlockDeviceId,
  fingerprint: Option<BlockDeviceFingerprint>,
  observed_epoch: UdisksEpoch,
  display_path: String,
}

LogicalLoadRequest { anchor: Option<LogicalCandidateAnchor> }
LogicalLoadResult { topology: LogicalTopology, candidate_resolution: LogicalCandidateResolution }

LogicalCandidateResolution =
  Resolved { root_id: LogicalEntityId }
  | Missing
  | Unavailable { source: String, reason: String }
```

`display_path` is retained solely for the existing human-readable unavailable
page and is never compared for resolution or copied into a `BlockDeviceRef`.
`load_logical_topology(LogicalLoadRequest { anchor })` captures a new snapshot
and returns `LogicalLoadResult`.

Resolution compares block ID and fingerprint when present. For an anchor
without a strong fingerprint, an epoch change yields `Unavailable`; it may not
resolve a recycled device number. The candidate resolution is calculated from
the same snapshot as topology and is never replaced by global source status.
The local-tools source can merge read-only display fields but cannot resolve a
candidate or authorize a mutation.

### Preflight and confirmation contract

`LogicalOperation` is too coarse for a form. Add a payload-free,
discriminated `LogicalActionKind` with one case for each action-matrix row:

```text
LogicalActionKind =
  CreateLvmVolumeGroup | AddLvmPhysicalVolume | RemoveLvmPhysicalVolume
  | CreateLvmLogicalVolume | DeleteLvmVolumeGroup | ResizeLvmLogicalVolume
  | ActivateLvmLogicalVolume | DeactivateLvmLogicalVolume | DeleteLvmLogicalVolume
  | CreateMdRaidArray | AddMdRaidMember | RemoveMdRaidMember | StartMdRaidArray
  | StopMdRaidArray | CheckMdRaidArray | RepairMdRaidArray | DeleteMdRaidArray
  | AddBtrfsDevice | RemoveBtrfsDevice | ResizeBtrfsFilesystem | SetBtrfsLabel
  | SetBtrfsDefaultSubvolume | CreateBtrfsSubvolume | DeleteBtrfsSubvolume
  | CreateBtrfsSnapshot
```

Use it with these contract values:

```text
LogicalPreflightRequestKey {
  target: Landing | Root(LogicalEntityId) | Candidate(LogicalCandidateAnchor),
  action_kind: LogicalActionKind,
  logical_load_generation: u64,
  draft_revision: u64,
}

LogicalPreflightRequest { request_key: LogicalPreflightRequestKey }

LogicalPreflightKey {
  request_key: LogicalPreflightRequestKey,
  udisks_epoch: u64,
}

LogicalPreflight {
  key: LogicalPreflightKey,
  availability: Ready | Blocked { reason },
  device_candidates: Vec<LogicalDeviceCandidate>,
  constraints: LogicalInputConstraints,
  review: LogicalReviewData,
}

ConfirmedLogicalAction { action: LogicalAction, preflight_key: LogicalPreflightKey }
```

The UI constructs only `LogicalPreflightRequestKey`; the adapter attaches the
UDisks epoch when it returns `LogicalPreflightKey`. State accepts a preflight
result only when `key.request_key` matches its current request key, then stores
the complete returned key for confirmation. A topology refresh, draft change,
target change, cancellation, or retry invalidates that stored key. This avoids
asking the UI to predict an epoch before it asks the adapter to capture one.

```text
LogicalDeviceCandidate =
  Ready { device: BlockDeviceRef, display: LogicalCandidateDisplay }
  | Blocked { display: LogicalCandidateDisplay, reason: CandidateBlockReason }

LogicalCandidateDisplay {
  label: LogicalDisplay<String>,
  path: LogicalDisplay<String>,
  size: LogicalDisplay<u64>,
}

CandidateBlockReason =
  NoStrongIdentity
  | AlreadyTargetMember
  | StructuredDataSignature { signature: String }
  | SourceUnavailable { source: String, reason: String }
  | Ineligible { reason: String }

LogicalInputConstraints {
  byte_size: Option<ByteSizeConstraint>,
  md_profiles: Vec<MdRaidProfileOption>,
}

ByteSizeConstraint { minimum: NonZeroU64, maximum: Option<NonZeroU64>, alignment: NonZeroU64 }
MdRaidProfileOption { profile: MdRaidCreateProfile, label: String, member_minimum: NonZeroU8, chunk_bytes: u64, metadata_version: String }

LogicalReviewData =
  None
  | DeviceInputs { devices: Vec<BlockDeviceRef> }
  | DestructiveScope { primary: LogicalEntityId, scope: ConfirmedDestructiveScope, policy: DestructiveScopePolicy }
  | MemberRemoval { primary: LogicalEntityId, member: BlockDeviceRef, policy: MemberRemovalPolicy }
  | BtrfsSubvolumeDeletion { filesystem: LogicalEntityId, subvolume: BtrfsSubvolumeRef, effect: NonRecursive }

DestructiveScopePolicy =
  DeleteLvmVolumeGroup { pv_label_policy: LvmWipePolicy, configuration_policy: ConfigurationCleanupPolicy }
  | DeleteLvmLogicalVolume { configuration_policy: ConfigurationCleanupPolicy }
  | DeleteMdRaidArray { configuration_policy: ConfigurationCleanupPolicy }

MemberRemovalPolicy =
  RemoveLvmPhysicalVolume { pv_label_policy: LvmWipePolicy }
  | RemoveMdRaidMember { member_signature_policy: MdRaidMemberWipePolicy }
  | RemoveBtrfsDevice
```

`Ready` is the only candidate form that contains a `BlockDeviceRef` and the
only one the picker may select. Every otherwise-discovered candidate remains a
`Blocked` row with its exact reason. `CandidateBlockReason` and its display are
non-authoritative and never become an action input. `ByteSizeConstraint` is
inclusive; `minimum <= maximum` when a maximum exists, and valid input is a
multiple of `alignment`. `md_profiles` is the complete sorted catalog for the
specific action; an unavailable catalog blocks the preflight rather than
returning an empty, apparently valid selector. Name and new-subvolume text
constraints are the typed action/newtype validators and their errors, not a
second preflight-owned rule set. `LogicalReviewData` is keyed to the exact
action kind: `DeviceInputs` is used only by selected-device creation/addition
actions, `DestructiveScope` only by the three scope-bound deletions,
`MemberRemoval` only by member-removal actions, and
`BtrfsSubvolumeDeletion` only by non-recursive Btrfs deletion. It is never
inferred from rendered text. All device-ref vectors and scopes are canonical;
`DestructiveScope` excludes `primary`.

`LogicalOperations` gains `preflight_logical_action` and changes execution to
accept `ConfirmedLogicalAction`. The UDisks implementation first captures one
fresh completed snapshot, compares its epoch to the returned preflight key,
then revalidates target capability, candidates, constraints, selected member
ownership, Btrfs ref, primary selection, and scope from that same snapshot
immediately before its native call. An epoch or value change returns `Conflict`.
The UI accepts a preflight result only by its request key and accepts a final
action result only when its stored complete preflight key still belongs to the
current confirmation. A confirmation cannot edit an action and becomes invalid
on any draft or topology change.

The constraint/profile sources are fixed as follows. LVM available capacity
comes from `VolumeGroup.FreeSize`; where no audited native bound exists the
explicit bound is `min = 1`, `max = None`, `alignment = 1` and the adapter is
final authority. The Btrfs resize form uses that open absolute-byte constraint;
it never translates `max`, grow-by, or shrink-by. MD profiles come from a
versioned, contract-owned audited profile catalog, not a level-to-profile
guess; the initial catalog may expose one profile per level but its preflight
shape supports several labelled profiles. An absent source produces a blocked
form, not an invented bound/profile.

### Refresh coordinator

`RefreshCoordinator` is owned by app state and is the only code allowed to
allocate logical or physical load generations. It owns a single monotonic
`RefreshClock`; all request and run ordering is expressed in that clock. It has
one `DomainRefreshState` per domain:

```text
RefreshDomain = Logical | Physical
RefreshCause = Manual | DeviceEvent | ActionSuccess { action_generation: u64 }
RefreshClock = u64

RefreshRequest {
  requested_at: RefreshClock,
  must_start_after: Option<RefreshClock>,
  causes: BTreeSet<RefreshCause>,
}

RefreshRun {
  run_id: u64,
  started_at: RefreshClock,
  causes: BTreeSet<RefreshCause>,
}

DomainRefreshState {
  running: Option<RefreshRun>,
  queued: Option<RefreshRequest>,
  next_run_id: u64,
}

RefreshCoordinator {
  clock: RefreshClock,
  logical: DomainRefreshState,
  physical: DomainRefreshState,
}
```

`request(domain, cause)` increments `clock`, creates a request with that value
as `requested_at` and no start barrier, then applies `enqueue`.
`action_succeeded(generation)` increments `clock` once to create a success
barrier, and creates one request in each domain with
`must_start_after = Some(barrier)` and cause `ActionSuccess { generation }`.
Failed actions create none. `enqueue` joins a request to the current run only
when no request is already queued and the run's `started_at` is strictly later
than the request's barrier (`None` is satisfied by every run). Otherwise it
joins (or creates) the queued request, using the greatest barrier and unioning
causes. A queued request always receives later manual/event requests, so a
manual/event request arriving after an action success joins its post-success
successor rather than the pre-success running run.

`enqueue` calls `start_next(domain)` immediately when the domain is idle.
`start_next(domain)` starts the queued request, increments `clock` for
`started_at`, allocates `run_id`, and asserts `started_at > must_start_after`
when a barrier exists. If it is busy, only `complete(domain, run_id)` may clear
the run; it ignores a stale `run_id` and then calls `start_next` exactly once.
Manual and device-event requests after an action success therefore join the
queued or running post-success run, while a run that began before the barrier
cannot satisfy that success. Multiple action successes may join one successor
only when its `started_at` is later than every merged barrier. Update arms emit
only `request`, `action_succeeded`, and `complete` messages; they do not
directly start `LoadLogicalEntities` or `LoadDrivesIncremental`.

## Page structure

Every root page uses one vertically ordered shape; child selection keeps the
same shell but only shows controls appropriate to that child.

```text
icon  Display name                           [Refresh] [Manage …]
      human-readable path/UUID when useful

applicable summary or member sections
subvolumes (Btrfs filesystem only)
```

The approved [Btrfs mock](mock.html) is the visual reference for the order and
relative density of the Btrfs header, Devices, and Subvolumes sections. It is
not a literal implementation: its CSS, Unicode stand-in icons, and hard-coded
pixel values must not be copied into the Rust UI.

- The title row contains a kind label and a health/status badge. An absent
  value is shown as `Unknown`, never as a fabricated healthy state.
- Capacity values use `bytes_to_pretty`; absent values render as `—`, not
  `0 bytes`. Compact, app-standard usage information may remain in the
  header, but the Btrfs page does not repeat it as a strip of metric cards.
  The opaque entity ID and raw metadata do not become a primary page section.
- `Refresh` schedules the existing logical load generation. It is disabled
  while a load is in flight, but a user can still read the last successfully
  loaded page.
- A source failure or an action result appears as a concise status/banner in
  the page. Multiple status sources are not silently collapsed: UDisks
  unavailability explains why mutation controls are disabled while local
  discovery data remains readable.
- `Manage` contains only operations relevant to the selected entity. A
  supported-but-blocked operation remains visible with its exact backend
  reason; unrelated operations do not appear as generic `Unavailable` rows.
- Destructive actions use the existing review/confirmation step. Its body
  names the entity, the selected input members, and every current collateral
  item that the native action can affect. The precise scope/review binding for
  every action is in [action-matrix.md](action-matrix.md). A conflict after
  review retains the draft, shows the conflict, and requires refresh/review
  before another submission.

## Component, icon, and theme contract

The logical page must compose existing controls before introducing a new one.
Extract a small shared primitive only when an existing control cannot express
the logical data without duplicated styling or message wiring.

| Mock element | Production source of truth | Implementation rule |
| --- | --- | --- |
| App `Volume` / `Usage` header | [`src/views/app.rs`](../../../src/views/app.rs) `header_center` and its existing tab button class | Keep this application-level header treatment; do not create a second logical-page tab bar. Remove the legacy Btrfs management tab as the logical Btrfs page becomes its replacement. |
| Logical header identity, action row, and compact usage ring | [`src/views/disk.rs`](../../../src/views/disk.rs) header composition and [`src/controls/usage_pie.rs`](../../../src/controls/usage_pie.rs) | Build a logical-specific header from the same text, row, action, and pie controls. Do not copy the physical-disk field layout or hand-draw another donut. |
| Icon-only header and row actions | [`src/controls/actions.rs`](../../../src/controls/actions.rs) `icon_tooltip_action` and `trailing_actions_row` | Reuse these for sizing, tooltip placement, disabled behaviour, alignment, and trailing-action spacing. |
| Device and subvolume list rows | [`src/controls/layout.rs`](../../../src/controls/layout.rs), [`src/views/sidebar.rs`](../../../src/views/sidebar.rs), and [`src/views/btrfs.rs`](../../../src/views/btrfs.rs) | Reuse row padding/selection treatment and extract the existing Btrfs tree row presentation rather than maintaining two trees. |
| Dialogs, fields, and confirmation | [`src/controls/fields.rs`](../../../src/controls/fields.rs), [`src/controls/wizard.rs`](../../../src/controls/wizard.rs), and [`src/views/dialogs/logical.rs`](../../../src/views/dialogs/logical.rs) | Reuse the existing form and destructive-review flows; logical pages only launch typed drafts. |

- Use `theme::active().cosmic().spacing` and the project's text constructors
  (`title*`, `body`, `caption`) instead of new point sizes, font families, or
  literal spacing scales. Headings and selected labels use the existing
  semibold treatment; metadata remains caption/body text.
- Use theme palette values and corner radii. The writable LED uses the theme
  success colour, with an accessible `Writable` label/tooltip; it is never a
  hard-coded green or the only expression of state.
- All icon-only controls use `widget::icon::from_name`, a tooltip, and an
  accessible Fluent label. Use the icon names already established in this
  application: `view-refresh-symbolic` (refresh), `tag-symbolic` (label),
  `list-add-symbolic` (add device/subvolume), `camera-photo-symbolic`
  (snapshot), `edit-delete-symbolic` (remove/delete), and
  `go-next-symbolic` / `go-down-symbolic` (tree expansion). The per-device
  More control uses the existing `preferences-other-symbolic` choice unless a
  project-wide overflow icon is added first; no Unicode glyph is shipped.
- New visual helpers must be parameterised by the active COSMIC theme. A
  helper may own the small LED indicator, but it must not fork app-wide button,
  font, colour, spacing, or tooltip styling.

## Flattened content

There is no internal tab rail and no separate activity page. Sections occur in
their natural operational order; the global `Volume`/`Usage` switch remains an
application-level control, not a logical-page tab.

### Header and summary

The overview gives the storage-specific summary and readable identity:

- **LVM VG:** total/allocated/free capacity, LV count, PV count, UUID, and
  health if supplied.
- **LVM LV:** size, active/inactive state, parent VG, and device path if
  known.
- **MD RAID:** level, capacity, health/degraded state, active/spare member
  count, and sync progress when present.
- **Btrfs filesystem:** label (falling back to filesystem UUID), UUID, mount
  point when known, and optional explicitly labelled *mount-point usage* in
  the standard header. It does not render a native-support banner, a
  capacity/used/free metric strip, filesystem-details panel, or sources panel.
- **Btrfs subvolume:** relative path, numeric subvolume ID, parent
  subvolume/parent filesystem, and a default indicator when applicable.

`LogicalEntity::metadata` is not a page layout API. The renderer maps known
typed/discovered fields to labels; an explicit typed diagnostic may be shown in
Technical information, but raw metadata is never rendered.

### Devices and members

The devices section follows the Btrfs header directly. Each Btrfs member row
has a small status LED at its far left; writable is green. The LED has an
accessible `Writable` label so colour is not its only meaning. The device
name/path and readable model/size occupy the centre, while icon-only More and
Remove controls occupy the far right with tooltips and accessible names.

The list inherits the app's standard row padding and action spacing. A member
remove control uses the established destructive icon/style and opens the
existing review flow; it is not a red text link or a custom button treatment.

Other logical roots use the same flattened member-list principle when they
have members. A row with a device path opens the existing physical sidebar
selection when that physical device still exists. A stale or no-longer-visible
path remains readable but is inert.

Member removal is offered on the member row only when the selected root's
capability permits it and that row carries a fresh device reference. The page
never turns a stale path into a new reference. `Add device` lives in the
root's Manage menu and opens the typed device picker described below.

Pending operation, progress, and completion/failure feedback appear next to
the applicable action or in the page's transient status presentation. They do
not require a separate Activity section or retain a debug-formatted Rust enum.
Generation rules still prevent a late completion from overwriting a newly
selected page.

### Subvolumes (Btrfs filesystem only)

The Btrfs filesystem page places a deterministic, parent-aware subvolume
tree/table directly below Devices. Each row shows path, ID, default status,
and only its applicable actions: make default, snapshot, or delete. The
section header has `New subvolume` and `New snapshot` actions. Btrfs
subvolume children selected from the sidebar link back to their parent
filesystem context for device-level management.

No separate Btrfs metadata, details, sources, devices-and-members, or activity
section is rendered.

Sibling rows sort by case-sensitive relative path and then numeric subvolume
ID. A root-level subvolume has parent ID zero or an absent parent. A duplicate
subvolume ID, duplicate path with a different ID, self-parent, parent cycle, or
missing non-root parent is a source-conflict diagnostic: render the affected
rows in a labelled, deterministic `Unparented` group (same comparator), disable
their actions, and keep the rest of the filesystem readable. Do not choose an
arbitrary parent or silently discard a conflicting row.

## Entry, loading, and empty states

- Opening a physical Btrfs/LVM/RAID candidate retains a
  `LogicalCandidateAnchor`: the current `BlockDeviceId`, observed object-manager
  epoch, available strong fingerprint, and a display-only path. It is navigation
  state, never mutation input. Re-resolution requires the same block ID and
  fingerprint when one exists; without a strong fingerprint, any epoch change
  makes the anchor unavailable rather than allowing a recycled device number to
  resolve to a different root.
  The logical-load facade returns a typed candidate resolution alongside the
  topology: `Resolved(root_id)`, `Unavailable { reason, source }`, or
  `Missing`. Source status remains global and is never overloaded with this
  per-candidate result. A stale anchor or a candidate with no Btrfs interface
  produces the anchored unavailable page with its precise UDisks/module reason
  and Retry. Do not select the first unrelated root.
- A selected entity missing after refresh produces a clear `This logical item
  is no longer present` state and selects no replacement until the user makes
  one, except the existing explicit candidate-to-root resolution.
- The empty state distinguishes `No logical storage found` from `Logical
  discovery failed`. It includes source status only where it explains the
  result.
- Existing load and action generations remain the authority for stale-message
  suppression. A successful current action submits exactly one logical and one
  physical refresh request to the refresh coordinator; a failure submits none
  and does not dismiss the dialog. A request begun before native success can
  never satisfy that action's refresh. Concurrent manual/event requests that
  begin after success may coalesce with the action's pair, but their shared
  result carries both causes and runs once per domain. The coordinator queues a
  successor when a pre-success load is in flight, so each successful action is
  observed by one post-success logical and physical snapshot.

## Typed forms and safety

Form state stores semantic values and a selected, freshly supplied
`BlockDeviceRef`; it does not store shell snippets, a serialized device list,
or paths as operation identity. A selected Btrfs subvolume is instead exactly:

```text
BtrfsSubvolumeRef {
  filesystem: LogicalEntityId,
  id: NonZeroU64,
  expected_relative_path: BtrfsRelativePath,
  expected_parent_id: Option<NonZeroU64>,
  observed_topology_epoch: UdisksEpoch,
}
```

The adapter resolves the ID in a fresh snapshot and returns `Conflict` if its
filesystem, path, parent, descendant, or diagnostic-blocked state changed; the
native relative path is therefore derived by the adapter, never accepted as
selected identity from the UI. A newly typed subvolume/snapshot destination is
allowed only as a validated creation payload, not as an identity. Form
validation happens before confirmation and again through
`LogicalAction::validate`/the adapter on submission.

For an action needing a device or destructive review, the UI requests an
operation-specific preflight from the logical operations facade with a
`LogicalPreflightRequestKey` of `{candidate/root, action kind, logical-load
generation, draft revision}`. The adapter records the UDisks object-manager
epoch in its returned `LogicalPreflightKey`. It returns every discovered
candidate as either a selectable `Ready` row with its current
`BlockDeviceRef` or a readable `Blocked` row with the precise reason, as well
as validation constraints and any operation-specific blocking reason. It
supplies a reviewed collateral scope only for actions whose typed request binds
one; other destructive actions return explicit review facts such as their
selected current member refs and fixed preserve policy.

Changing root/candidate, action kind, any draft field, or cancelling/retrying a
draft increments its revision and invalidates the old result. The state accepts
a preflight result only when its request key matches; otherwise it is ignored.
A confirmation stores the returned complete key and becomes invalid on any
draft or topology change. The adapter independently re-resolves all identities
and scopes immediately before the native call. If preflight cannot prove a
candidate is safe/current, no candidate is selectable and the form explains
why.

The precise fields and ownership of each form are in
[action-matrix.md](action-matrix.md). In particular, the Btrfs resize form
offers an absolute byte size only. `max`, grow-by, and shrink-by remain
visible as unsupported native syntax where relevant, with the exact audited
reason, and can never be silently translated.

## Btrfs topology requirements

The UDisks discovery source groups Btrfs interface objects by canonical
filesystem UUID before producing logical entities. Canonicalisation trims ASCII
whitespace, parses a UUID case-insensitively, and serialises lowercase hyphenated
UUID text; a missing or malformed UUID creates an anchored unavailable
diagnostic and never a name-derived root. One canonical UUID yields exactly one
`BtrfsFilesystem` root with:

- every Btrfs member as a `BtrfsDevice` child projection of the root's typed
  `BtrfsMember`, with a fresh `BlockDeviceRef` when a strong fingerprint
  exists, readable path, and only values whose provenance is known;
- member order fixed by `(BlockDeviceId major, minor, fingerprint, UDisks
  object path)`. The first member with a Btrfs interface in that order is the
  sole primary proxy for discovery and mutation in that snapshot. A primary
  proxy failure is reported as unavailable; the adapter never falls back to an
  arbitrary secondary proxy during a call;
- subvolume children carrying typed/validated ID and parent-ID information;
- label, aggregate capacity, usage/free space, and default-subvolume values
  only when a documented filesystem-wide UDisks source supplies them. Per-block
  capacity is never summed. If multiple reported filesystem-wide values
  disagree after normalisation, publish `Unknown` plus a diagnostic rather than
  selecting one;
- subvolumes read from the deterministic primary proxy only, then normalised
  and validated before tree construction. Duplicate/invalid hierarchy handling
  follows the Subvolumes rule above.

The implementation must document the UDisks property/method and normalisation
used for every field, and add fixtures for a two-device filesystem. Fixtures
must permute ObjectManager input order and primary-candidate order and prove
the emitted root, primary proxy, members, subvolumes, and action target are
unchanged. Deduplicating per-device roots after the fact is forbidden because
it loses membership information.

## Accessibility and localisation

All new user-facing strings use Fluent keys. Icon-only actions have tooltips,
disabled controls retain readable text/reasons, the section and form order is
keyboard navigable, and status is communicated by text as well as colour.

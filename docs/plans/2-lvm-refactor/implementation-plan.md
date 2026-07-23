# LVM Refactor Implementation Plan

**Status:** Planned
**Integration branch:** create 079-lvm-refactor from origin/main at
0ba27cc2caac19acb7a8d98747f8b65058dab877.
**Behaviour source:** origin/079-lvm-support at
93094bd4ecea0f7b6236b17e34753470b8ee616d.
**Required outcome:** every logical-storage capability, UI change, asynchronous
sidebar change, test harness, lab workflow, and integration suite from the
behaviour source is present on the new branch. Its removed project-service
transport is replaced, and only the explicit identity/native-semantics and
fixture-boundary conversions in this plan are additionally permitted.

## Non-negotiable integration rules

This is not a Git rebase, because the old branch's project-owned privileged
D-Bus service no longer exists on main. It is a three-way semantic graft:
`reconciliation.md` first identifies current-main-only, feature-only, and
independent-overlap ranges. Feature-only UI, application-logic, and harness
ranges are literal source-first transplants; an overlapping main range is kept
or composed according to its reconciliation row, never overwritten as a
preliminary copy. Only the narrow mechanical, transport,
identity/native-semantics, fixture-boundary, harness-execution,
async-concurrency, or completed main-preservation adaptations in
source-fidelity.md are permitted. Do not redesign or independently reimplement
a source file that can be transplanted.

Do not recreate the removed service, its client, JSON wire format, system unit,
Polkit policy, or project D-Bus signals. Replace only those transport calls
with the existing in-process operations architecture and UDisks2's native
interfaces while retaining the callers' exact control flow and behaviour. The
separate ledgered identity/native-semantics, fixture-boundary,
harness-execution, and concurrency conversions remain limited to the cases
named in source-fidelity.md.

The final feature must retain all of the following old-branch behaviour:

- Logical discovery and the entity hierarchy for LVM VG/LV/PV, MD RAID
  array/member, Btrfs filesystem/device/subvolume.
- Every action in the branch's LVM, MD RAID, and Btrfs dialogs and control
  surface, including disabled/actionable reasons and post-operation refresh.
- The logical detail tabs, overview/members/operations/Btrfs content, selection
  persistence, member navigation, status reporting, and asynchronous sidebar.
- The complete storage-testing harness, disposable lab, fixture ledger, and all
  existing integration test families.

The feature has no silent read-only downgrade. An action which cannot be
represented by the installed UDisks2 interfaces is rendered disabled with its
specific unavailable/unsupported reason; when the interface is present, the
typed operation invokes the native UDisks2 method and lets its Polkit
authorization result reach the user.

“Equivalent” has a strict meaning here: the same source controls, dialog
layout, text, defaults, validation, ordering, icons, state fields, message
variants, update branches, refresh timing, tests, fixtures, and lab behaviour.
Similarity is insufficient. Every non-verbatim source range must have a
source-fidelity ledger entry and an equivalence test; every range independently
changed on main additionally needs the completed three-way preservation row and
separate evidence that main behaviour remains.

## Exact old-action replacement matrix

The first column is the public behaviour to preserve, not source code to copy.
Each request is a typed contract value; no UI or operations module constructs a
command line, D-Bus object path, JSON payload, or authorization option map.
Source text fields may remain visible, but their values must be resolved against
current state into typed values before a request is constructed.

| Old branch operation | Typed action | Native adapter call | Completion |
| --- | --- | --- | --- |
| lvm_create_volume_group | CreateLvmVolumeGroup | Manager.LVM2.VolumeGroupCreate | refresh physical and logical |
| lvm_delete_volume_group | DeleteLvmVolumeGroup(PreservePvLabels, PreserveConfiguration, ConfirmedScope) | VolumeGroup.Delete(wipe=false, tear-down=false) | refresh physical and logical |
| lvm_add_physical_volume | AddLvmPhysicalVolume | VolumeGroup.AddDevice | refresh physical and logical |
| lvm_remove_physical_volume | RemoveLvmPhysicalVolume(PreservePvLabel) | VolumeGroup.RemoveDevice(wipe=false) | refresh physical and logical |
| lvm_create_logical_volume | CreateLvmLogicalVolume | VolumeGroup.CreatePlainVolume | refresh physical and logical |
| lvm_delete_logical_volume | DeleteLvmLogicalVolume(PreserveConfiguration, ConfirmedScope) | LogicalVolume.Delete(tear-down=false) | refresh physical and logical |
| lvm_resize_logical_volume | ResizeLvmLogicalVolume | LogicalVolume.Resize | refresh physical and logical |
| lvm_activate_logical_volume | ActivateLvmLogicalVolume | LogicalVolume.Activate | refresh physical and logical |
| lvm_deactivate_logical_volume | DeactivateLvmLogicalVolume | LogicalVolume.Deactivate | refresh physical and logical |
| mdraid_create_array | CreateMdRaidArray | Manager.MDRaidCreate with the locked source-compatible profile | refresh physical and logical only after the resulting array is running |
| mdraid_delete_array | DeleteMdRaidArray(PreserveConfiguration, ConfirmedScope) | MDRaid.Delete(tear-down=false) | refresh physical and logical; explicitly ledger its all-member metadata destruction |
| mdraid_start_array | StartMdRaidArray | MDRaid.Start | refresh physical and logical |
| mdraid_stop_array | StopMdRaidArray | MDRaid.Stop | refresh physical and logical |
| mdraid_add_member | AddMdRaidMember | MDRaid.AddDevice | refresh physical and logical |
| mdraid_remove_member | RemoveMdRaidMember(PreserveMemberSignatures) | MDRaid.RemoveDevice(wipe=false) | refresh physical and logical |
| mdraid_request_sync_action check | RequestMdRaidSync(Check) | MDRaid.RequestSyncAction("check") | refresh physical and logical |
| mdraid_request_sync_action repair | RequestMdRaidSync(Repair) | MDRaid.RequestSyncAction("repair") | refresh physical and logical |
| btrfs_add_device | AddBtrfsDevice | Filesystem.BTRFS.AddDevice | refresh physical and logical |
| btrfs_remove_device | RemoveBtrfsDevice | Filesystem.BTRFS.RemoveDevice | refresh physical and logical |
| btrfs_resize | ResizeBtrfsFilesystem | Filesystem.BTRFS.Resize for an absolute request; retain unsupported source size syntax as disabled | refresh physical and logical |
| btrfs_set_label | SetBtrfsLabel | Filesystem.BTRFS.SetLabel | refresh physical and logical |
| btrfs_set_default_subvolume | SetBtrfsDefaultSubvolume | Filesystem.BTRFS.SetDefaultSubvolumeID | refresh physical and logical |

The UDisks2 method result is definitive. Do not emulate a missing method with
vgcreate, lvcreate, mdadm, btrfs CLI, sysfs writes, sudo, pkexec, a root helper,
or a project authorization policy. The fixture-only lab executor is not an
application fallback: it may perform its allow-listed allocation/reset/teardown
commands only on a ledger-allocated device in the disposable test environment.

### Native destructive-semantics contract

The source handlers used CLI commands, while the target must use native UDisks
methods. The following choices are therefore explicit API semantics, not
adapter defaults. The UI retains the source form and review step, and the
confirmation text must include the listed irreversible effect.

| Action | Source behaviour to preserve | Native call and fixed choice | Required visible confirmation and fixture assertion |
| --- | --- | --- | --- |
| Create LVM VG | `vgcreate --yes` initializes the selected PVs. | `VolumeGroupCreate`; all selected `BlockDeviceRef`s must still match and native wiping is expected. | Name every selected device and data loss; assert only the ledgered devices gained LVM metadata. |
| Delete LVM VG | `vgremove -f` removes the group without a separate `pvremove` or configuration cleanup. | `VolumeGroup.Delete(wipe=false, tear-down=false)`. | Say LVs are deleted but PV labels and existing configuration are preserved; assert the fixture PV signature remains. |
| Add LVM PV | `vgextend` initializes/adds the selected PV. | `VolumeGroup.AddDevice`; native wiping is expected. | Name the device and data loss; assert only that ledgered device became a PV. |
| Remove LVM PV | `vgreduce` removes an unused PV without wiping it. | `VolumeGroup.RemoveDevice(wipe=false)`. | Say the PV label is preserved and the device must be unused; assert the signature remains. |
| Delete LV | `lvremove -f` removes the selected LV without configuration cleanup. | `LogicalVolume.Delete(tear-down=false)` only where the dependent-volume scope is source-equivalent. | Name the LV, reviewed dependent scope, and preserved configuration; assert exactly the approved scope is absent after refresh, otherwise the control is blocked. |
| Create MD RAID | `mdadm --create --force --run --metadata=0.90` consumes the selected members. | `Manager.MDRaidCreate` with the locked profile below. | Name every member and metadata loss; assert the array is running before reporting success. |
| Delete MD RAID | Source stops the array and attempts to zero its superblock without configuration cleanup. | `MDRaid.Delete(tear-down=false)` destroys RAID metadata on every member; no CLI fallback exists. | State this approved native-semantics difference verbatim, name every affected member, and assert every ledgered member lost RAID metadata. |
| Remove MD member | Source removes membership without a general wipe request. | `MDRaid.RemoveDevice(wipe=false)`. | Name the member and require the native precondition; assert the member signature is preserved and no unledgered device changed. |
| Remove Btrfs member | Source removes membership without a general wipe request. | `Filesystem.BTRFS.RemoveDevice` with an empty options map; it has no wipe option. | Name the member and require the native precondition; assert no unledgered device changed. |
| Btrfs add/resize/label/default | Source invokes the named operation against its mount input. | Resolve the retained form to the selected filesystem and native BTRFS interface. | Name filesystem/member and assert post-refresh FSID, members, label, size, or default ID as applicable. |

Every row is covered by an adapter assertion for the exact method/options and a
disposable-fixture assertion for the stated side effect. A caller may not
invent a `wipe`, `tear-down`, `force`, or options-map value.

## Commit sequence and path ownership

Create these commits in order. Do not combine unrelated changes. A commit may
touch only the listed paths plus direct test files for those paths.

| # | Commit title | Paths |
| --- | --- | --- |
| 1 | docs(plan): specify full lvm semantic graft | docs/plans/2-lvm-refactor/** |
| 2 | feat(logical): add complete logical domain and contracts | crates/storage-types/**, crates/storage-contracts/** |
| 3 | feat(udisks): implement native logical storage backend | crates/storage-udisks/**, crates/storage-sys/** |
| 4 | feat(logical): add operations and application state | src/operations/**, src/state/**, src/message/**, src/update/**, src/app.rs, src/subscriptions/** |
| 5 | feat(logical): port logical controls dialogs and views | src/controls/**, src/views/**, i18n/en/cosmic_ext_storage.ftl |
| 6 | feat(sidebar): port complete asynchronous sidebar loading | src/{app.rs,logging.rs,message/app.rs,models/{load.rs,mod.rs},state/{dialogs.rs,sidebar.rs},subscriptions/app.rs,update/{mod.rs,nav.rs,network.rs},views/{network.rs,sidebar.rs}}, focused tests |
| 7 | test(harness): restore serviceless integration harness and lab | Cargo.toml, justfile, tools/storage-testing/**, resources/lab-specs/**, .github/** |
| 8 | docs: record logical storage support and validation | README.md, docs/plans/2-lvm-refactor/** |

Use the old branch commits only as an ordered behaviour inventory:

| Source commit(s) | Treatment on the new branch |
| --- | --- |
| 4d25b66 | Replace with this plan-set; do not copy its stale architecture assumptions. |
| c8b6631, 06732f9 | Port domain behaviour, mappers, views, and tests; replace service calls with Tasks 2–5. |
| 9094a29, 4b53757, 5b0d8a6 | Port all harness/lab/test behaviour in Task 7; delete service launch/client assumptions and centralize the permitted fixture lifecycle commands. |
| 2c862ff | Port application call-site semantics into the operations façade; do not restore contract client proxies. |
| 93094bd | Port its asynchronous sidebar semantics after the logical state and operations exist. |
| storage-service handlers, policies, client proxies, D-Bus signals | Replace their behaviour through native UDisks2 calls; do not copy implementation or artifacts. |

## Deterministic implementation and phase-gate protocol

Every phase is an implementation boundary, not a declaration that a numbered
task is “complete”. Its gate must prove durable product behaviour that remains
valuable after this graft: public type invariants, native-call translation,
state transitions, rendering decisions, or fixture safety. A broad test filter
such as `cargo test … logical` is supplementary only: Cargo exits successfully
when that filter selects zero tests.

Each code-bearing phase therefore adds the named integration/component test
target listed below. The target contains the named behavioural tests shown in
the third column. Its gate first lists the test names and fails if a required
name is absent, then executes the whole target. Test names describe the
long-lived contract, never the task number, branch name, or whether a port is
finished.

| Phase | Required test target | Required behavioural test names |
| --- | --- | --- |
| 1 | `crates/storage-types/tests/logical_domain_contract.rs` | `stable_ids_ordering_and_capability_precedence`; `block_reference_encoding_is_canonical`; `confirmed_destructive_scope_is_canonical`; `btrfs_default_id_range_is_u32` |
| 2 | `crates/storage-contracts/tests/logical_contract.rs` | `all_actions_have_typed_validation`; `registry_separates_sources_from_executor`; `error_kind_reaches_operation_error` |
| 3 | `crates/storage-udisks/tests/logical_adapter_contract.rs` and `crates/storage-sys/tests/logical_readonly_contract.rs` | `native_action_matrix_uses_documented_options`; `stale_block_reference_never_calls_proxy`; `object_manager_epoch_changes_only_on_topology_events`; `capability_block_reason_precedence_is_stable`; `local_tools_use_stable_readonly_argv_and_order`; `malformed_local_record_fails_source`; `local_tools_reject_mutating_commands` |
| 4 | `tests/logical_operations_contract.rs` and `tests/logical_state_contract.rs` | `topology_merge_has_stable_precedence`; `operation_generation_ignores_late_completion`; `logical_refresh_preserves_selection`; `failed_action_preserves_form_and_topology` |
| 5 | `tests/logical_ui_contract.rs` | `logical_dialogs_preserve_source_defaults`; `blocked_action_is_visible_and_inert`; `confirmation_binds_current_device_reference`; `confirmation_rejects_changed_destructive_scope` |
| 6 | `tests/sidebar_async_contract.rs` | `newer_load_wins`; `event_and_action_refresh_coalesce`; `physical_network_and_logical_loading_do_not_block_startup` |
| 7 | `tools/storage-testing/tests/harness_execution_contract.rs` | `selected_cases_cannot_be_skipped`; `timeout_is_failed`; `fixture_target_and_cleanup_are_enforced` |

Use this portable non-empty-target pattern, substituting the phase package,
target, and required names from the table:

~~~sh
cargo test -p <package> --locked --test <target> -- --list | rg -F '<required_test_name>: test'
cargo test -p <package> --locked --test <target>
~~~

For each Task 0–8 gate, perform and record a manual plan-to-code review in the
phase record in `validation.md`: inspect the scoped diff; trace one normal and
one failure/edge path from the relevant specification clause through the public
boundary to its test; compare every touched source/reconciliation row; and
record the commit SHA, files/ranges reviewed, evidence, result, and reviewer.
Manual review supplements tests—it cannot waive a failed, missing, or empty
test target. Tasks 0 and 8 are documentation/preflight phases and instead
record their exact repository/documentation evidence.

## Task 0 — create and verify the integration branch

Run these commands before any code change:

~~~sh
git fetch origin --prune
git switch --create 079-lvm-refactor origin/main
git rev-parse HEAD
git merge-base HEAD origin/079-lvm-support
git log --reverse --format='%H %s' HEAD..origin/079-lvm-support
git diff --find-renames --name-status 3f8c340..HEAD
git diff --find-renames --name-status 3f8c340..origin/079-lvm-support
cargo metadata --no-deps --format-version=1
~~~

Record the target SHA, the merge-base
3f8c340e8a6cc17ea854ef244b0c3b3bb1028128, and all eight source commits in
the implementation notes. Confirm the initial workspace contains the root
application plus five published libraries and no service package.

Before changing an implementation file, create review-only exports of its
common-ancestor, current-main, and feature source content and record their
SHAs. Complete its `reconciliation.md` row before selecting a transplant
strategy. For a feature-only mapped range, transplant the source content before
editing it; for an overlap, preserve or compose the main range exactly as the
reconciliation row requires. Do not begin with an empty new module or a
reimplementation from memory. Keep source comments, helper/function names,
message variants, state fields, test names, and ordering. Update
source-fidelity.md's ledger for every non-verbatim feature range and cite the
reconciliation row for every main-preservation range.

Do not run rebase, merge, cherry-pick, or checkout against the feature branch.
Those operations preserve source layout and service dependencies rather than
the requested behaviour.

**Gate**

~~~sh
git diff --name-only origin/main...HEAD
git diff --check
~~~

At this gate, only docs/plans/2-lvm-refactor is changed.
The known overlap inventory in reconciliation.md has a row for every mapped
file that will be changed in a later task.

**Manual plan-to-code review**

Verify the recorded branch, merge-base, source-commit inventory, normalized
overlap inventory, and root-path dispositions against the actual refs. Confirm
the scoped diff contains no implementation file and enter that evidence in the
Task 0 phase record.

## Task 1 — create the complete logical domain

### Files and exports

Create crates/storage-types/src/logical.rs. Export it through
crates/storage-types/src/lib.rs. Keep crates/storage-types/src/lvm.rs source
compatible; it may become an adapter input but must not remain the UI's logical
model. Add logical_storage to StorageBackendCapabilities in disk.rs.

Every logical value derives Debug, Clone, PartialEq, Eq, Serialize, and
Deserialize. Entity/member IDs, finite enums used as map keys, and operation
kinds also derive PartialOrd, Ord, and Hash.

### Domain values

Define the following public values. These retain every display/state field
consumed by the old branch while replacing ambiguous strings at API boundaries.

~~~rust
pub struct LogicalEntityId(pub String);
pub struct LogicalMemberId(pub String);
/// Live lookup key only; never sufficient to authorize a mutation.
pub struct BlockDeviceId(pub String);
/// Opaque, versioned encoding of immutable device identity observed by the adapter.
pub struct BlockDeviceFingerprint(pub String);
/// A selected device bound to its observed identity and object-manager epoch.
pub struct BlockDeviceRef {
    pub id: BlockDeviceId,
    pub fingerprint: BlockDeviceFingerprint,
    pub observed_generation: u64,
}
/// Collateral objects/devices the user reviewed for a destructive native call.
pub struct ConfirmedDestructiveScope {
    pub entity_ids: Vec<LogicalEntityId>,
    pub device_refs: Vec<BlockDeviceRef>,
}

pub enum LogicalEntityKind {
    LvmVolumeGroup,
    LvmLogicalVolume,
    LvmPhysicalVolume,
    MdRaidArray,
    MdRaidMember,
    BtrfsFilesystem,
    BtrfsDevice,
    BtrfsSubvolume,
}

pub enum LogicalOperation {
    Create,
    Delete,
    Resize,
    AddMember,
    RemoveMember,
    Activate,
    Deactivate,
    Start,
    Stop,
    Check,
    Repair,
    SetLabel,
    SetDefaultSubvolume,
}

pub struct LogicalBlockedReason {
    pub operation: LogicalOperation,
    pub reason: String,
}

pub struct LogicalCapabilities {
    pub supported: Vec<LogicalOperation>,
    pub blocked: Vec<LogicalBlockedReason>,
}

pub struct ProgressRatio(/* ten-thousandths, clamped to 0..=1 */);

pub struct LogicalMember {
    pub id: LogicalMemberId,
    pub kind: LogicalEntityKind,
    pub name: String,
    pub device_ref: Option<BlockDeviceRef>,
    pub device_path: Option<String>,
    pub role: Option<String>,
    pub state: Option<String>,
    pub size_bytes: Option<u64>,
}

pub struct LogicalEntity {
    pub id: LogicalEntityId,
    pub kind: LogicalEntityKind,
    pub name: String,
    pub uuid: Option<String>,
    pub parent_id: Option<LogicalEntityId>,
    pub device_path: Option<String>,
    pub size_bytes: u64,
    pub used_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    pub health_status: Option<String>,
    pub progress_fraction: Option<ProgressRatio>,
    pub members: Vec<LogicalMember>,
    pub capabilities: LogicalCapabilities,
    pub metadata: BTreeMap<String, String>,
}

pub enum LogicalSource {
    Udisks,
    LocalTools,
}

pub enum LogicalSourceAvailability {
    Available,
    Unavailable { reason: String },
    Failed { reason: String },
}

pub struct LogicalSourceStatus {
    pub source: LogicalSource,
    pub availability: LogicalSourceAvailability,
}

pub struct LogicalTopology {
    pub entities: Vec<LogicalEntity>,
    pub sources: Vec<LogicalSourceStatus>,
}
~~~

Retain the old inferred_used_bytes, inferred_free_bytes,
LogicalAggregateSummary, and summarize_entities behaviour exactly, with ID
newtypes substituted where a parent/member identifier is used. `LogicalTopology`
construction rejects duplicate entity IDs or duplicate source statuses, sorts
source statuses by `LogicalSource` discriminant, and requires entity emission
to use the ordering rule below; callers do not receive an unordered map.

### Identity, validation, and ordering

`BlockDeviceId` is an opaque `block:<major>:<minor>` lookup key made from the
live UDisks `Block.DeviceNumber`, not a display name, `/dev` path, or D-Bus
object path. It is not stable: a kernel may reuse the same major/minor pair
after removal. Every action therefore carries `BlockDeviceRef`. Its fingerprint
is a versioned, length-delimited canonical encoding of exactly one first
available identity tier—never a display string and never a hash-map iteration:

1. loop backing-file `st_dev` and `st_ino` for a loop device;
2. normalized lowercase partition UUID;
3. normalized lowercase filesystem UUID plus filesystem type; or
4. normalized lowercase drive WWN plus serial.

The encoding is `v1` followed by NUL-delimited, UTF-8 fields in the stated
order; an absent or empty field makes that tier unavailable. Do not fall back to
model, vendor, size, `/dev` path, or a D-Bus object path. If no complete tier
exists, no mutation reference is issued. The reference also carries the
object-manager generation observed during discovery.

`BlockDeviceId` is serialized only as `block:<major>:<minor>`, where both
components are canonical unsigned decimal (no sign or leading zero except
`0`). A `BlockDeviceFingerprint` validates the `v1` prefix, known tier, exact
field count, and non-empty fields before it can enter a `BlockDeviceRef`.
`observed_generation` may be zero for the initial cache epoch. These are
structural validation rules only; matching the reference to a current device is
the Task 3 resolver's responsibility.

`ConfirmedDestructiveScope` is a sorted, duplicate-free representation of
collateral native effects, not a display-only confirmation flag. It contains
only affected children/members, never the primary entity targeted by the action:
VG deletion lists its LVs, LV deletion lists dependent snapshots/thin volumes,
and MD deletion lists member device references. The adapter recomputes this
scope from its final snapshot and refuses the action with `Conflict` if it is
not exactly equal to the confirmed scope.

The resolver accepts a reference only when one current candidate has the same
device number *and* fingerprint, and the final immediately-before-call snapshot
still matches it. A device event/generation change, duplicate candidate,
missing strong fingerprint, or same-number/different-fingerprint replacement is
`Conflict`; it never falls through to the new device. Where no strong
fingerprint can be produced, display remains readable but the action is blocked
until the user refreshes and explicitly reselects a current device. Neither the
fingerprint nor a D-Bus object path is rendered in UI text.

Use these logical IDs:

| Entity | ID |
| --- | --- |
| LVM volume group | lvm-vg:<VG UUID> |
| LVM logical volume | lvm-lv:<VG UUID>:<LV name> |
| LVM physical volume | lvm-pv:<VG UUID>:<BlockDeviceFingerprint> |
| MD RAID array | mdraid:<UUID>, falling back to mdraid:<BlockDeviceFingerprint> only when UUID is absent |
| MD RAID member | mdraid-member:<BlockDeviceFingerprint> |
| Btrfs filesystem | btrfs:<FSID> |
| Btrfs device | btrfs-device:<FSID>:<devid> |
| Btrfs subvolume | btrfs-subvolume:<FSID>:<subvolume ID> |

An adapter may use the documented fallback only when UDisks omits a UUID. It
must record that choice in metadata, never coalesce entities by name. Do not use
the LogicalVolume UUID as the public ID: UDisks documents that it may change;
the VG UUID plus LV name is unique for the supported action set and stays stable
across normal refreshes. Preserve source display ordering exactly: root and
child entities sort by case-sensitive name and then stable ID; member rows keep
their source discovery order. Where the native snapshot is unordered, establish
the old mapper's order before exposing members. Define and test capability
normalization without changing that display ordering: every blocked operation
takes precedence over supported.

### Required domain tests

Add unit tests for serde, progress clamping, used/free inference, aggregate
summary, canonical ID construction, deterministic sorting, parent/member
linking, capability precedence, and malformed action input. They must cover
all eight entity kinds and every LogicalOperation variant. Add reference tests
for every fingerprint tier, malformed/missing reference rejection, and canonical
major/minor serialization, plus sorted/unique destructive scopes. The Task 3
adapter contract, not this pure domain crate, proves that a same-major/minor
replacement returns `Conflict` before any proxy method is called.

**Gate**

~~~sh
cargo test -p storage-types --locked --test logical_domain_contract -- --list | rg -F 'stable_ids_ordering_and_capability_precedence: test'
cargo test -p storage-types --locked --test logical_domain_contract -- --list | rg -F 'block_reference_encoding_is_canonical: test'
cargo test -p storage-types --locked --test logical_domain_contract -- --list | rg -F 'confirmed_destructive_scope_is_canonical: test'
cargo test -p storage-types --locked --test logical_domain_contract -- --list | rg -F 'btrfs_default_id_range_is_u32: test'
cargo test -p storage-types --locked --test logical_domain_contract
cargo fmt --all -- --check
~~~

**Manual plan-to-code review**

Inspect every public logical type/export and its serde representation against
Task 1 and the specification's domain requirements. Walk the fixed fingerprint
tier table and canonical major/minor encoding by hand; verify no path, display
label, hash iteration, or unversioned fallback can determine an ID, fingerprint,
or display order. Record the reviewed ranges and test output in the Task 1
phase record.

## Task 2 — define typed discovery and mutation contracts

Create crates/storage-contracts/src/traits/logical.rs. Re-export its traits
from traits/mod.rs and lib.rs. `LogicalTopologySource` and
`LogicalOperations` are independent object-safe contracts; do not add either
as a supertrait of `BlockStorageBackend`. One Udisks adapter is both the native
logical source and the native logical action executor, but the local-tools
adapter is a second discovery-only source and cannot implement the block
backend.

### Topology contract

~~~rust
#[async_trait]
pub trait LogicalTopologySource: Send + Sync {
    fn logical_source(&self) -> LogicalSource;
    fn logical_availability(&self) -> LogicalSourceAvailability;
    async fn list_logical_entities(&self) -> Result<Vec<LogicalEntity>, StorageError>;
}
~~~

LogicalSource is Udisks or LocalTools. LocalTools is discovery-only. A source
unavailable because a plugin/executable is absent returns an availability
record; an invocation failure returns StorageError.

### Registration contract

Extend `src/operations::BackendRegistry`, not `BlockStorageBackend`, with:

~~~rust
pub logical_topology_sources: Vec<Arc<dyn LogicalTopologySource>>,
pub logical_operations: Arc<dyn LogicalOperations>,
~~~

`StorageOperations::new` constructs one `Arc<UdisksBackend>`, registers clones
of that same instance as `block`, the UDisks topology source, and the logical
operation executor, then registers one `LocalLogicalTopologySource` from
`storage-sys`. The local source is registered even when an executable is absent
so it can report typed unavailability. Only the composition root sees those
concrete adapters. Test constructors inject the same two source slots and one
executor with fakes.

### Action contract

Use one exhaustive action enum and one execution method, so the UI cannot omit
a branch action or manufacture an unsupported textual command:

~~~rust
pub enum LogicalAction {
    CreateLvmVolumeGroup { name: String, devices: Vec<BlockDeviceRef> },
    DeleteLvmVolumeGroup {
        volume_group: LogicalEntityId,
        pv_label_policy: LvmWipePolicy,
        configuration_policy: ConfigurationCleanupPolicy,
        confirmed_scope: ConfirmedDestructiveScope,
    },
    AddLvmPhysicalVolume { volume_group: LogicalEntityId, device: BlockDeviceRef },
    RemoveLvmPhysicalVolume {
        volume_group: LogicalEntityId,
        device: BlockDeviceRef,
        pv_label_policy: LvmWipePolicy,
    },
    CreateLvmLogicalVolume { volume_group: LogicalEntityId, name: String, size_bytes: u64 },
    DeleteLvmLogicalVolume {
        logical_volume: LogicalEntityId,
        configuration_policy: ConfigurationCleanupPolicy,
        confirmed_scope: ConfirmedDestructiveScope,
    },
    ResizeLvmLogicalVolume { logical_volume: LogicalEntityId, size_bytes: u64 },
    ActivateLvmLogicalVolume { logical_volume: LogicalEntityId },
    DeactivateLvmLogicalVolume { logical_volume: LogicalEntityId },

    CreateMdRaidArray {
        name: MdRaidName,
        level: MdRaidLevel,
        devices: Vec<BlockDeviceRef>,
        profile: MdRaidCreateProfile,
    },
    DeleteMdRaidArray {
        array: LogicalEntityId,
        configuration_policy: ConfigurationCleanupPolicy,
        confirmed_scope: ConfirmedDestructiveScope,
    },
    StartMdRaidArray { array: LogicalEntityId },
    StopMdRaidArray { array: LogicalEntityId },
    AddMdRaidMember { array: LogicalEntityId, device: BlockDeviceRef },
    RemoveMdRaidMember {
        array: LogicalEntityId,
        device: BlockDeviceRef,
        member_signature_policy: MdRaidMemberWipePolicy,
    },
    RequestMdRaidSync { array: LogicalEntityId, action: MdRaidSyncAction },

    AddBtrfsDevice { filesystem: LogicalEntityId, device: BlockDeviceRef },
    RemoveBtrfsDevice { filesystem: LogicalEntityId, device: BlockDeviceRef },
    ResizeBtrfsFilesystem {
        filesystem: LogicalEntityId,
        request: BtrfsResizeRequest,
    },
    SetBtrfsLabel { filesystem: LogicalEntityId, label: String },
    SetBtrfsDefaultSubvolume {
        filesystem: LogicalEntityId,
        subvolume_id: NonZeroU32,
    },
}

/// Source operations preserve labels rather than wiping them.
pub enum LvmWipePolicy {
    Preserve,
}

/// Source commands do not remove tracked fstab/crypttab configuration.
pub enum ConfigurationCleanupPolicy {
    Preserve,
}

/// Source MD member removal does not wipe known signatures.
pub enum MdRaidMemberWipePolicy {
    Preserve,
}

#[async_trait]
pub trait LogicalOperations: Send + Sync {
    async fn execute_logical_action(&self, action: LogicalAction)
        -> Result<LogicalActionOutcome, StorageError>;
}
~~~

The closed preserve-only policy enums make native destructive choices part of
the request contract: VG deletion maps to `wipe=false, tear-down=false`; PV
removal to `wipe=false`; LV and MD deletion to `tear-down=false`; and MD member
removal to `wipe=false`. The adapter rejects no alternative because no
alternative variant exists; Btrfs member removal has an empty options map.
Every delete action additionally carries a `ConfirmedDestructiveScope`; local
validation requires its sorted/unique invariant before discovery, and final
adapter resolution requires an exact fresh-snapshot match before the native
method.

`MdRaidName` is created exactly as the source's `md_name_from_device` helper:
split the original text on `/`, retain the final component, then trim that
component. Thus `/dev/md0` becomes `md0`, `/dev/md/name` becomes `name`, and
`md127` remains `md127`; a trailing slash or whitespace-only final component
is invalid. Apply the ordinary name validation only to that normalized result.
Do not add a narrower `/dev/md…` grammar or reinterpret the retained source
field. `MdRaidLevel` is a closed enum; unknown source text remains visible but
has a disabled Apply control with the reason
“No audited UDisks profile exists for this source RAID level.”
`MdRaidSyncAction` is exactly Check or Repair.

`MdRaidCreateProfile` is not a UI command fragment. It is the following locked
translation of the branch's `mdadm --create --force --run --metadata=0.90`
behaviour. `version` is encoded as the D-Bus `ay` byte sequence `b"0.90"`
without a NUL terminator; no string-valued option is valid. `chunk_bytes` is
the current `mdadm --create` default where source supplied none.

| Source level text | MdRaidLevel | Minimum members | chunk_bytes | metadata version | Postcondition |
| --- | --- | --- | --- | --- | --- |
| `raid0` | Raid0 | 2 | 524288 | `0.90` | Native resulting array is running. |
| `raid1` | Raid1 | 2 | 0 | `0.90` | Native resulting array is running. |
| `raid4` | Raid4 | 3 | 524288 | `0.90` | Native resulting array is running. |
| `raid5` | Raid5 | 3 | 524288 | `0.90` | Native resulting array is running. |
| `raid6` | Raid6 | 4 | 524288 | `0.90` | Native resulting array is running. |
| `raid10` | Raid10 | 2 | 524288 | `0.90` | Native resulting array is running. |

The adapter additionally rejects more than 28 members or a component size that
would violate the `0.90` metadata limit, with an actionable blocked reason.
If the installed UDisks cannot accept the exact level, chunk, or `ay` version
option (including versions before 2.11), Create is visibly blocked rather than
silently using a different default. A successful `MDRaidCreate` is not reported
until its returned object resolves to a running MDRaid object; a non-running
result is `Conflict`, not a concealed extra Start call.

`BtrfsResizeRequest` is a closed enum: `AbsoluteBytes(u64)`, `Maximum`,
`GrowBy(u64)`, or `ShrinkBy(u64)`. The source size-spec field and its default
`max` remain visible. Only a non-zero absolute request is dispatched to
`Filesystem.BTRFS.Resize`; a source syntax which has no verified native
equivalent remains displayed and is disabled with its exact reason. It is never
passed as a string to a command or D-Bus call.

LogicalActionOutcome includes the action, affected entity IDs when known, and
optional native job ID/progress. It must not expose a D-Bus object path.

Validate before adapter dispatch:

1. Names are trimmed, non-empty, contain no slash or NUL, and satisfy the
   existing LVM/MD length rules. The source MD array-device form is normalised
   to `MdRaidName` before this validation.
2. A create list has the UDisks-required minimum non-duplicate canonical block
   IDs for its level.
3. LV resize and `BtrfsResizeRequest::AbsoluteBytes` are non-zero absolute byte
   counts. Other Btrfs request variants are parsed from the retained source
   field but cannot dispatch without a verified native mapping.
4. IDs have the prefix appropriate for the action. Every `BlockDeviceRef` has
   canonical decimal major/minor form, a structurally valid versioned
   fingerprint, and its observed generation; a malformed reference is
   `InvalidInput` before discovery/resolution.
5. A Btrfs label has no NUL and a default subvolume ID is within
   `1..=u32::MAX`. Parse the retained source `u64` text before conversion to
   `NonZeroU32`; an out-of-range value remains in the field and reports the
   exact UDisks 32-bit limitation without dispatch.
6. A `ConfirmedDestructiveScope` is sorted and duplicate-free, has no primary
   target entity, and uses only structurally valid block references. The final
   native resolver, not the UI, verifies its fresh-snapshot equality.

The contract does not itself decide device free/busy eligibility; discovery
capabilities and UDisks remain authoritative.

### Contract error mapping

Add `StorageErrorKind::Other` for uncategorised remote/native failures while
retaining `Internal` for local adapter defects. Add matching
`OperationError::{Busy, Conflict, Other}` variants; map all other `Internal`
errors to the existing generic failed presentation. Preserve these distinctions:
NotFound, InvalidInput, Unsupported, Unavailable, PermissionDenied, Busy,
Conflict, and Other. Dialog state carries the structured kind plus safe detail
so the UI can render an action-specific message without exposing raw D-Bus
object paths.

Add trait-object mock tests for every enum arm, validation failure, source
availability, and app-visible error mapping.

**Gate**

~~~sh
cargo test -p storage-contracts --locked --test logical_contract -- --list | rg -F 'all_actions_have_typed_validation: test'
cargo test -p storage-contracts --locked --test logical_contract -- --list | rg -F 'registry_separates_sources_from_executor: test'
cargo test -p storage-contracts --locked --test logical_contract -- --list | rg -F 'error_kind_reaches_operation_error: test'
cargo test -p storage-contracts --locked --test logical_contract
cargo test -p storage-types --locked --test logical_domain_contract
~~~

**Manual plan-to-code review**

Match every `LogicalAction` variant one-for-one with the replacement matrix and
verify that all device-bearing variants use `BlockDeviceRef`. Trace the source
MD normalization examples through validation, then trace one error of each
public kind into the app-facing error type. Confirm the registry permits a
source without mutation authority and exactly one mutation executor. Record the
reviewed ranges and test output in the Task 2 phase record.

## Task 3 — implement the native UDisks logical adapter

### Module layout

Create this exact logical module family under
crates/storage-udisks/src/logical:

~~~text
logical/
  mod.rs
  proxy.rs
  discover.rs
  resolve.rs
  operations.rs
  error.rs
  tests.rs
~~~

Retain the existing lvm/list.rs only as a read-only enrichment input until its
facts are available from the UDisks object manager. It must not add mutation
commands. Place all adapter-only zbus proxy declarations in logical/proxy.rs;
do not export them from storage-udisks.

Reuse the existing UdisksBackend DiskManager connection and object-manager
snapshot. Do not open a second bus connection per logical request.

`UdisksBackend` owns one `AtomicU64` object-manager epoch. Initialize it before
the first published logical snapshot and increment it exactly once after the
backend applies each UDisks ObjectManager `InterfacesAdded` or
`InterfacesRemoved` event to its cache. A read of an unchanged cache does not
increment the epoch. Every topology snapshot and every `BlockDeviceRef` carries
that epoch. Native ObjectManager events are permitted for cache coherence; no
project-owned logical-topology signal is introduced.

### Private proxies and resolution

In proxy.rs declare private proxies for:

- org.freedesktop.UDisks2.Manager.LVM2;
- org.freedesktop.UDisks2.VolumeGroup;
- org.freedesktop.UDisks2.LogicalVolume; and
- org.freedesktop.UDisks2.Filesystem.BTRFS.

Use the generated UDisks ManagerProxy and MDRaidProxy for Manager.MDRaidCreate
and MDRaid operations already supported by the udisks2 crate. Pass an empty,
typed a{sv} options map unless the typed request has a documented native
option. The source-compatible MD profile is such an exception: after a UDisks
2.11-or-newer capability preflight, pass exactly the D-Bus variant `ay`
containing `b"0.90"` (without a terminating NUL) as its `version` option.
Never substitute a string, omit the option, or create an array with a different
profile. The Btrfs proxy must cover discovery as well as mutation: use
`GetSubvolumes` and `GetDefaultSubvolumeID` to construct the source-visible
subvolume/default state when the interface is present.

resolve.rs is the only place that converts LogicalEntityId and `BlockDeviceRef`
to a current object-manager object path. `BlockDeviceId` alone is a lookup and
display key; it is never sufficient authorization to mutate a block device. A
resolution must:

1. fetch one snapshot;
2. index block paths by `BlockDeviceId` **and** immutable fingerprint, plus LVM
   UUIDs, MD UUIDs, Btrfs FSIDs, and subvolume IDs;
3. require the reference fingerprint to match exactly and record the snapshot
   generation used for every resolved block;
4. apply this fixed resolution precedence: an invalid request is `InvalidInput`;
   an absent logical target or block ID is `NotFound`; a present block with a
   missing, mismatched, or ambiguous fingerprint is `Conflict`; a missing
   required native interface is `Unsupported`; and a mismatched observed epoch
   is `Conflict`;
5. return the same snapshot to the caller for capability evaluation.

Immediately before the native method, obtain a fresh snapshot and resolve every
`BlockDeviceRef` **and target entity** again. The fresh fingerprint, epoch,
required interface, and capability must still match the reference and resolved
target; otherwise return the fixed error above before any proxy method is
called. Never cache object paths across an operation. Thus a hot-unplug/replug
that reuses a major:minor number can never redirect a queued operation to the
replacement device.

### Discovery and capability construction

discover.rs creates root and child entities for the full old topology. Populate
all old display fields, parent relationships, members, metadata, health, and
progress when native properties expose them. Construct the stable IDs from Task
1, including `Block.DeviceNumber` for the live block lookup and the immutable
fingerprint/observed-generation pair for mutation references; do not store or
export an object path as a domain ID. When Filesystem.BTRFS is available,
preserve a Btrfs subvolume entity from `GetSubvolumes` and the filesystem's
current default ID from `GetDefaultSubvolumeID`; each subvolume remains a child
of its filesystem. If the interface cannot expose either fact, retain the
source field as unavailable rather than inventing a value.

Build each entity's LogicalCapabilities from the interface actually present,
the entity state, and input prerequisites. Examples:

- an active LV supports Deactivate; an inactive LV supports Activate;
- a VolumeGroup with the native interface supports Delete whether or not it has
  LVs; the currently discovered LV list is captured for the destructive
  confirmation because native deletion removes those LVs too. Do not disable
  this source operation merely because children exist;
- an MD RAID array exposes Start/Stop based on its running state and
  Check/Repair when RequestSyncAction is present;
- Btrfs operations are blocked with a plugin-unavailable reason if
  Filesystem.BTRFS is absent; and
- SetDefaultSubvolume is additionally blocked unless the discovered Btrfs
  interface exposes `SetDefaultSubvolumeID` (UDisks 2.11 or newer);
- Create MD RAID is blocked unless the selected `MdRaidCreateProfile` is
  supported by the running UDisks version and its requested level/member set;
- an operation with an unresolved required member is blocked with a concrete
  not-found/busy reason.

The capability contract is exhaustive; no control infers availability from its
label or display path:

| Operations | Required discovered interface/state/prerequisite |
| --- | --- |
| Create LVM VG | Manager.LVM2 and one or more validated, distinct, currently resolvable block references. |
| Delete VG; add/remove PV; create LV | VolumeGroup; the relevant validated block reference for add/remove; non-zero size for create. Delete remains available with discovered LVs and lists them in confirmation. |
| Delete/resize LV | LogicalVolume; non-zero resize. |
| Activate / deactivate LV | LogicalVolume and respectively inactive / active `Active` property. |
| Create MD RAID | Manager, UDisks 2.11+, an enabled locked profile, its minimum distinct current block references, and supported `ay` version option. |
| Delete MD RAID | MDRaid. |
| Start / stop MD RAID | MDRaid and respectively `Running=false` / `Running=true`. |
| Add MD member | MDRaid plus a validated current block reference. |
| Remove MD member | MDRaid plus a validated current block reference present in `ActiveDevices`. |
| Check / repair MD RAID | MDRaid, `Running=true`, and RequestSyncAction support. |
| Add/remove Btrfs device | Filesystem.BTRFS plus a validated current block reference. |
| Resize / label Btrfs | Filesystem.BTRFS plus respectively dispatchable absolute size / valid label. |
| Set Btrfs default subvolume | Filesystem.BTRFS with the 2.11 method, a discovered subvolume belonging to the selected filesystem, and ID `1..=u32::MAX`. |

For VG/LV deletion, discovery must enumerate the exact native collateral set
(LVs and dependent snapshots/thin volumes respectively). A delete is enabled
only when its native collateral semantics are proven equivalent to the source
fixture for that entity shape; otherwise keep the source control visible but
block it with “Native delete would remove unsupported dependent volumes.” MD
delete remains enabled with its separately documented all-member-metadata
semantic difference, but always requires a confirmation scope containing every
current member reference.

Use a blocked reason for unavailable native functionality rather than removing
the control. The capability reducer is pure: it receives a snapshot entity,
interface set, native version, and validated input prerequisites; emits each
operation once; sorts operations by `LogicalOperation` discriminant; and chooses
the first applicable blocked reason in this order: unavailable plugin/version,
missing interface, unresolved member, incompatible entity state, then busy
native state. A local-tools-only entity has every mutation blocked with
“Not discovered by UDisks”; it never acquires capabilities from parsed CLI
output. Do not claim a CLI fallback capability.

### Dispatch sequence

operations.rs implements `LogicalOperations` for `UdisksBackend`; `mod.rs`
implements `LogicalTopologySource` for that same backend. The local source
implements only `LogicalTopologySource` in `storage-sys`.

The native dispatch sequence is:

1. validate the typed action from Task 2;
2. get one resolution snapshot;
3. resolve the target entity and every `BlockDeviceRef` against that snapshot;
4. ensure the action is allowed by the target capability;
5. immediately re-resolve the target and every block reference against a fresh
   snapshot, re-evaluate capability, recompute any destructive collateral
   scope, and return the fixed error if an observed epoch, fingerprint,
   interface, capability, or confirmed scope changed;
6. call the exact native method in the replacement matrix;
7. map native job/object result into LogicalActionOutcome;
8. return success only after the D-Bus method/job reports success.

For calls returning a job object, wait for the native job completion using the
existing UDisks job helper pattern; do not report success merely because the
job was created. Return native cancellation/failure as the mapped StorageError.

For `CreateMdRaidArray`, test every enabled `MdRaidCreateProfile` against the
running UDisks version before exposing it: name normalisation, level, member
minimum, `chunk_bytes`, and metadata `version` option must all match the source
behaviour. The test asserts the exact `ay b"0.90"` option, every row in the
locked profile table, and a running returned array; it never supplies a plain
string or compensates for a stopped result with an implicit Start. For Btrfs
resize, dispatch only `AbsoluteBytes`; retain `Maximum`,
`GrowBy`, and `ShrinkBy` as typed, explicitly blocked source form values until
a native mapping is documented and tested.

Map error names consistently:

| Native condition | Storage error |
| --- | --- |
| missing object/interface | NotFound or Unsupported |
| service/plugin absent | Unavailable |
| NotAuthorized/AccessDenied | PermissionDenied |
| device busy/in use | Busy |
| invalid native argument | InvalidInput |
| conflicting device/state change | Conflict |
| all other D-Bus/job failure | Other |

Cover transient disappearance with a user-visible Conflict/NotFound result and
leave the prior UI topology intact until the scheduled refresh completes.

### Local-tools source

Create or retain a read-only `LocalLogicalTopologySource` in storage-sys. Inject
its command runner in tests and construct it once at the application composition
root. It may query lvs, vgs, pvs, mdadm detail, and btrfs show/list facts that
UDisks does not provide. Its allow-list contains only those read operations. Any
mutating subcommand, shell string, shell interpreter, privilege elevation, or
write to sysfs is forbidden by unit test.

The allow-list fixes argument order and discovery ordering: LVM queries use
`--noheadings`, `--units b`, `--nosuffix`, their fixed output columns,
tab separator, and `--sort` `vg_name`, `vg_name,lv_name`, or `vg_name,pv_name`
for `vgs`, `lvs`, and `pvs` respectively. After parsing `mdadm` and Btrfs
output, sort arrays/filesystems by stable entity ID and locally added members by
`LogicalMemberId`; preserve that order in the result. A nonblank malformed
record is a source failure with tool name and 1-based record number—never a
silently dropped partial result. A missing executable/plugin returns the typed
Unavailable status rather than an empty successful topology. Unit tests assert
the exact read-only argv arrays, ordering, malformed-record error, and absence
of mutating argv.

Task 4's application façade performs the cross-source merge. This phase makes
LocalTools output deterministic on its own; no LocalTools entity has an
action-capable mutation result.

Each source reports only its own availability/result. Task 4's façade applies
the deterministic one-/two-source fallback and status policy; neither adapter
silently substitutes or upgrades the other source. This permits useful
read-only enrichment without allowing an optional CLI probe to affect mutation
authority.

### Required adapter tests

Use fake object-manager snapshots and proxy seams. Test every replacement-matrix
action for exact target resolution, method selection, options map, returned job
handling, and error mapping. Test discovery for every entity kind, parent and
member links, plugin absence, capability blocking, local parser malformed
output, and no-mutating-command enforcement. Include
native `wipe=false, tear-down=false` assertions for LVM VG deletion,
`wipe=false` for PV/MD-member removal, and `tear-down=false` for LV/MD deletion;
the complete MD profile table and its `ay` variant; Btrfs subvolume/default
discovery and the `u32` default-ID boundary; and a same-major:minor replacement
fixture that returns Conflict before a proxy call. Assert that an unchanged
cache read does not advance the object-manager epoch, each applied add/remove
event advances it once, and the capability reducer's documented blocked-reason
precedence is stable for equivalent snapshots. Exercise every row of the
capability table in both its enabled state and its first blocked state. Include
a plain LV, LV with non-thin snapshots, and thin-pool fixture to prove the
source/native deletion-scope decision; any unsupported cascade is visibly
blocked rather than submitted.

**Gate**

~~~sh
cargo test -p storage-udisks --locked --test logical_adapter_contract -- --list | rg -F 'native_action_matrix_uses_documented_options: test'
cargo test -p storage-udisks --locked --test logical_adapter_contract -- --list | rg -F 'stale_block_reference_never_calls_proxy: test'
cargo test -p storage-udisks --locked --test logical_adapter_contract -- --list | rg -F 'object_manager_epoch_changes_only_on_topology_events: test'
cargo test -p storage-udisks --locked --test logical_adapter_contract -- --list | rg -F 'capability_block_reason_precedence_is_stable: test'
cargo test -p storage-udisks --locked --test logical_adapter_contract
cargo test -p cosmic-ext-storage-storage-sys --locked --test logical_readonly_contract -- --list | rg -F 'local_tools_use_stable_readonly_argv_and_order: test'
cargo test -p cosmic-ext-storage-storage-sys --locked --test logical_readonly_contract -- --list | rg -F 'malformed_local_record_fails_source: test'
cargo test -p cosmic-ext-storage-storage-sys --locked --test logical_readonly_contract -- --list | rg -F 'local_tools_reject_mutating_commands: test'
cargo test -p cosmic-ext-storage-storage-sys --locked --test logical_readonly_contract
cargo clippy -p storage-udisks -p cosmic-ext-storage-storage-sys --all-features --locked
~~~

**Manual plan-to-code review**

Walk every replacement-matrix row from `LogicalAction` through the final
fresh-snapshot resolution to its exact proxy method/options and completion
condition. Inspect the epoch increment path, fixed error precedence, capability
reducer, and local parser ordering against Tasks 1–3; then replay the fake
stale-reference scenario. Confirm proxy types remain private and no CLI
mutation path exists. Record the reviewed ranges and test output in the Task 3
phase record.

## Task 4 — add the application operation façade and logical state

### Operations

Create src/operations/logical.rs and export it from src/operations/mod.rs.
It contains the only app-facing calls to LogicalTopologySource and
LogicalOperations.

Provide these façade methods:

~~~rust
async fn load_logical_topology(&self) -> Result<LogicalTopology, OperationError>;
async fn execute_logical_action(
    &self,
    action: LogicalAction,
) -> Result<LogicalActionOutcome, OperationError>;
~~~

`load_logical_topology` reads `registry.logical_topology_sources` in registered
order and applies this deterministic UDisks-over-local merge policy:

1. The composition root registers the fixed source order `Udisks`, then
   `LocalTools`; duplicate or unknown registrations are a construction error.
   Sources may be queried concurrently, but results are consumed only in this
   declared order.
2. Form the sorted union of entity IDs with a
   `BTreeMap<LogicalEntityId, LogicalEntity>`. For matching IDs, UDisks owns
   identity, parent, state, health, capabilities, and members. LocalTools fills
   only an `Option::None` display/usage field and metadata key absent from the
   UDisks map; empty values are present and are not overwritten.
3. Preserve UDisks member order. Append only LocalTools members whose ID is
   absent, in the local parser's documented deterministic order. Emit
   entities by Task 1's case-sensitive name-then-ID order and source statuses
   by `LogicalSource` discriminant.
4. A LocalTools-only entity is displayable with every mutation blocked as “Not
   discovered by UDisks”. If UDisks is unavailable or its list invocation
   fails, a successful LocalTools result remains displayable but every mutation
   is blocked with the exact UDisks unavailable/failed reason. If neither
   source supplies topology, return the UDisks error when present, otherwise
   the LocalTools error. Never derive an executable capability from local CLI
   output.

`execute_logical_action` calls only `registry.logical_operations`. The façade
performs no parsing, D-Bus access, command execution, dialog validation, or
privilege decisions. The update layer schedules exactly one physical refresh and
one logical refresh after a successful action. A failed action keeps the old
logical topology and returns a mapped error/status; it must not clear selection.

### State

Create src/state/logical.rs from the feature-only source ranges after completing
the corresponding `reconciliation.md` rows, then export it. Keep the old
LogicalState fields and semantics byte-for-byte except for documented
current-main imports, the narrow service-to-façade result boundary, and these
additive current-main fields:

- canonical entity list/index;
- selected LogicalEntityId;
- selected detail tab: Overview, Members, Operations, Btrfs;
- pending LogicalAction and pending entity;
- monotonically increasing logical-action generation and its pending generation;
- action result/progress/status;
- source availability for each registered source;
- logical-load generation for stale completion suppression; and
- loading and last-refresh error state.

Move the old branch's selection and pending-operation rules intact: a selection
survives refresh when its stable ID remains; it falls back to no selection only
when the entity is absent; dialog state remains open on validation/action
failure; success closes the dialog after refresh is scheduled.

Only one logical action is pending application-wide. On confirmed submission,
increment `logical_action_generation`, store it with the pending action, and
attach it to every progress/finished task message. A progress or completion
message may mutate state, close a dialog, or schedule refresh only when its
generation equals the current pending generation; otherwise discard it without
user-visible status changes. Cancel/back can abandon an unsubmitted form but
cannot claim to cancel a dispatched native job. A second confirmation while an
action is pending is blocked with “Another logical operation is in progress.”

Update src/state/app.rs, state/mod.rs, and state/sidebar.rs to include logical
state without weakening the existing disk/volume/Btrfs state ownership.

### Messages and update flow

Port the source message families in src/message/app.rs and src/message/dialogs.rs
with their source names and branch ordering wherever the current-main router
permits. Add only the following documented integration messages when no source
variant exists:

- LoadLogicalEntities and LogicalEntitiesLoaded;
- LogicalSelectionChanged and LogicalDetailTabSelected;
- LogicalActionPrompted, LogicalActionConfirmed, LogicalActionCancelled;
- LogicalActionFinished and LogicalActionProgressed;
- dialog-field messages for LVM, MD RAID, Btrfs, and operation-control forms;
- LogicalMemberSelected.

Add `LogicalActionError { kind: OperationErrorKind, detail: String }` (or the
current-main equivalent structured payload) so Busy, Conflict, and Other remain
distinct through the dialog and status surface.

Create src/update/logical.rs from feature-only ranges and reconciled overlapping
ranges; do not use a source overwrite as an intermediate state. Retain its
branch order and all pre-existing helper boundaries. The transport,
opaque-identity, native-semantics, generation, and completed-reconciliation
substitutions documented in source-fidelity.md are the only planned changes. In
that composed update module, handle them in this order:

1. retain the source form/default/validation presentation, then resolve a
   source device or mount input against the latest state into a `BlockDeviceRef`
   (including fingerprint and observed generation) or logical ID in a typed
   action; reject a missing, ambiguous, changed, or out-of-range default-Btrfs
   subvolume reference before confirmation. For destructive deletion, derive
   the `ConfirmedDestructiveScope` from that same latest state; if it differs
   from the scope currently shown in the confirmation, replace the confirmation
   with the new sorted scope and require a fresh user confirmation without
   dispatching;
2. check the currently rendered entity capability;
3. store pending state and spawn the operation façade task;
4. render progress/status only when the task's logical-action generation is
   still current;
5. on a current success, schedule both refreshes, clear the pending action, and close the
   successful dialog;
6. on a current failure, retain the form and show the mapped error;
7. after refresh, re-resolve selection by stable ID and ignore an older logical
   load generation.

Wire these messages through update/mod.rs, app.rs, and subscriptions/app.rs.
There is no project-service client construction, proxy subscription, or
project-owned logical-topology signal.

**Gate**

~~~sh
cargo test -p cosmic-ext-storage --locked --test logical_state_contract -- --list | rg -F 'operation_generation_ignores_late_completion: test'
cargo test -p cosmic-ext-storage --locked --test logical_state_contract -- --list | rg -F 'logical_refresh_preserves_selection: test'
cargo test -p cosmic-ext-storage --locked --test logical_state_contract -- --list | rg -F 'failed_action_preserves_form_and_topology: test'
cargo test -p cosmic-ext-storage --locked --test logical_state_contract
cargo test -p cosmic-ext-storage --locked --test logical_operations_contract -- --list | rg -F 'topology_merge_has_stable_precedence: test'
cargo test -p cosmic-ext-storage --locked --test logical_operations_contract
~~~

**Manual plan-to-code review**

Trace both source results through the fixed merge policy, including a
LocalTools-only entity and each one-/two-source failure outcome. Then trace a
submitted action, a late completion after a newer action begins, and a failed
action through messages, pending state, dialog retention, and refresh. Verify
only the façade reaches contracts and every changed source overlap has its
reconciliation/source-fidelity evidence. Record the reviewed ranges and test
output in the Task 4 phase record.

## Task 5 — port every logical UI flow and dialog

Create the logical surface by literal-transplanting feature-only ranges and
composing every overlapping range according to `reconciliation.md` before any
adaptation:

~~~text
src/controls/logical/mod.rs
src/views/logical.rs
src/views/dialogs/logical.rs
src/update/logical.rs
src/state/logical.rs
~~~

Update controls/mod.rs, views/mod.rs, views/dialogs/mod.rs, and the relevant
existing app/sidebar files to expose them.

### Sidebar and detail rendering

Preserve the old branch UI literally, bar-for-bar, in current COSMIC
components. The target may not merely resemble the source:

- a Logical sidebar section with roots and nested LVs/PVs, MD members, Btrfs
  devices, and Btrfs subvolumes;
- entity icons, source name-then-ID child ordering, selected-row state, loading
  state, and empty/error state;
- Overview, Members, Operations, and Btrfs detail tabs;
- capacity/used/free summary, health/status/progress, metadata, member role and
  state, and logical aggregate summary;
- member rows that navigate to a known physical disk and remain readable but
  inert for a vanished device;
- operation buttons from LogicalCapabilities. A blocked button displays its
  precise blocked reason rather than disappearing.

Do not replace any old branch control with a generic warning or read-only
notice. Existing disk, partition, encryption, filesystem, image, Btrfs
subvolume, usage-scan, and rclone screens must remain reachable and unchanged
except for required navigation integration.

### Dialog inventory

Copy all four source dialog families and their validation/confirmation paths
before adapting their operation call:

1. LVM wizard: create/delete VG, add/remove PV, create/delete LV, absolute LV
   resize, and activate/deactivate LV.
2. MD RAID wizard: retain the source array-device and level fields, derive the
   documented native create profile before Apply, and show a blocked reason when
   the profile cannot be represented; retain start/stop/delete, add/remove
   member, check, and repair controls.
3. Btrfs wizard: retain its source member-device, mount-point, and size-spec
   fields (including default `max`), plus set label and choose/set default
   subvolume. Absolute size specs dispatch natively; unsupported source syntax
   remains visible and disabled with its exact reason.
4. Per-entity control dialog: destructive confirmations, capability reason,
   pending/progress state, success/failure state, and cancel/back behaviour.

Deletion confirmation is bound to a sorted `ConfirmedDestructiveScope`, not
just a primary entity label: VG confirmation lists every LV that native Delete
will remove, LV confirmation lists all supported dependent volumes, and MD
confirmation lists every member whose RAID metadata native Delete will destroy.
If fresh state changes that list, the old confirmation is invalid and the user
must review the replacement scope before submission.

Do not replace source text fields or defaults with a newly designed selector.
At submission, resolve the entered source device/mount value against the latest
physical/logical state into a `BlockDeviceRef` (canonical live ID, immutable
fingerprint, and observed object-manager generation) or logical ID. Never
derive identity from a display label or let a D-Bus path cross the UI boundary.
Revalidate immediately before confirmation and surface a changed-device error
rather than submitting stale input. This identity conversion is ledgered for
each affected source range.

Retain source labels and text exactly. Add an i18n key only where a current-main
component API requires it; its initial English value must equal the source text,
and the layout/API ledger entry must name it. Retain all existing localization
keys.

### UI tests

Add deterministic unit/component tests for source name-then-ID child ordering,
tab selection, blocked reason rendering (including unsupported Btrfs size
syntax and MD profiles), every action-to-dialog mapping, form validation,
confirmation/cancel rules, pending/success/failure state transition, selection
survival across a refresh, stale member navigation, changed destructive-scope
reconfirmation, and the absence of a project-service import.

**Gate**

~~~sh
cargo test -p cosmic-ext-storage --locked --test logical_ui_contract -- --list | rg -F 'logical_dialogs_preserve_source_defaults: test'
cargo test -p cosmic-ext-storage --locked --test logical_ui_contract -- --list | rg -F 'blocked_action_is_visible_and_inert: test'
cargo test -p cosmic-ext-storage --locked --test logical_ui_contract -- --list | rg -F 'confirmation_binds_current_device_reference: test'
cargo test -p cosmic-ext-storage --locked --test logical_ui_contract -- --list | rg -F 'confirmation_rejects_changed_destructive_scope: test'
cargo test -p cosmic-ext-storage --locked --test logical_ui_contract
cargo fmt --all -- --check
~~~

**Manual plan-to-code review**

Compare each affected source dialog/control/view range with its target and
reconciliation row. Manually exercise one enabled destructive action, one
unsupported Btrfs form, one unavailable MD profile, and one stale member link
in the component harness; verify labels/defaults/order remain source-faithful,
the confirmation describes the native side effect, and no display value is used
as identity. Change a delete's collateral scope before confirmation and verify
the dialog requires a newly reviewed scope. Record screenshots or component
snapshots plus the reviewed ranges in the Task 5 phase record.

## Task 6 — port the complete asynchronous sidebar change

Copy every feature-only behaviour-bearing `93094bd` change and compose every
overlap listed in `reconciliation.md` into its mapped current-main file after
Task 4 has the shared façade. This includes app startup, drive loaders,
dialog/sidebar state, subscriptions, network loading/rendering, logging, and
their harness adjustments—not only the logical sidebar files. Preserve its
semantics and branch structure; replace only its source-layout imports,
operations calls, ledgered current-main concurrency guards, and a completed
main-preservation reconciliation range.

- Sidebar startup issues logical, physical, and network loads asynchronously;
  it does not block app construction. Preserve the source incremental physical
  drive rows, spinner state, deterministic drive ordering, and network loading
  indication as well as the logical section.
- Loading, empty, success, and error states preserve the existing selected
  physical/logical location.
- A physical device event and a successful logical action coalesce duplicate
  refreshes, then perform one sidebar redraw from the newest result. This is a
  documented current-main concurrency guard; it may not change the source
  visible selection/default behaviour.
- An async result for an older request sequence cannot overwrite a newer
  selection/topology. Store monotonically increasing logical load generation
  in state and ignore stale completions.
- The sidebar never instantiates UdisksBackend, LocalLogicalTopologySource, a
  D-Bus proxy, or a service client; it reads state only.

Test source incremental drive ordering/loading, network loading, logical load
generation, selection preservation, event/action refresh coalescing, error
retention, and no synchronous blocking during app initialization. Port the
source harness tests changed by `93094bd` in Task 7 and ledger their path/setup
adaptations.

**Gate**

~~~sh
cargo test -p cosmic-ext-storage --locked --test sidebar_async_contract -- --list | rg -F 'newer_load_wins: test'
cargo test -p cosmic-ext-storage --locked --test sidebar_async_contract -- --list | rg -F 'event_and_action_refresh_coalesce: test'
cargo test -p cosmic-ext-storage --locked --test sidebar_async_contract -- --list | rg -F 'physical_network_and_logical_loading_do_not_block_startup: test'
cargo test -p cosmic-ext-storage --locked --test sidebar_async_contract
cargo clippy -p cosmic-ext-storage --all-features --locked
~~~

**Manual plan-to-code review**

Trace the app startup and one physical-event/action race using deterministic
test-controlled completions. Verify the newest logical-load generation wins,
exactly one refresh is scheduled for coalesced triggers, and the source
physical/network spinner and ordering branches remain present beside the new
logical branch. Compare every touched overlap to its reconciliation/source
fidelity rows and record the evidence in the Task 6 phase record.

## Task 7 — restore the full serviceless test harness and lab

Create tools/storage-testing as a non-published root-workspace member named
storage-testing. Add it to workspace.members; retain the root application as a
default member so ordinary app commands remain scoped as today. The final
workspace has seven packages: the root app, five published libraries, and this
non-published test tool. Release/publish commands continue to name only the
root app and five published library manifests.

Copy the entire old storage-testing tree into tools/storage-testing first,
retaining its directory/file topology, before changing any code:

~~~text
tools/storage-testing/
  Cargo.toml
  src/
  tests/
  resources/ (if introduced only for current-main-relative lab paths)
~~~

Preserve its fixture ledger, loop-device allocation/cleanup protocol,
disposable-media confirmation, test isolation, result reporting, and all test
families: Btrfs, disk, filesystem, image, logical, LUKS, partition, and rclone.
Keep every old test body, test name, fixture, assertion, ledger check, and lab
step verbatim wherever possible. Replace service/client setup so the same test
invokes typed LogicalTopologySource/LogicalOperations through UdisksBackend and
the app operations façade, never LogicalClient or the removed project D-Bus
name. Record every changed test range in the source-fidelity ledger.

Create one internal `FixtureCommandExecutor` for the source lab's unavoidable
privileged fixture lifecycle. It is the only location in the test tool allowed
to spawn a process. Its allow-list is the reviewed source lab allocation/reset/
teardown set (for example loop attach/detach, partition rescan, mount cleanup,
and the source's LVM/Btrfs signature reset); every argument must resolve to a
currently allocated ledger target or a run artifact path. It may not run a
shell, accept a free-form command, use `sudo`/`pkexec`, or be linked by an
application crate. Tests move direct source cleanup calls into this executor
without changing their ordering, cleanup result, or failure reporting. The
logical mutation that a test is asserting must still go through the typed
native adapter. Replace the source harness `id -u` process preflight with a
process-free effective-UID check; do not add it to the fixture command
allow-list.

Recreate the old public entry points in justfile:

~~~text
just harness
just harness-nondestructive
just lab
~~~

They construct/use a disposable fixture environment, verify UDisks2/plugin
prerequisites, and fail closed if a target is not an allocated loop-backed
fixture. `just harness` is the full destructive suite: it must run only in the
gated disposable VM with `STORAGE_TESTING_ENABLE_DESTRUCTIVE=1` and invoke the
runner with `--profile full-lab --require-executed`. The runner has a static,
validated case catalog: every case has one unique ID, suite, ordered fixture
requirements, and `NonDestructive` or `Destructive` safety class. An unknown
suite/ID/profile, duplicate ID, or empty selection is a CLI error. The
`nondestructive` profile excludes destructive cases while constructing the
selection; it is not a run-time skip.

The execution report is a versioned JSON artifact at
`$STORAGE_TESTING_ARTIFACT_DIR/run-report.json`. Each harness recipe creates a
fresh ledger-owned run-artifact directory and passes that variable to the
runner; a direct runner invocation fails before selection if it is unset,
inaccessible, or not a run-artifact directory. Write the report atomically and
include the sorted selected case IDs, per-case result, setup/teardown result,
fixture ledger references, and summary counts. A selected case outcome is only
`Passed`, `Failed`, or
`Blocked { reason }`; `Skipped` is removed from the runner model. A preflight,
fixture setup, missing required plugin, or unavailable authorization path that
prevents a selected case from running is `Blocked` and gives a non-zero exit.
A timeout is `Failed`, never `Blocked`. `--require-executed` gives a non-zero
exit unless every selected case ran and passed, and it verifies that the report
contains exactly one result for every selected ID. Cleanup always runs after a
started group; a cleanup failure fails the group and report.

Convert each source `support::skip` path by its cause: destructive opt-out is
profile exclusion; missing fixture or unavailable service/plugin is a failed or
blocked setup/preflight; a timeout is failed; and plugin-absence behaviour is a
separate deterministic adapter/UI blocked-capability test. No source test may
return a success-like omission. Non-destructive CI invokes
`just harness-nondestructive`; that recipe passes `--profile nondestructive
--require-executed` to the runner. `just harness` passes `--profile full-lab
--require-executed`, and full-lab is the only profile permitted to select
destructive cases. The runner receives any required host privilege from its
disposable VM or gated CI environment; it must not start a project service,
install a policy, invoke sudo, self-elevate, or mutate a non-fixture disk.

Add CI jobs that compile, format, lint, and run `just harness-nondestructive`.
Run `just harness` and destructive lab suites only in an explicitly gated
disposable VM job with required UDisks2 LVM2/Btrfs plugins, the required
API-version preflight, `--require-executed`, and logged fixture
cleanup/command ledger. Preserve existing main CI checks; no service
start/healthcheck remains.

Do not add a tool-local lockfile. Generate/update the root Cargo.lock only if
the workspace member introduces an unavoidable new resolved dependency. Any
such delta must be reviewed as an upgrade-preserving addition, never a
downgrade to branch-era packages.

**Gate**

~~~sh
cargo fmt --all -- --check
cargo test -p storage-testing --locked --test harness_execution_contract -- --list | rg -F 'selected_cases_cannot_be_skipped: test'
cargo test -p storage-testing --locked --test harness_execution_contract -- --list | rg -F 'timeout_is_failed: test'
cargo test -p storage-testing --locked --test harness_execution_contract -- --list | rg -F 'fixture_target_and_cleanup_are_enforced: test'
cargo test -p storage-testing --locked --test harness_execution_contract
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-features --locked
just harness-nondestructive
# Gated disposable VM only.
STORAGE_TESTING_ENABLE_DESTRUCTIVE=1 just harness
~~~

Run `just lab` only in the documented disposable environment. The full-lab
command must attach its report and fail unless every selected case executed and
passed; no selected case may be omitted, blocked, or skipped.

**Manual plan-to-code review**

Inspect the catalog/selection code, JSON report schema, fixture executor
allow-list, and cleanup `finally` path. Deliberately run the reusable runner
tests for an empty selection, a missing prerequisite, a timeout, a non-fixture
target, and a cleanup failure; verify their report outcome and non-zero exit.
Then compare each ported source harness family to its source-fidelity row and
record command output, report artifacts, and reviewed ranges in the Task 7
phase record.

## Task 8 — documentation and final acceptance record

Update README.md with the final logical-storage feature set, the UDisks2 LVM2,
MD RAID, and Btrfs plugin prerequisites, the native Polkit authorization
expectation, and harness/lab usage. Remove or avoid all instructions that
refer to the removed service.

Complete validation.md with the exact branch SHA, OS/UDisks versions, enabled
plugins, commands, and pass/fail results. A failed unavailable-plugin scenario
is acceptable only if the UI gives the documented disabled reason; a selected
action test is not accepted if it is omitted, blocked, or silently skipped.

**Gate**

Before handoff, inspect the whole diff for forbidden artifacts:

~~~sh
rg -n -i 'storage-service|org\.cosmic\.ext\.Storage\.Service|LogicalClient|sudo|pkexec|vgcreate|vgremove|vgextend|vgreduce|lvcreate|lvremove|lvresize|lvchange|mdadm|btrfs .*device (add|remove)' src crates resources .github Cargo.toml justfile
rg -n 'Command::new|std::process::Command' tools/storage-testing
git diff --check
cargo metadata --no-deps --format-version=1
cargo fmt --all -- --check
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-features --locked
cargo build --workspace --release --locked
~~~

The production search may match the read-only storage-sys parser allow-list but
must find no production logical mutation implementation outside the native
UDisks adapter. The test-tool search must identify only
`FixtureCommandExecutor`; review every match and every allow-list entry against
the fixture ledger. Historical plan text is deliberately excluded from this
gate.

**Manual plan-to-code review**

Review the completed phase records, reconciliation rows, source-fidelity
ledger, test-target inventory, action matrix, and validation evidence as one
traceable chain. For each public action, select its contract test, adapter test,
UI/component test, and fixture/lab evidence; verify all refer to the same
behaviour rather than a task/branch milestone. Check README and CI recipes
against the implemented commands. Record the final reviewed commit range and
reviewer in the Task 8 phase record.

## Completion definition

The graft is complete only when the target branch is based on current main;
all matrix actions work through native UDisks2 authorization; all old logical
UI/dialog/sidebar behaviour exists as a literal transplant in current source
layout except for ledgered unavoidable adaptations; all original harness/lab
suites are restored against typed in-process adapters; and every automated plus
disposable-desktop validation is recorded. A branch with only topology, only
UI, only selected actions, a loosely similar rewrite, or a downgraded harness
does not meet this plan. The only test-only command exception is the reviewed
ledger-validated fixture executor; it is not a product fallback.

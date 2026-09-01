# LVM Refactor Validation

**Status:** Repository implementation and safe-profile validation complete at
`34c39c0f7a2dc27fbd2956dd44797b784c1c9dd7`. The destructive native-action
matrix remains deliberately gated to a disposable VM; it was not run from this
developer host.

This record validates a full semantic graft of 079-lvm-support. It is not a
read-only topology acceptance record: every logical action, UI flow, async
sidebar behaviour, harness entry point, and integration test family must be
covered.

## Preconditions and test environment

Use a disposable COSMIC Wayland VM with a current main-based build, system
UDisks2 2.11 or newer, and the LVM2 and Btrfs UDisks plugins enabled. The full
matrix requires 2.11 because the source-compatible MD metadata profile and
`SetDefaultSubvolumeID` depend on APIs introduced in that line. The test user
must use the normal desktop UDisks/Polkit authorization path. Record the result
of:

~~~sh
git rev-parse HEAD
udisksctl --version
udisksctl status
systemctl status udisks2
busctl introspect org.freedesktop.UDisks2 /org/freedesktop/UDisks2/Manager
cargo metadata --no-deps --format-version=1
~~~

After allocating representative fixtures, record the introspection result for
the selected LVM VolumeGroup/LogicalVolume, MDRaid, and Btrfs block objects.
It must prove the exact methods used by the action matrix, including the MD
profile `version` option encoded as D-Bus `ay b"0.90"`, the LVM `wipe=false`
delete/remove-PV calls, explicit `tear-down=false` deletion options, MD member
`wipe=false`, Btrfs `GetSubvolumes`/`GetDefaultSubvolumeID`, and Btrfs
`SetDefaultSubvolumeID` with its unsigned-32-bit argument.

The VM must allocate every test target from the harness loop-backed fixture
ledger. Do not use a host disk, manually launched project daemon, project
policy, sudo, or pkexec. The logical action under test must use the typed native
adapter. Fixture allocation/reset/teardown may use only the reviewed
`FixtureCommandExecutor`, with each command and target recorded in the ledger.

For each native plugin unavailable in the image, run the corresponding UI
scenario and record the required blocked reason. That establishes graceful
unavailability only; it does not substitute for the same scenario on an image
where the plugin is enabled.

## Durable test contracts and phase records

The named test targets in the implementation plan are product-contract suites,
not one-off assertions that this branch happened to be merged. Their test names
describe stable behaviour: identity safety, native call translation, state
generation, UI binding, async ordering, and harness safety. Extend those suites
when future work changes the relevant behaviour; do not replace them with a
branch/task completion test.

Every code-bearing phase gate first proves that its named test target exists and
contains the required named behavioural tests using `cargo test --test … --
--list | rg`, then executes the entire target. A broad filter such as
`cargo test … logical` is never sufficient because it can select zero tests and
still pass. The exact target/name inventory and commands are authoritative in
the [implementation plan](implementation-plan.md#deterministic-implementation-and-phase-gate-protocol).

After every Task 0–8 gate, append a row here. The manual review is a scoped
code-review artifact, not a substitute for the automated tests.

| Task | Commit/range | Specification and plan clauses traced | Source/reconciliation rows checked | Automated evidence | Manual normal + edge/failure path reviewed | Result | Reviewer |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | `2dcba1d` | Baseline and plan protocol | n/a | Existing workspace baselined before implementation | No service/client paths introduced | pass | Codex |
| 1–2 | `78d0972` | Typed identities, topology, contracts | types/contracts | `cargo test -p storage-types -p storage-contracts --locked` | Canonical IDs and structural action validation | pass | Codex |
| 3 | `2f15f65` | Native adapter and read-only local source | UDisks/native-semantics | adapter and storage-sys contract targets | Native proxy calls retain typed options; local tools are read-only | pass | Codex |
| 4 | `3d0070c` | Application operations/state | app composition | `logical_state_contract`, `logical_operations_contract` | Ordered source merge and generation guards | pass | Codex |
| 5 | `6654517`, `24e9d64` | Logical sidebar/detail/action confirmation | UI/transport | `logical_ui_contract` and `cargo check -p cosmic-ext-storage --locked` | Typed action confirmation, blocked input-taking actions, and post-action refresh | pass | Codex |
| 6 | `28d3e1f` | Concurrent incremental sidebar loading | async lifecycle | `sidebar_async_contract` | Physical, network, and logical loads are independent | pass | Codex |
| 7 | `34c39c0` | Harness catalog/report/fixture boundary | fixture-boundary/harness-execution | `harness_execution_contract`; `just harness-nondestructive` | Required safe profile writes report and ledger; full profile rejects an ungated host | pass (safe profile) | Codex |
| 8 | documentation acceptance commit | Final documentation and repository evidence | root/docs | workspace tests, fmt, clippy, metadata, release build | Native destructive VM remains explicitly unclaimed | pass (repository gates) | Codex |

The Task 7 full-lab record additionally links the versioned runner JSON report,
selected case IDs, fixture ledger, and cleanup result. The Task 8 record links
the complete table and checks the evidence chain from public action to contract,
adapter, UI/component, and fixture/lab test.

## Automated repository gates

Run from the repository root. When a local compiler wrapper is unavailable,
unset RUSTC_WRAPPER first.

| Check | Required result |
| --- | --- |
| cargo metadata --no-deps --format-version=1 | Seven workspace packages: root app, five published libraries, and non-published tools/storage-testing; no service package. |
| cargo fmt --all -- --check | Pass. |
| cargo test --workspace --all-features --locked | Pass, including types, contracts, adapter, app state/view, and harness-runner contract tests; disposable fixture execution is exercised only through the explicit harness profiles. |
| cargo clippy --workspace --all-features --locked | Pass without new diagnostics. |
| cargo build --workspace --release --locked | Pass and emits no storage-service binary. |
| Named phase test targets | Every Task 1–7 target/name check and full target execution specified by the implementation plan passes; the test target cannot be empty or missing. |
| just harness-nondestructive | Invokes the runner's `nondestructive` profile with `--require-executed`; its report contains exactly one Passed result for every selected non-destructive case. |
| just harness | Runs only in the gated disposable fixture environment with `STORAGE_TESTING_ENABLE_DESTRUCTIVE=1`; invokes `full-lab` with `--require-executed`; every selected case executes and passes. |
| just lab | Runs only in the gated disposable VM, records cleanup, and attaches the versioned execution report. |

## Architecture gate

Review the accepted diff and mark every item pass/fail.

- The application calls logical discovery/actions only through
  src/operations/logical.rs and storage-contracts traits.
- BackendRegistry owns separate `logical_topology_sources` and
  `logical_operations`; neither logical trait is a BlockStorageBackend
  supertrait. The one UdisksBackend instance is registered as the UDisks source
  and executor, and storage-sys is registered only as the local source.
- UdisksBackend is created once at the composition root and reuses its existing
  DiskManager connection for logical calls. Its object-manager epoch advances
  only after an applied native add/remove event; cached reads do not fabricate a
  new generation.
- The only concrete LVM, LogicalVolume, Btrfs, MDRaid, and Manager proxy use is
  inside crates/storage-udisks.
- Logical IDs are stable domain IDs. `BlockDeviceId` is an opaque live
  `block:<major>:<minor>` lookup key, not a D-Bus path. Every block mutation
  carries `BlockDeviceRef` with an immutable fingerprint and observed
  object-manager generation; the adapter rechecks both immediately before the
  native call and returns Conflict if a device-number reuse is detected. Object
  paths and raw option maps do not cross into contracts, state, messages,
  dialogs, views, or tests above the adapter boundary.
- `LogicalTopology` rejects duplicate entity/source IDs, emits source statuses
  in discriminant order and entities in case-sensitive name-then-ID order. The
  façade's UDisks-over-local merge never overwrites an empty-but-present UDisks
  value or grants a LocalTools-only entity mutation capability.
- Only one logical action is pending. Operation generation and logical-load
  generation independently prevent late progress/completion or load results
  from changing state, closing a dialog, or scheduling a refresh.
- storage-sys only performs allow-listed read-only discovery. It has no
  state-changing command invocation, shell invocation, privilege elevation, or
  sysfs write.
- The application and production crates have no project service package,
  project D-Bus client/proxy, system unit, policy XML, project D-Bus signal,
  service launch recipe, sudo, pkexec, or privilege fallback. The only
  test-tool process spawner is the reviewed ledger-validated fixture executor.
- The UDisks error bridge preserves NotFound, InvalidInput, Unsupported,
  Unavailable, PermissionDenied, Busy, Conflict, and Other through the UI;
  `StorageErrorKind::Other` and the matching OperationError variants have
  focused mapping/presentation tests.
- The default app/package publishing flow still names only the app and five
  published libraries; storage-testing is not published.
- Cargo.lock preserves main's package upgrades except reviewed additions
  required by the new workspace test member.

## Literal source-fidelity gate

Review every mapped file in source-fidelity.md against
origin/079-lvm-support at 93094bd4ecea0f7b6236b17e34753470b8ee616d. The
review standard is literal preservation, not approximate functional
equivalence.

- The target began as a direct source-file transplant, with original helper
  boundaries, names, comments, message variants, state fields, render order,
  defaults, validation, confirmation/cancel branches, fixtures, test bodies,
  and lab steps retained for feature-only ranges. Every overlap first has a
  completed `reconciliation.md` row with base/main/feature evidence and a
  test proving both preserved behaviours.
- Every changed contiguous source range has a completed source-fidelity ledger
  entry containing its allowed reason, precise replacement, test evidence, and
  reviewer.
- A layout/import move or removed service call is not permission to alter
  unrelated UI or logic in the same function/file.
- No old logical UI element, dialog field, status/result, state transition,
  action branch, test, fixture, ledger protection, or lab step is missing,
  renamed, consolidated, reordered, or newly designed unless its exact
  unavoidable reason and behaviour-preservation evidence appear in the ledger.
- Visual ranges have a desktop screenshot/recording reference. State/update
  ranges have a focused regression test named in the ledger. Harness ranges
  have the retained test/command evidence.
- Every source harness `Skipped` conversion is ledgered as
  `harness-execution` with its exact replacement (profile exclusion, Blocked,
  or Failed) and retains the original case ID, fixture lifecycle, and test
  intent. A conversion that simply removes a selected case is a merge blocker.

For every mapped file, inspect a source export against the destination:

~~~sh
git diff --no-index --word-diff=porcelain \
  <source-export-path> <target-path>
~~~

An unledgered difference or a target UI/logic that merely resembles the source
is a merge blocker.

Use these audit searches and review each match:

~~~sh
rg -n -i 'storage-service|org\.cosmic\.ext\.Storage\.Service|LogicalClient|sudo|pkexec|vgcreate|vgremove|vgextend|vgreduce|lvcreate|lvremove|lvresize|lvchange|mdadm' src crates resources .github Cargo.toml justfile
rg -n 'Command::new|std::process::Command' tools/storage-testing
~~~

The first command may match the storage-sys read-only discovery allow-list but
never a production mutation path. The second must identify only
`FixtureCommandExecutor` and its tests; each command must have a ledger target.
The harness privilege preflight uses a process-free effective-UID API.

## Unit and adapter action matrix

Every row needs a deterministic fake-proxy unit test and an adapter assertion
that the exact native UDisks call is selected with resolved object paths and an
empty typed options map unless documented otherwise.

| Feature | Required unit/adapter assertion |
| --- | --- |
| Block mutation reference | Uses the exact versioned fingerprint tier order (loop backing identity, partition UUID, filesystem UUID/type, WWN+serial). A malformed/missing reference fingerprint is InvalidInput; a same-major:minor hot-unplug/replug replacement, missing/ambiguous current fingerprint, or changed observed generation is Conflict before a proxy method; a matching reference resolves only its current device. |
| Create LVM VG | Validates name/devices and `BlockDeviceRef`s; resolves blocks; calls Manager.LVM2.VolumeGroupCreate; waits for result. |
| Delete LVM VG | Resolves VG even when it has LVs; confirmation carries an exact fresh-snapshot LV scope; calls VolumeGroup.Delete with native `wipe=false` and `tear-down=false`; maps busy/permission/native job errors. |
| Add/remove LVM PV | Resolves VG and `BlockDeviceRef`; calls VolumeGroup.AddDevice/RemoveDevice, with RemoveDevice native `wipe=false`; refreshes result. |
| Create LVM LV | Validates absolute non-zero size; calls VolumeGroup.CreatePlainVolume. |
| Delete/resize LVM LV | Resolves LV; exercises plain, non-thin-snapshot, and thin-pool fixtures. Delete is permitted only where source/native dependent-volume scope parity is proven, rechecks the confirmed scope, calls Delete with `tear-down=false` or Resize, and waits for native job. |
| Activate/deactivate LV | Calls LogicalVolume.Activate/Deactivate and reports authorization result. |
| Create MD RAID | Normalizes the source array-device field; validates every locked level/minimum-member/chunk row; calls Manager.MDRaidCreate with exact `version: ay b"0.90"` (not a string); and accepts success only for a running returned array. |
| Delete/start/stop MD RAID | Calls MDRaid.Delete with `tear-down=false`, Start/Stop; Delete rechecks the confirmed complete member-reference scope and records destruction of every member's MD metadata in the confirmation/fixture ledger; maps state conflicts. |
| Add/remove MD member | Resolves a matching `BlockDeviceRef` and calls MDRaid.AddDevice/RemoveDevice; RemoveDevice has explicit `wipe=false`. |
| MD check/repair | Maps closed enum to RequestSyncAction check/repair only. |
| Add/remove Btrfs device | Resolves Filesystem.BTRFS and a matching `BlockDeviceRef`; calls AddDevice/RemoveDevice. |
| Resize Btrfs | Preserves/parses the source size-spec field. An absolute non-zero size calls Filesystem.BTRFS.Resize; `max`, grow, and shrink syntax remains visible and blocked unless a documented native mapping is verified. |
| Set Btrfs label | Validates label; calls Filesystem.BTRFS.SetLabel. |
| Set Btrfs default subvolume | Discovers subvolumes/default with `GetSubvolumes`/`GetDefaultSubvolumeID`; accepts only `1..=u32::MAX`; calls Filesystem.BTRFS.SetDefaultSubvolumeID with a non-zero unsigned 32-bit ID. |
| Plugin/API absent | Produces a blocked capability and Unavailable/Unsupported UI result, including Btrfs default-subvolume and MD metadata-profile version absence; does not run a CLI fallback. |
| Native denial | Maps UDisks/Polkit denial to PermissionDenied and leaves current topology selection intact. |
| Device disappearance/replacement | Maps disappearance to NotFound or Conflict and same-device-number replacement to Conflict; never acts on a cached D-Bus path. |

Also test all discovery entity kinds: LVM VG/LV/PV, MD array/member, Btrfs
filesystem/device/subvolume. Cover hierarchy, stable IDs, members, metadata,
health/progress, capability sorting/block precedence, missing-plugin source
status, and deterministic UDisks-over-local merge behaviour.

## Desktop UI and full-action matrix

Run each scenario in the disposable COSMIC desktop VM with screenshots or
recording references and a harness ledger ID. After every successful action,
confirm one physical and one logical refresh, correct updated hierarchy, no
stale selection, and no project service process/socket.

| Area | Scenario | Required visible result |
| --- | --- | --- |
| Sidebar | Cold start | Physical, Logical, and Network sections load asynchronously; source incremental physical rows/spinners/order are retained and UI remains responsive while loading. |
| Sidebar | Repeated refresh/event | Stale async result cannot overwrite newer selection/topology; duplicate event/action refreshes coalesce. |
| Identity safety | Hot-unplug/replug same device number | A pending device dialog reports changed-device Conflict on confirmation; the replacement fixture is untouched and no native method is sent. |
| LVM | Create VG | Source VG/device-CSV fields and defaults remain visible; entered fixture devices resolve to canonical references with fingerprint/generation, validation/confirmation/native authorization run, and the new VG/PV hierarchy appears. |
| LVM | Delete VG | Control requires confirmation that lists the exact affected LV scope and preserved PV/configuration state. A changed scope requires reconfirmation; busy/dependency or permission failure is surfaced, and removal happens only after successful native result. |
| LVM | Add/remove PV | Source PV-device form resolves to a current canonical fixture reference with fingerprint/generation and updates membership/free capacity. |
| LVM | Create/delete LV | Wizard validates name/absolute size; detail/sidebar update LV hierarchy and capacity. |
| LVM | Resize LV | Control accepts absolute size, shows pending/result state, and reflects capacity after refresh. |
| LVM | Activate/deactivate LV | Operations tab reflects state/capability change and resulting block availability. |
| MD RAID | Create array | Source array-device/level fields remain visible; every enabled level uses its locked source-compatible chunk and `0.90` `ay` profile, then creates a running array/member hierarchy with correct level/status. |
| MD RAID | Start/stop/delete | Controls are present, confirmed where destructive; delete confirmation and the fixture ledger enumerate an exact current member scope and all metadata destruction. A changed scope requires reconfirmation; state/error/result renders correctly. |
| MD RAID | Add/remove member | Membership controls refresh member roles/states and capacity/health. |
| MD RAID | Check/repair | Operations controls call check/repair distinctly and display running/progress/result state. |
| Btrfs | Add/remove device | Btrfs dialog controls use an existing filesystem and eligible fixture, then update device hierarchy. |
| Btrfs | Resize filesystem | Source size-spec/default remains visible. Absolute size validates and reflects capacity after native completion; unsupported source syntax gives its exact blocked reason without submission. |
| Btrfs | Set label | Displays old/new label and immediate refreshed detail state. |
| Btrfs | Set default subvolume | Discovers/selects a valid subvolume and refreshes the Btrfs detail/default state; zero and above-u32 IDs remain visible with the exact blocked reason and do not submit. |
| Detail UI | Tabs and members | Overview, Members, Operations, and Btrfs tabs preserve old content; member links open a known physical disk and are inert/readable when stale. |
| Capability UI | Blocked action | Control remains visible with precise unavailable/busy/dependency reason; it cannot submit. |
| Failure UI | Polkit deny/native error and late completion | Dialog remains open with form input and action-specific mapped error; old topology/selection remain. A completion from an older action generation cannot close the current dialog or schedule refresh. |
| Existing main | Non-logical workflows | Disk, partition, LUKS, filesystem, image, Btrfs subvolume, usage scan, rclone, and network/volume workflows remain usable. |

Record fixture cleanup status after each destructive group. A failing cleanup
prevents later groups from being accepted.

## Harness and lab parity

Port and execute every historical storage-testing family:

| Suite | Required evidence |
| --- | --- |
| disk | Fixture allocation, discovery, drive operations, cleanup. |
| partition | Create/edit/delete/resize fixture partitions and ledger cleanup. |
| filesystem | Format/mount/label/check and failure reporting on fixtures. |
| luks | Encrypt/unlock/lock and cleanup without a project service. |
| image | Image/loop setup and teardown. |
| btrfs | Subvolume plus multi-device Btrfs discovery/action coverage. |
| logical | Every LVM, MD RAID, and Btrfs action in the matrix through typed adapters. |
| rclone | Existing configuration/operation tests remain present and runnable. |
| harness | Ledger refusal for non-fixture target, isolation, failure cleanup, reporting. |
| lab | Disposable VM provisioning, plugin preflight, run record, and cleanup. |
| execution policy | The static catalog gives every case a unique ID, fixture requirements, and safety class. `--require-executed` reports exactly one Passed result for every selected ID; the runner has no `Skipped` outcome, a timeout is Failed, and a preflight/setup failure is Blocked with a non-zero exit. Plugin absence is an explicit blocked-capability UI test, while destructive cases are excluded only by the named non-destructive profile. |

The test tool may call UdisksBackend and the in-process façade directly. Any
remaining LogicalClient, project D-Bus address, service start, policy
installation, or application/project privilege-helper assumption is a parity
failure. A host privilege requirement for the disposable lab is permitted only
through its ledger-validated fixture executor and is not an acceptable reason
to omit a typed logical action. Attach the selected/passed/failed/blocked case
report to each validation record; a selected Blocked or Failed case blocks
acceptance.

## Merge acceptance

The change is ready to merge only when all automated gates pass, the
architecture gate is clean, every enabled-plugin desktop action row passes,
every unavailable-plugin row displays the documented blocked reason, and all
harness/lab parity evidence is recorded. The literal source-fidelity gate and
its complete deviation ledger are independently required, as is a complete
three-way reconciliation record for every overlapping range. No selected
harness or lab case may be omitted, blocked, failed, or skipped.

Do not accept a branch that preserves only visual controls, only discovery,
only a subset of mutations, or a reduced harness. The target is a bar-for-bar
functional graft into the serviceless architecture.

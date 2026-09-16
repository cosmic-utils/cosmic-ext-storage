# Retired testing-infrastructure history

These are verbatim historical excerpts, not current commands or acceptance evidence.
Testing V2 supersedes these harness instructions. See [the specification](spec.md)
and [current lab commands](../../../tools/storage-lab/README.md).

Original snapshot: commit `6a3aec3`. Existing product requirements outside these
excerpts remain in their original plans. No old run is relabelled as a new test run.

## h001

Source: [docs/plans/2-lvm-refactor/baseline.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/baseline.md).

`````markdown
- The root package is `cosmic-ext-storage`; the retained libraries are under
  `crates/`. Before this graft, `cargo metadata` reports the root package plus
  `storage-udisks`, `storage-btrfs`, `storage-types`, `storage-contracts`, and
  `cosmic-ext-storage-storage-sys`. The completed graft additionally restores the
  non-published `tools/storage-testing` workspace member while leaving the six
  release/publish packages unchanged.
- `storage-service`, `storage-macros`, project systemd/D-Bus/Polkit resources,
  and all application client proxies were deliberately deleted.
- The application constructs `StorageOperations` once. Its `BackendRegistry`
  exposes typed `storage-contracts` traits; `UdisksBackend` is constructed only
  at the composition root.
- The workspace already has upgraded pins, including `toml = "1.0"`,
  `zbus = "5.15.0"`, `vergen-git2 = "10"`, and libcosmic revision
  `ef162b8e16ba4493e05c169cd56c7b9f77f0fda5`. The current `Cargo.lock` is the
  only dependency baseline to retain.
- A small, older LVM read path already exists in
  `crates/storage-types/src/lvm.rs` and `crates/storage-udisks/src/lvm/`. It
  lists LVs for a physical volume; it is not the first-class logical-topology
  feature intended by #79.
`````

## h002

Source: [docs/plans/2-lvm-refactor/implementation-plan.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/implementation-plan.md).

`````markdown
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
`````

## h003

Source: [docs/plans/2-lvm-refactor/implementation-plan.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/implementation-plan.md).

`````markdown
- Logical discovery and the entity hierarchy for LVM VG/LV/PV, MD RAID
  array/member, Btrfs filesystem/device/subvolume.
- Every action in the branch's LVM, MD RAID, and Btrfs dialogs and control
  surface, including disabled/actionable reasons and post-operation refresh.
- The logical detail tabs, overview/members/operations/Btrfs content, selection
  persistence, member navigation, status reporting, and asynchronous sidebar.
- The complete storage-testing harness, disposable lab, fixture ledger, and all
  existing integration test families.
`````

## h004

Source: [docs/plans/2-lvm-refactor/implementation-plan.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/implementation-plan.md).

`````markdown
| 7 | test(harness): restore serviceless integration harness and lab | Cargo.toml, justfile, tools/storage-testing/**, resources/lab-specs/**, .github/** |
`````

## h005

Source: [docs/plans/2-lvm-refactor/implementation-plan.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/implementation-plan.md).

`````markdown
| 7 | `tools/storage-testing/tests/harness_execution_contract.rs` | `selected_cases_cannot_be_skipped`; `timeout_is_failed`; `fixture_target_and_cleanup_are_enforced` |
`````

## h006

Source: [docs/plans/2-lvm-refactor/implementation-plan.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/implementation-plan.md).

`````markdown
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
`````

## h007

Source: [docs/plans/2-lvm-refactor/reconciliation.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/reconciliation.md).

`````markdown
| `Cargo.toml`, `Cargo.lock` | Current workspace graph, package upgrades, and no service package. | Non-published `storage-testing` membership when its harness is restored. | Modify main's manifests minimally; retain current versions unless the new test member needs a reviewed additive resolution. Never copy the feature lockfile. |
`````

## h008

Source: [docs/plans/2-lvm-refactor/reconciliation.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/reconciliation.md).

`````markdown
| `Cargo.toml` workspace members | Main keeps six publishable app/library packages without a service. | Feature adds a non-published harness package. | Independent overlap | `Cargo.toml` workspace members; `Cargo.lock` storage-testing entry | `cargo metadata --no-deps --format-version=1` | `harness_execution_contract` | Codex |
`````

## h009

Source: [docs/plans/2-lvm-refactor/reconciliation.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/reconciliation.md).

`````markdown
| `justfile`, CI and root documentation | Main retains normal workspace checks and no service lifecycle. | Feature adds harness entry points and safe CI execution. | Independent overlap | `justfile`, `.github/workflows/ci.yml`, `README.md` | `cargo test --workspace --all-features --locked` | `just harness-nondestructive` | Codex |
`````

## h010

Source: [docs/plans/2-lvm-refactor/source-fidelity.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/source-fidelity.md).

`````markdown
| storage-testing/** | tools/storage-testing/** | Copy every harness, lab, ledger, binary, test registration, and test case first. Service/client calls become typed setup; fixture-only commands move to the ledger-validated executor without changing lifecycle/test intent. |
`````

## h011

Source: [docs/plans/2-lvm-refactor/source-fidelity.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/source-fidelity.md).

`````markdown
   ~~~sh
   git diff --name-only 3f8c340..origin/079-lvm-support -- \
     storage-app storage-types storage-udisks storage-sys storage-contracts \
     storage-testing justfile resources .github
   git diff --name-only 3f8c340..origin/main -- \
     storage-app storage-types storage-udisks storage-sys storage-contracts \
     storage-testing justfile resources .github
   ~~~
2. For a feature-only range, create the mapped target from feature-branch
   content, preserving comments, helper boundaries, function names, message
   names, enum variants, field names, rendering composition, tests, and
   ordering. For an overlap, use the completed reconciliation classification;
   do not overwrite a main-only range as a preliminary transplant step.
3. Apply only the smallest compiling modifications allowed by the Rule.
4. Add a ledger row below for every changed contiguous source range. A moved
   range is recorded once, with source and target paths/line ranges. An overlap
   also cites its `reconciliation.md` row.
5. Add or retain a test proving the source behaviour at that range. Visual UI
   changes require a screenshot/recording reference in validation.md.
6. During review, compare the source and target with whitespace ignored only
   for path/import formatting. Any unexplained behavioral or layout divergence
   blocks the commit.
`````

## h012

Source: [docs/plans/2-lvm-refactor/source-fidelity.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/source-fidelity.md).

`````markdown
| `storage-testing/src/{cmd,ledger,harness/**,lab/**}` | `tools/storage-testing/src/**` | fixture-boundary / harness-execution | root workspace row | Arbitrary command execution and skip outcomes are replaced by a closed fixture executor, marker-bearing artifact directory, and Passed/Failed/Blocked report model. | `harness_execution_contract`, `just harness-nondestructive` | Codex |
`````

## h013

Source: [docs/plans/2-lvm-refactor/spec.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/spec.md).

`````markdown
The separate host-only test lab is not a production privilege path. Its
fixture setup and cleanup may use only the audited, ledger-validated command
executor in `tools/storage-testing`, running in an explicitly disposable VM or
CI runner with the privileges supplied by that environment. It may never
elevate itself, target an unallocated device, or be called by application code.
Logical actions under test still go through typed UDisks contracts; direct
fixture commands are limited to allocation, teardown, and reset.
`````

## h014

Source: [docs/plans/2-lvm-refactor/spec.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/spec.md).

`````markdown
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
`````

## h015

Source: [docs/plans/2-lvm-refactor/validation.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/validation.md).

`````markdown
| 7 | `34c39c0` | Harness catalog/report/fixture boundary | fixture-boundary/harness-execution | `harness_execution_contract`; `just harness-nondestructive` | Required safe profile writes report and ledger; full profile rejects an ungated host | pass (safe profile) | Codex |
`````

## h016

Source: [docs/plans/2-lvm-refactor/validation.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/validation.md).

`````markdown
| cargo metadata --no-deps --format-version=1 | Seven workspace packages: root app, five published libraries, and non-published tools/storage-testing; no service package. |
`````

## h017

Source: [docs/plans/2-lvm-refactor/validation.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/validation.md).

`````markdown
| just harness-nondestructive | Invokes the runner's `nondestructive` profile with `--require-executed`; its report contains exactly one Passed result for every selected non-destructive case. |
`````

## h018

Source: [docs/plans/2-lvm-refactor/validation.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/validation.md).

`````markdown
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
`````

## h019

Source: [docs/plans/2-lvm-refactor/validation.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/validation.md).

`````markdown
~~~sh
rg -n -i 'storage-service|org\.cosmic\.ext\.Storage\.Service|LogicalClient|sudo|pkexec|vgcreate|vgremove|vgextend|vgreduce|lvcreate|lvremove|lvresize|lvchange|mdadm' src crates resources .github Cargo.toml justfile
rg -n 'Command::new|std::process::Command' tools/storage-testing
~~~
`````

## h020

Source: [docs/plans/2-lvm-refactor/validation.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/2-lvm-refactor/validation.md).

`````markdown
Port and execute every historical storage-testing family:
`````

## h021

Source: [docs/plans/3-logical-ui/baseline.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/3-logical-ui/baseline.md).

`````markdown
The current full-lab command is intentionally not a baseline gate for this
plan: `FullLabExecutor` blocks every selected case until a fixture scenario is
registered. The targeted logical-suite gate described in
[validation.md](validation.md#disposable-fixture-gate) replaces that impossible
claim and must be implemented before final acceptance.
`````

## h022

Source: [docs/plans/3-logical-ui/implementation-plan.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/3-logical-ui/implementation-plan.md).

`````markdown
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
`````

## h023

Source: [docs/plans/3-logical-ui/implementation-plan.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/3-logical-ui/implementation-plan.md).

`````markdown
| Verification | `tests/logical_*`, `crates/storage-{types,contracts,udisks}/tests/*`, `tools/storage-testing/{src,tests}/**`, `justfile`, required logical-suite multi-device Btrfs cases |
`````

## h024

Source: [docs/plans/3-logical-ui/validation.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/3-logical-ui/validation.md).

`````markdown
## Disposable fixture gate

Add this `harness-logical` `justfile` recipe:

```text
harness-logical:
    @test "${STORAGE_TESTING_ENABLE_DESTRUCTIVE:-}" = "1" || { echo "STORAGE_TESTING_ENABLE_DESTRUCTIVE=1 is required in the disposable fixture VM" >&2; exit 1; }
    @artifact_dir=$(cargo run --quiet -p storage-testing --bin lab -- create-artifact --label harness-logical); STORAGE_TESTING_ARTIFACT_DIR="$artifact_dir" cargo run --quiet -p storage-testing --bin harness -- --profile full-lab --suite logical --require-executed
```

It creates a marker-bearing artifact, requires
`STORAGE_TESTING_ENABLE_DESTRUCTIVE=1`, and runs precisely the logical suite.
The suite gains `logical.btrfs.primary_ordering` and
`logical.btrfs.subvolume_ref_conflict` alongside the existing add/remove case.
Every selected logical case must be `Passed`; `Blocked`, omitted, or optional
is not acceptance evidence. The broad `just harness` command is not a gate for
this UI plan until unrelated full-lab suites have executors. The desktop
application itself must never require elevated shell commands.
`````

## h025

Source: [docs/plans/3-logical-ui/validation.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/3-logical-ui/validation.md).

`````markdown
| Fixture execution | `FullLabExecutor` executes—not blocks—`logical.list_entities.schema_integrity`, `logical.lvm.create_resize_delete_lv`, `logical.mdraid.create_start_stop_delete`, `logical.btrfs.add_remove_member`, `logical.btrfs.primary_ordering`, and `logical.btrfs.subvolume_ref_conflict` through a ledgered disposable fixture. The Btrfs cases use three marker-owned 1-GiB loops; only a fixed, ledger-validated fixture `mkfs.btrfs` command may format the first, subsequent topology actions use the typed UDisks test boundary, and successful unmount/detach/artifact cleanup is recorded before passing. |
`````

## h026

Source: [docs/plans/3-logical-ui/validation.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/3-logical-ui/validation.md).

`````markdown
```text
cargo test --locked --test logical_ui_contract --test logical_state_contract --test logical_operations_contract --test sidebar_async_contract
cargo test -p storage-types --locked --test logical_domain_contract
cargo test -p storage-udisks --locked --test logical_adapter_contract
cargo test -p storage-contracts --locked --test logical_contract
cargo test -p storage-testing --locked --test harness_execution_contract
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```
`````

## h027

Source: [docs/plans/4-testing/README.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/4-testing/README.md).

`````markdown
The new layer complements, and deliberately does not replace,
[`storage-testing`](../../../tools/storage-testing/README.md). The existing
harness remains the only place where disposable loop devices and real backend
adapters are exercised. The scenario backend makes UI state, failure paths,
and mutation flows repeatable without touching the host.
`````

## h028

Source: [docs/plans/4-testing/application-workflow-v2-revision-plan.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/4-testing/application-workflow-v2-revision-plan.md).

`````markdown
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
`````

## h029

Source: [docs/plans/4-testing/e2e-revision-plan.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/4-testing/e2e-revision-plan.md).

`````markdown
- This remains a scenario test system: it must not access UDisks, host disks,
  host mounts, rclone configuration, a host browser, or the system D-Bus.
- It complements, never replaces, `storage-testing` and the disposable
  real-backend harness.
- Do not reintroduce a test-only coordinate click, sleep-based wait, mock
  accessibility tree, semantic-only “visual” baseline, or implicit golden
  update as a shortcut around a failing case.
- A broad command that does not select the named capability/case target is not
  evidence. `ui-assert-tests` must verify names before each target executes.
- Any change to the environment lock, capture helper, fonts, renderer,
  selector schema, image threshold, or fixture schema is a reviewed E2E
  contract change and must update tests, baseline evidence, and this plan in
  the same pull request.
`````

## h030

Source: [docs/plans/4-testing/implementation-plan.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/4-testing/implementation-plan.md).

`````markdown
No implementation commit may modify `tools/storage-testing/**` unless a
separate real-harness change is explicitly requested. The existing harness is
not a staging area for test-backend code.
`````

## h031

Source: [docs/plans/4-testing/implementation-plan.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/4-testing/implementation-plan.md).

`````markdown
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
`````

## h032

Source: [docs/plans/4-testing/implementation-plan.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/4-testing/implementation-plan.md).

`````markdown
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
`````

## h033

Source: [docs/plans/4-testing/implementation-plan.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/4-testing/implementation-plan.md).

`````markdown
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
`````

## h034

Source: [docs/plans/4-testing/implementation-plan.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/4-testing/implementation-plan.md).

`````markdown
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
`````

## h035

Source: [docs/plans/4-testing/implementation-plan.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/4-testing/implementation-plan.md).

`````markdown
Review the final diff against `spec.md` and `validation.md` phase by phase.
Confirm package path/name is `crates/test-backend`/`test-backend` everywhere;
release/package commands omit the optional feature; no app/scenario dependency
reaches `tools/storage-testing`; and all required artifacts exist. Resolve
every unmet criterion before accepting the work.
`````

## h036

Source: [docs/plans/4-testing/phase-record.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/4-testing/phase-record.md).

`````markdown
- Commit SHA: `37302df` (`feat(logical): complete logical storage UI`)
- Implementer and reviewer: planning worktree / Codex review
- Scope and reviewed file ranges: root `Cargo.toml`; `justfile`;
  `.github/workflows/ci.yml`; `src/main.rs`; `src/app.rs`;
  `src/operations/{mod,filesystems,image}.rs`; `src/update/**`;
  `src/subscriptions/app.rs`; `crates/storage-contracts/src/**`; and
  `tools/storage-testing/**` boundary documentation.
- Contract/schema/traceability rows changed: none; the Phase-0a lock follows
  this record and is not retroactively treated as baseline code.
- Required named tests listed before execution (command and output artifact):
  `cargo test --workspace --all-features --locked -- --list`; the captured list
  covered 33 root unit tests, 16 root integration tests, 10 `storage-sys` unit
  tests, 4 `storage-contracts` unit tests, and all then-existing crate and
  harness contract targets.
- Commands executed and exit status: `git status --short` (0); `cargo metadata
  --no-deps --format-version=1` (0); `cargo test --workspace --all-features
  --locked -- --list` (0); `git diff --check` (0).
- Generated artifacts and stable locations: this baseline summary; cargo target
  inventory at the command above. Workspace members were root,
  `storage-{btrfs,contracts,sys,types,udisks}`, and `tools/storage-testing`.
- Manual review observations: the root has a process-global
  `SHARED_OPERATIONS` in `src/operations/mod.rs`; `AppModel::init` uses `()`
  flags; operations, models, update paths, and subscriptions call `shared()` or
  `*Client::new()`; host usage/image work remains in
  `src/operations/filesystems.rs`, `src/operations/image.rs`, and
  `src/update/image/dialogs.rs`. `storage-testing` is a non-published workspace
  tool, is not an app dependency, and is invoked by `just harness-nondestructive`.
  CI currently ignores Markdown paths and has build, clippy, fmt, and safe
  harness coverage only.
- Deviations/decisions (links): the Phase-0a bootstrap comprises the
  [schema descriptor](schema-v1.json),
  [contract inventory](contract-surface-v1.toml), and
  [test manifest](../../../tests/ui/required-tests.toml). It is not immutable
  or sign-off ready until the closed DTO/transition catalog completion gate in
  [scenario-schema-v1.md](scenario-schema-v1.md) passes. The concrete E2E
  image lock is intentionally created at the start of Phase 8 according to the
  [environment-lock contract](e2e-environment-v1.md), before any golden exists.
- Result: pass
- Date: 2026-07-31
`````

## h037

Source: [docs/plans/4-testing/spec.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/4-testing/spec.md).

`````markdown
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
`````

## h038

Source: [docs/plans/4-testing/spec.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/4-testing/spec.md).

`````markdown
The existing `tools/storage-testing::spec::LabSpec` is intentionally a
loop-image fixture description. It describes image sizes, partition layout,
and mounts for a destructive or nondestructive real-backend harness run. It
cannot describe dialogs, logical preflights, network schemas, errors, delayed
completions, or a mutable simulated system. Reusing it would couple UI tests to
fixture commands and make the harness a dependency of the desktop application,
which is prohibited.
`````

## h039

Source: [docs/plans/4-testing/spec.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/4-testing/spec.md).

`````markdown
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
`````

## h040

Source: [docs/plans/4-testing/spec.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/4-testing/spec.md).

`````markdown
| `harness-nondestructive` | existing Ubuntu job | continue `just harness-nondestructive`; it remains a real-adapter check |
`````

## h041

Source: [docs/plans/4-testing/validation.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/4-testing/validation.md).

`````markdown
~~~text
just ui-plan-check
python3 tools/ui-testing/assert_tests.py --plan-only
just ui-assert-tests phase=all
cargo test -p storage-contracts --locked --test ui_workflow_contract
cargo test -p test-backend --locked
cargo test -p cosmic-ext-storage --locked --features test-backend --test ui_runtime_contract
cargo test -p cosmic-ext-storage --locked --test scenario_feature_disabled_contract
cargo test -p cosmic-ext-storage --locked --features test-backend --test ui_scenario_contract
cargo test -p cosmic-ext-storage --locked --features test-backend --test application_workflows
cargo test --workspace --all-features --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
just ui-scenario-check
just harness-nondestructive
just ui-e2e
just package-check
~~~
`````

## h042

Source: [docs/plans/4-testing/validation.md](https://github.com/cosmic-utils/cosmic-ext-storage/blob/6a3aec3/docs/plans/4-testing/validation.md).

`````markdown
The E2E capability job starts with a private user runtime directory and D-Bus
session, must not connect to the system bus, and builds the binary inside the
locked image with `test-backend` before launching it using `--backend scenario`.
It records the Containerfile base-image digest, package/font lock, Sway command,
capture/input helpers, viewport, and image-comparison settings in its evidence.
A successful capability run proves no real device is needed and that the
compositor prerequisites work; it does not yet substitute for the per-case
semantic and golden comparisons, `harness-nondestructive`, or a disposable
full-lab run.
`````

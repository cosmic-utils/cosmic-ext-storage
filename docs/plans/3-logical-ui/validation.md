# Validation

## Automated gates

Add named tests; do not rely on a broad Cargo filter that can select nothing.

| Behaviour | Required coverage |
| --- | --- |
| Typed domain migration | `LogicalDisplay` distinguishes known from unknown without a sentinel `0`; every page field is read from the matching `LogicalEntityDetails` variant—not `metadata` or a generic entity/member string. Fixtures cover LVM VG/LV/PV, MD array/member, Btrfs filesystem/device/subvolume, including unknown property reasons. Contract tests prove Btrfs default/delete/snapshot actions accept `BtrfsSubvolumeRef` and reject a raw path/ID constructor. |
| Btrfs topology and determinism | Adapter fixtures group two Btrfs interface objects with the same mixed-case filesystem UUID into one root, preserve both members, choose the specified primary operation target, and map captured `GetSubvolumes`/`GetDefaultSubvolumeID` observations without a mapper D-Bus call. Permuting ObjectManager input and primary-candidate order—including `None` versus `Some` fingerprint ordering—must emit identical public root/member/subvolume/action-target output. Malformed UUIDs, conflicting filesystem-wide values, duplicate subvolume IDs/paths, orphan parents, and cycles retain every row with the prescribed row key, attachment state, and diagnostic rather than arbitrary selection. |
| Snapshot identity | Fixture mapping derives member refs only from partition UUID bound to drive WWN/serial, drive WWN/serial, or loop backing identity. A partition UUID alone and two Btrfs members sharing `Block.IdUUID`/`IdType` never gain a member ref; an identity-less member is readable/inert. `Block.ReadOnly` maps exactly to writable/read-only/unknown state. Discovery and execution use one comparator-selected primary from the same captured snapshot and never fall back to a second matching object. |
| Missing Btrfs support | A Btrfs sidebar candidate with no resolvable native Btrfs root renders an anchored, candidate-specific unavailable state with its Btrfs/module reason, never the first unrelated logical root or a global-source error substituted for that reason. |
| Candidate capture | A sidebar display path is used only by `capture_logical_candidate`; later resolution accepts the same block ID/fingerprint in the load snapshot, and an un-fingerprinted anchor becomes unavailable after any epoch change. No `LogicalState` selection/action field can retain a path as its identity. |
| Detail rendering | Fixture entities exercise every kind, unknown values, the flattened Btrfs header/devices/subvolumes flow, meaningful labels/byte formatting, source warning, and no raw metadata as primary content. |
| Component reuse | Source review proves the logical view uses the existing COSMIC header/action/usage/list primitives (or a documented extraction), active theme spacing and typography, and the approved symbolic icon names; it contains no copied mock CSS or Unicode stand-in icons. |
| Member navigation | A live member path emits the physical sidebar selection; a stale/unknown path is readable and cannot emit an action. |
| Preflight and identity | The UI sends only a `LogicalPreflightRequestKey`; a response key adds the captured UDisks epoch. It accepts preflight only by the request key, stores the returned full key for confirmation, and invalidates it on topology/draft change. The picker only accepts a `Ready` current candidate ref; `Blocked` candidates carry no ref, and no message/form path can construct one from text. Ineligible/signature-bearing/current-member/identity-less devices have an exact disabled reason. A selected Btrfs subvolume/default/snapshot source uses `BtrfsSubvolumeRef`; a changed ID/path/parent/descendant or diagnostic-blocked state conflicts before native submission. |
| Forms | Each action-matrix row produces the exact valid `LogicalAction`, including the audited MD profile. Invalid names, zero/out-of-range or misaligned sizes, missing member, invalid creation destination, invalid/malformed subvolume reference, and unsupported Btrfs resize syntax are inert with an explanation. |
| Confirmation | Each action has the matrix's exact review classification. `ConfirmedLogicalAction` carries the full preflight key; execution captures one fresh completed snapshot, rejects an old key against its epoch, then revalidates from that same snapshot. Collateral scope is shown only where the typed action binds it, excludes the primary target, and is recomputed/reviewed after a conflict. Member-removal/subvolume reviews prove their required fresh ref and no-extra-collateral condition. |
| Async state | `RefreshCoordinator` transition tests, not literal counters, assert clock values, barriers, run IDs, and cause sets. They prove that current action completion alone submits one logical plus one physical post-success request; failures retain the draft and topology. A manual/device-event run begun before success cannot satisfy that request, while a post-success request may coalesce with it exactly once per domain; two action causes may join only a successor that starts after both barriers; a stale completion cannot start or clear a run. |
| Consolidation | No physical Btrfs mutation message reaches a separate `BtrfsClient` workflow; source search/test proves logical actions are the one mutation route. |
| Fixture execution | `FullLabExecutor` executes—not blocks—`logical.list_entities.schema_integrity`, `logical.lvm.create_resize_delete_lv`, `logical.mdraid.create_start_stop_delete`, `logical.btrfs.add_remove_member`, `logical.btrfs.primary_ordering`, and `logical.btrfs.subvolume_ref_conflict` through a ledgered disposable fixture. The Btrfs cases use three marker-owned 1-GiB loops; only a fixed, ledger-validated fixture `mkfs.btrfs` command may format the first, subsequent topology actions use the typed UDisks test boundary, and successful unmount/detach/artifact cleanup is recorded before passing. |

Minimum commands after the tests exist:

```text
cargo test --locked --test logical_ui_contract --test logical_state_contract --test logical_operations_contract --test sidebar_async_contract
cargo test -p storage-types --locked --test logical_domain_contract
cargo test -p storage-udisks --locked --test logical_adapter_contract
cargo test -p storage-contracts --locked --test logical_contract
cargo test -p storage-testing --locked --test harness_execution_contract
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

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

## Manual desktop trace

Perform this trace on a non-production Btrfs filesystem with the UDisks Btrfs
module available, and record the UDisks version/module condition:

1. Open each logical root from the sidebar and confirm its title, type, health,
   capacity/unknown state, members, and applicable flattened sections match the
   discovered topology.
2. Open a multi-device Btrfs filesystem. Confirm it appears once, both devices
   are visible, the default subvolume is marked, and the physical-device link
   returns to the correct disk page.
3. Create a subvolume, create a read-only snapshot, set its default
   subvolume, and refresh. Confirm the tree and contextual completion feedback
   update once per completed action.
4. Exercise Btrfs label, absolute resize, add-device, and remove-device flows
   whenever the selected fixture reports them available. Confirm the review
   text and native Polkit result are visible. When unavailable, record the
   exact blocked reason; do not silently omit the flow. Verify unsupported
   resize syntax stays disabled with its documented reason.
5. Run at least one LVM and one MD non-destructive operation to verify the
   shared shell/forms do not regress their relevant capability controls.
6. Disable or omit the UDisks Btrfs module, select a Btrfs candidate, and
   verify the unavailable page/retry text rather than an unrelated entity.
7. Repeat a form after changing/removing its selected device, changing a draft
   field while preflight is pending, and changing a selected subvolume's
   path/parent. Verify conflict handling retains non-identity input, asks for
   fresh selection/review, and sends no action to a recycled device identity or
   changed subvolume reference.
8. Compare the Btrfs page against [mock.html](mock.html): header, Devices,
   and Subvolumes appear in that order with no logical-page tab rail, metric
   card strip, filesystem details/sources sections, duplicate members section,
   or Activity section. Confirm the production view uses the app's normal
   fonts, spacing, symbolic icons, and tooltips rather than the mock's
   stand-ins.

## Regression checks

- Physical disk, partition, encryption, network, and image pages remain
  unchanged except for the intentional Btrfs handoff link.
- Leaving logical view preserves the cached topology but returns cleanly to
  physical selection, as the existing state contract requires.
- Keyboard focus reaches every section action, menu item, form field, cancel
  button, and confirmation button; tooltip text is not the sole source of
  meaning.
- Confirm all new strings are extracted in the English Fluent catalog and
  missing translations fall back correctly.
- Record the UDisks property/method and normalisation used for every displayed
  Btrfs value against the inventory in [baseline.md](baseline.md#verified-udisks-field-inventory).

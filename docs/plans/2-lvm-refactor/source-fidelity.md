# Source-Fidelity Ledger

**Status:** Required implementation record
**Source:** origin/079-lvm-support at
93094bd4ecea0f7b6236b17e34753470b8ee616d.

## Rule

For feature-branch UI, application logic, and the integration harness, the
source branch supplies the implementation to transplant. The source does not
override an independently changed current-main range: reconcile that range
first under [reconciliation.md](reconciliation.md). For a feature-only range,
begin with literal source content copied into the current-main destination.
Keep every source line unchanged unless one of the following narrowly-defined
reasons applies:

1. the current-main path/module layout changed;
2. the removed project-service transport must be replaced by the typed
   in-process operations façade; or
3. an upgraded compiler/dependency API makes the original line impossible.
4. the listed identity/native-semantics conversion is required to turn a source
   `/dev`/CSV value into an opaque contract ID or to retain a source form that
   has no UDisks-native equivalent as a specifically blocked action; or
5. a source test/lab fixture command must move behind the constrained,
   ledger-validated fixture executor. This exception is limited to fixture
   allocation, reset, and teardown; it never applies to the logical action
   being tested.
6. a current-main asynchronous lifecycle guard is required to suppress stale
   completion or coalesce duplicate refreshes while preserving source-visible
   selection, ordering, defaults, and completion behaviour.
7. a completed three-way reconciliation row requires a current-main range to
   remain in place. This exception is limited to the exact overlapping range;
   it must name the reconciliation row and independently prove preservation of
   the main and feature behaviours.
8. a source harness `Skipped` path must become deterministic profile exclusion,
   `Blocked`, or `Failed` under the required-execution runner contract. This
   exception may not remove a selected case, weaken its fixture/cleanup
   lifecycle, or turn an enabled action into an untested omission.

“Cleaner”, “more idiomatic”, “similar”, “the new architecture prefers it”, or
“main already changed it” are not permitted reasons. The target must be the
same feature, UI, state machine, and test suite—not a new implementation
inspired by the old one or a source transplant that drops main behaviour.

## Required file-transplant map

This is the complete behaviour-bearing inventory for source commits `c8b6631`,
`2c862ff`, and `93094bd`, plus the harness commits. A brace list is an explicit
finite list, not permission to omit a matching file. A source file with no
destination is handled only by the action/transport matrix named in its row.
Every instruction to “copy” in this table applies only to a feature-only range.
For a file or range that changed on both sides, complete its
`reconciliation.md` row first and compose the two behaviours; never overwrite
the current-main file merely to establish a source-first starting point.

| Source file | Destination | Transplant requirement |
| --- | --- | --- |
| storage-app/src/app.rs | src/app.rs | Reconcile the main composition root first, then transplant logical startup/routing and full async startup behaviour, including concurrent physical/network loads. |
| storage-app/src/{controls/mod.rs,controls/logical/mod.rs} | src/{controls/mod.rs,controls/logical/mod.rs} | Copy controls, helpers, source row ordering, icons, and action availability. |
| storage-app/src/{message/app.rs,message/dialogs.rs} | src/{message/app.rs,message/dialogs.rs} | Copy every logical and asynchronous-load message/variant; alter only typed result payloads. |
| storage-app/src/models/{helpers.rs,load.rs,mod.rs,ui_drive.rs,ui_volume.rs} | src/models/{helpers.rs,load.rs,mod.rs,ui_drive.rs,ui_volume.rs} | Reconcile the current-main operations conversion before copying source call-site semantics, including incremental drive loading and timing. |
| storage-app/src/{state/app.rs,state/dialogs.rs,state/logical.rs,state/mod.rs,state/sidebar.rs} | src/{state/app.rs,state/dialogs.rs,state/logical.rs,state/mod.rs,state/sidebar.rs} | Copy logical state, dialog fields, and physical/logical/network loading semantics. Add only documented source-generation and opaque-ID boundary fields. |
| storage-app/src/subscriptions/app.rs | src/subscriptions/app.rs | Copy device-event/spinner subscription semantics without restoring a project signal subscription. |
| storage-app/src/update/{logical.rs,mod.rs,nav.rs,network.rs,btrfs.rs,drive.rs,smart.rs} | src/update/{logical.rs,mod.rs,nav.rs,network.rs,btrfs.rs,drive.rs,smart.rs} | Reconcile every overlap, then copy branch order, refresh timing, and async update semantics; replace every legacy client call with the equivalent current operations call. |
| storage-app/src/update/{image/dialogs.rs,image/ops.rs,volumes/btrfs.rs,volumes/create.rs,volumes/encryption.rs,volumes/filesystem.rs,volumes/mount.rs,volumes/mount_options.rs,volumes/partition.rs} | src/update/{image/dialogs.rs,image/ops.rs,volumes/btrfs.rs,volumes/create.rs,volumes/encryption.rs,volumes/filesystem.rs,volumes/mount.rs,volumes/mount_options.rs,volumes/partition.rs} | Preserve the `2c862ff` client-to-contract call-site semantics already represented by current main; record any divergence rather than assuming main is equivalent. |
| storage-app/src/{views/app.rs,views/logical.rs,views/mod.rs,views/network.rs,views/sidebar.rs,views/dialogs/logical.rs,views/dialogs/mod.rs} | src/{views/app.rs,views/logical.rs,views/mod.rs,views/network.rs,views/sidebar.rs,views/dialogs/logical.rs,views/dialogs/mod.rs} | Copy logical detail/dialog content and the complete async sidebar/network rendering behaviour. |
| storage-app/src/{logging.rs,main.rs} | src/{logging.rs,main.rs} | Preserve source runtime/init semantics that accompany the async port without reviving client/service initialization; retain the current-main composition-root ownership proven by the reconciliation record. |
| storage-app/Cargo.toml | Cargo.toml | Retain main's root package/dependency baseline and add only the reviewed current-main-compatible dependencies needed by the mapped app semantics. |
| storage-app/src/client/** and storage-contracts/src/client/** | _No destination_ | Do not copy any project client/proxy. Current main's operations façade is authoritative; the mapped model/update call sites preserve non-logical behaviour, while every logical public method/error is reproduced by the action matrix and native adapter tests. |
| storage-types/src/{lib.rs,logical.rs} | crates/storage-types/src/{lib.rs,logical.rs} | Copy topology/capability/progress/summary behaviour and its export. Ledger the opaque-ID conversion, preserving source root/child ordering of case-sensitive name then ID and source member discovery order. |
| storage-contracts/{Cargo.toml,src/lib.rs} | crates/storage-contracts/{Cargo.toml,src/lib.rs} | Port only the current-main-compatible contract/export additions; never revive client dependencies. |
| storage-udisks/src/{lib.rs,logical/**} | crates/storage-udisks/src/{lib.rs,logical/**} | Use the source discovery fields/mappers as an inventory. The native adapter is a required replacement implementation; ledger every source display field and prove it via discovery/action fixtures. |
| storage-sys/src/{lib.rs,logical/**} | crates/storage-sys/src/{lib.rs,logical/**} | Copy read-only parser behaviour behind an injected runner. No source mutating handler/tool code is eligible for this destination. |
| storage-testing/** | tools/storage-testing/** | Copy every harness, lab, ledger, binary, test registration, and test case first. Service/client calls become typed setup; fixture-only commands move to the ledger-validated executor without changing lifecycle/test intent. |
| {justfile,resources/lab-specs/**,.github/workflows/ci.yml} | Same root-relative path | Preserve public harness/lab/CI entry-point semantics while removing service lifecycle steps and recording the explicit fixture privilege boundary. |
| storage-service/**, resources/systemd/org.cosmic.ext.storage.service.policy | _No destination_ | Removed architecture. Preserve only its public logical action/error semantics through the typed action matrix; do not copy its process, policy, signal, or privilege implementation. |
| {Cargo.toml,Cargo.lock,README.md,.dockerignore} and historical docs/plans/** | Current-main root files and this plan set | Resolve workspace/lockfile from main, retain only compatible test-tool membership, and rewrite documentation for the serviceless result. Do not copy source-era dependency, service-installation, or historical-plan assumptions. |

The source service handler and contract client have no destination file because
their project-service transport was deliberately removed. Their action
semantics must instead be reproduced exactly by the typed logical action enum
and native UDisks adapter; the mapping is the operation, identity, and
native-semantics matrix in implementation-plan.md.

## Required transplant procedure

Perform these steps for each mapped file before refactoring it:

1. Save the common-ancestor, feature-branch, current-main, and target-baseline
   file content in the implementation review record. Generate the inventory
   from both commit ranges and reconcile every overlap under
   `reconciliation.md` before starting a task:

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

Do not replace an entire source file with a fresh implementation merely because
a few calls need transport adaptation. Isolate replacement calls in the
operations façade/adapter and leave the caller's control flow intact.

## Deviation ledger

Populate this table during implementation. There must be no blank “reason”,
“equivalence evidence”, or “reviewer” cell. A row that cites a broad file
rather than the smallest contiguous range is rejected.

| Source path and lines | Target path and lines | Permitted reason | Reconciliation row | Exact code/UI/logic change | Equivalence evidence | Reviewer |
| --- | --- | --- | --- | --- | --- | --- |
| `storage-types/src/logical.rs` source logical IDs | `crates/storage-types/src/logical.rs` | identity | n/a | Raw source device/path values become canonical opaque IDs and fingerprinted live references. | `logical_domain_contract` | Codex |
| `storage-app/src/update/logical.rs` client calls | `src/operations/logical.rs`, `src/update/mod.rs` | transport | operations façade | Removed LogicalClient calls become typed operations façade requests with generation guards. | `logical_operations_contract`, `logical_state_contract` | Codex |
| `storage-app/src/views/{logical.rs,dialogs/logical.rs}` | `src/views/{logical.rs,dialogs/logical.rs}` | transport / native-semantics | app/view overlap | Action buttons construct only complete typed actions; actions needing a fresh device reference remain visibly blocked rather than accepting a raw path. | `logical_ui_contract`, `cargo check -p cosmic-ext-storage --locked` | Codex |
| `storage-testing/src/{cmd,ledger,harness/**,lab/**}` | `tools/storage-testing/src/**` | fixture-boundary / harness-execution | root workspace row | Arbitrary command execution and skip outcomes are replaced by a closed fixture executor, marker-bearing artifact directory, and Passed/Failed/Blocked report model. | `harness_execution_contract`, `just harness-nondestructive` | Codex |
| `storage-app/src/{app.rs,models/load.rs,state/sidebar.rs}` | corresponding `src/**` files | concurrency / main-preservation | async sidebar overlap | Current in-process operation ownership is retained while source startup loading is made independent and generation-safe. | `sidebar_async_contract` | Codex |

## Mandatory comparison review

For every mapped file, record the source and target revision and inspect:

~~~sh
git diff --no-index --word-diff=porcelain \
  <source-export-path> <target-path>
~~~

Treat the following as defects unless explicitly ledgered:

- a source UI control, tab, field, label, icon, member row, dialog, action,
  status/result state, or confirmation is absent or materially rearranged;
- a source message variant, state field, update branch, validation condition,
  selection rule, refresh sequence, or test scenario is absent or altered;
- a source harness fixture, ledger protection, lab step, command entry point,
  test family, or individual test is omitted; and
- any new behavior which did not exist in the source branch, except the native
  UDisks transport/error adaptation, opaque-ID conversion, or constrained
  fixture-executor move required by this plan; or the ledgered current-main
  stale-result/refresh-coalescing guard.

The comparison is not a license to preserve obsolete service imports or unsafe
raw identity/fixture calls. Those lines are replaced only at the documented
transport, opaque-identity, native-semantics, fixture-boundary, or concurrency
boundary, with the exact equivalent described in the ledger.

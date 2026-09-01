# Three-Way Preservation Record

**Status:** Required before implementation

This graft has two authorities which must both survive:

- `main` at `0ba27cc2caac19acb7a8d98747f8b65058dab877` supplies every
  post-ancestor service-removal, workspace, dependency, and in-process
  operation change; and
- `origin/079-lvm-support` at
  `93094bd4ecea0f7b6236b17e34753470b8ee616d` supplies the logical feature,
  asynchronous sidebar, harness, and lab behaviour.

The common ancestor is `3f8c340e8a6cc17ea854ef244b0c3b3bb1028128`. A feature
source file is not automatically authoritative when `main` changed the same
base file. Neither a direct file copy nor a line-oriented conflict resolution
is sufficient evidence that both behaviours remain.

## Required reconciliation procedure

Before editing a mapped file, export the three inputs and create one row in the
table below for every changed overlap:

~~~sh
base=3f8c340e8a6cc17ea854ef244b0c3b3bb1028128
main=0ba27cc2caac19acb7a8d98747f8b65058dab877
feature=93094bd4ecea0f7b6236b17e34753470b8ee616d

git diff --find-renames --name-status "$base".."$main"
git diff --find-renames --name-status "$base".."$feature"
git diff --word-diff=porcelain "$base".."$main" -- <base-path>
git diff --word-diff=porcelain "$base".."$feature" -- <base-path>
~~~

Normalize the deliberate layout move `storage-app/src/**` to `src/**` and the
library move `storage-*/**` to `crates/storage-*/**` before deciding whether
two files overlap. For every overlap, record both behaviour deltas and the
target implementation/test which retains them. A source-fidelity ledger entry
does not replace this record: it explains feature-source deviations, whereas
this record proves that independent main behaviour was not lost.

## Known overlap inventory

After normalizing the `storage-app/src` move, the following 35 feature-changed
application files also have a main-side change and therefore require a
reconciliation row before transplantation:

~~~text
src/app.rs
src/controls/mod.rs
src/logging.rs
src/main.rs
src/message/{app.rs,dialogs.rs}
src/models/{helpers.rs,load.rs,mod.rs,ui_drive.rs,ui_volume.rs}
src/state/{app.rs,dialogs.rs,mod.rs,sidebar.rs}
src/update/{btrfs.rs,drive.rs,mod.rs,nav.rs,network.rs,smart.rs}
src/update/image/{dialogs.rs,ops.rs}
src/update/volumes/{btrfs.rs,create.rs,encryption.rs,filesystem.rs,mount.rs,mount_options.rs,partition.rs}
src/views/{app.rs,mod.rs,network.rs,sidebar.rs}
src/views/dialogs/mod.rs
~~~

The root `Cargo.toml`, `Cargo.lock`, `README.md`, `justfile`, project policy
path, and the old service/client paths also require explicit disposition. In
particular, a source client/proxy deletion is satisfied only by retaining
main's operations façade; it is never grounds to restore or replace that
façade with old client code.

## Required root and removed-path dispositions

These paths are not exempt merely because they do not normalize into the
35-file application inventory. Record the final commit/range and evidence for
each in the reconciliation record before it is changed.

| Path/group | Current-main behaviour to preserve | Feature behaviour to preserve | Required treatment |
| --- | --- | --- | --- |
| `Cargo.toml`, `Cargo.lock` | Current workspace graph, package upgrades, and no service package. | Non-published `storage-testing` membership when its harness is restored. | Modify main's manifests minimally; retain current versions unless the new test member needs a reviewed additive resolution. Never copy the feature lockfile. |
| `README.md`, `docs/plans/**` | Main's serviceless install/runtime guidance. | Logical feature, lab, and validation guidance. | Compose current documentation; historical service instructions remain absent. |
| `justfile`, `.github/workflows/**`, `resources/lab-specs/**` | Main's normal checks and no service lifecycle. | Harness/lab entry points and CI coverage. | Preserve main checks and add the required-execution harness profiles; no service start, policy installation, or skipped selected test. |
| `storage-app/src/client/**`, `storage-contracts/src/client/**`, `storage-service/**`, policy/systemd resources | Deliberate removal of project transport, privilege, and process boundaries. | Public logical action/error behaviour formerly reached through that transport. | Keep these paths absent. Reproduce public behaviour only through typed contracts, the app façade, and UDisks-native adapter tests. |
| Layout moves (`storage-app/src/**` → `src/**`, `storage-*/**` → `crates/storage-*/**`) | Current-main module ownership and imports. | Every mapped feature range. | Reconcile after path normalization; a rename never converts an independent overlap into a feature-only copy. |

## Target rule

Classify each range as one of the following:

| Classification | Required target treatment |
| --- | --- |
| Main-only | Retain the current-main range verbatim, apart from a separately ledgered mechanical move. |
| Feature-only | Transplant the feature range literally, then apply only an allowed source-fidelity adaptation. |
| Same semantic change | Retain one implementation and prove equivalence with the existing main test plus the source behaviour test. |
| Independent overlap | Compose both behaviours in the target; cite a focused test for each and an integration test for their interaction. |
| Direct contradiction | Keep main's serviceless/security invariant, record the narrowly permitted replacement, and prove the feature-visible result through the typed native operation. |

No row may be closed with “copied source”, “main already changed it”, or a
broad file-level reference. The target must identify the smallest changed
range, the retained main behaviour, the retained source behaviour, and the
evidence for both.

| Base path/range | Current-main delta and retained behaviour | Feature delta and retained behaviour | Classification | Target path/range | Main evidence | Feature evidence | Reviewer |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `Cargo.toml` workspace members | Main keeps six publishable app/library packages without a service. | Feature adds a non-published harness package. | Independent overlap | `Cargo.toml` workspace members; `Cargo.lock` storage-testing entry | `cargo metadata --no-deps --format-version=1` | `harness_execution_contract` | Codex |
| `src/app.rs` startup task batch | Main owns in-process application composition. | Feature loads logical/physical/network sections asynchronously. | Independent overlap | `src/app.rs`, `src/update/mod.rs`, `src/state/sidebar.rs` | `sidebar_async_contract` | `sidebar_async_contract` | Codex |
| `src/update/mod.rs` logical actions | Main has no project client/service transport. | Feature has logical action completion and refresh semantics. | Direct contradiction | `src/operations/logical.rs`, `src/update/mod.rs` | `logical_operations_contract` | `logical_state_contract` | Codex |
| `src/views/{logical.rs,dialogs/**}` | Main dialog host and controls remain authoritative. | Feature supplies logical operation affordances and review flow. | Independent overlap | `src/views/logical.rs`, `src/views/dialogs/logical.rs`, `src/views/app.rs` | `cargo check -p cosmic-ext-storage --locked` | `logical_ui_contract` | Codex |
| `justfile`, CI and root documentation | Main retains normal workspace checks and no service lifecycle. | Feature adds harness entry points and safe CI execution. | Independent overlap | `justfile`, `.github/workflows/ci.yml`, `README.md` | `cargo test --workspace --all-features --locked` | `just harness-nondestructive` | Codex |

## Reconciliation acceptance gate

Before a commit that touches an overlapping file:

1. its reconciliation row is complete;
2. the source-fidelity ledger separately accounts for every non-verbatim
   feature-source range;
3. all main-only ranges remain present in the target or have a documented,
   tested serviceless replacement; and
4. the row names distinct evidence for main preservation and feature parity.

At final review, inspect both target comparisons:

~~~sh
git diff --word-diff=porcelain "$main" -- <target-path>
git diff --word-diff=porcelain "$feature" -- <source-export-or-mapped-target>
~~~

An unexplained loss from either side is a merge blocker.

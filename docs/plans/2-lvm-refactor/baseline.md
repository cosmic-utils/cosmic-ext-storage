# LVM Refactor Baseline

**Captured:** 2026-07-23
**Target:** `main` at `0ba27cc2caac19acb7a8d98747f8b65058dab877`
(`refactor: remove storage service (#116)`)
**Feature tip:** `origin/079-lvm-support` at
`93094bd4ecea0f7b6236b17e34753470b8ee616d`
(`feat: async sidebar loading and clean workspace checks`)
**Common ancestor:** `3f8c340e8a6cc17ea854ef244b0c3b3bb1028128`

## Current-main facts

> Historical testing-infrastructure note superseded by [Testing V2](../5-testing-v2/spec.md); [original record](../5-testing-v2/legacy-harness-history.md#h001).

## Feature-branch contents

The branch has eight commits after the common ancestor:

| Commit | Intent | Integration disposition |
| --- | --- | --- |
| `4d25b66` | Old logical planning documents | Replace with this plan-set; do not replay. |
| `c8b6631` | Logical types, discovery, service API, app UI | Port its domain/UI intent through the new contracts and operations boundary. |
| `06732f9` | Repair old `just check` | Superseded by the root `justfile` check recipe. |
| `9094a29` | Initial lab and harness | Recreate all harness/lab behaviour after replacing service/client transport and moving privileged fixture lifecycle behind the constrained test-only executor. |
| `2c862ff` | Move D-Bus clients into contracts, add harness CI | Drop the client move; reconsider the CI job after the harness port. |
| `4b53757` | Integration-suite planning docs | Consolidate useful scenarios into this validation plan. |
| `5b0d8a6` | Host-only testing suite and lab refinement | Port every fixture, scenario, ledger, and lab refinement against the in-process test API and the ledger-validated fixture executor. |
| `93094bd` | Async sidebar loading | Port its behaviour last, against current `src/` and typed operations. |

## Rebase assessment

An isolated `git rebase --onto main 3f8c340` was attempted from the feature
tip. The first commit conflicted only in `README.md`, where old "Later" text
overlapped the post-service-removal documentation. Skipping that historical
documentation commit exposed the material conflict in `c8b6631`:

- renamed paths: `storage-app/**` must now be `src/**`; retained libraries must
  be under `crates/**`;
- content conflicts in `crates/storage-sys/src/lib.rs`,
  `crates/storage-types/src/lib.rs`, `src/app.rs`, `src/update/mod.rs`, and the
  app view/sidebar routing;
- add-in-renamed-directory conflicts for the new logical state, update, and
  view files;
- modify/delete conflicts for `storage-app/src/client/mod.rs` and all changed
  `storage-service/**` registration files, because those boundaries no longer
  exist.

This proves a mechanical rebase would either revive deleted architecture or
hide a substantial rewrite behind conflict markers. The implementation must
use the feature branch as a reference and produce new, reviewable commits on a
fresh branch based on `main`.

## Three-way preservation baseline

This is not a two-way source transplant. After normalizing main's
`storage-app/src/**` to `src/**` layout move, 35 feature-changed application
files also contain an independently changed main range. The affected groups
are app/bootstrap, messages, models, state, updates, views, and sidebar
routing; `Cargo.toml`, `Cargo.lock`, `README.md`, `justfile`, policy, service,
and client paths also need an explicit disposition. Before porting any such
file, complete the per-range record in `reconciliation.md`. A source-fidelity
comparison alone cannot prove the retained current-main operation and
composition-root behaviour.

## Preserved safety constraints

- UDisks2 native Polkit remains the only supported application privilege path.
  No project service, policy, helper, socket, `sudo`, or shell elevation may be
  added. The host-only disposable test lab may retain its ledger-validated
  fixture allocation/reset/teardown executor; it is not an application path and
  cannot self-elevate.
- Calls outside the composition root use `storage-contracts` and
  `storage-types`, never a concrete adapter or D-Bus proxy.
- Mutating logical commands must be explicit typed requests, validate all
  device/mount inputs, and surface `PermissionDenied`, `Unsupported`, or
  `Unavailable` to the UI. They must not accept JSON command fragments or
  arbitrary command/argument strings from the application.
- `Cargo.toml` and `Cargo.lock` are resolved from the current root workspace;
  do not attempt a textual three-way merge of the feature branch lockfile.

# Phase record template

Create one section per completed phase. A phase is incomplete until every
required field is present; `N/A` is not permitted without a linked decision
record explaining why the requirement moved to a later phase.

## Phase 0 — pre-implementation baseline

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

## Phase <number> — <title>

- Commit SHA:
- Implementer and reviewer:
- Scope and reviewed file ranges:
- Contract/schema/traceability rows changed:
- Required named tests listed before execution (command and output artifact):
- Commands executed and exit status:
- Generated artifacts and stable locations:
- Manual review observations:
- Deviations/decisions (links):
- Result: pass / fail
- Date:

## Revision R0–R2 — executable scenario foundation

- Commit SHA: pending implementation commit.
- Implementer and reviewer: Codex implementation / review pending.
- Scope and reviewed file ranges: `crates/test-backend/{src,tests}/**`;
  `tests/ui/scenarios/**`; `src/{runtime,operations,app}.rs`;
  `src/views/app.rs`; root scenario-runtime tests; and
  `e2e-revision-plan.md`.
- Contract/schema/traceability rows changed: the executable fixture loader now
  accepts only schema v2 with typed disk, partition, filesystem, LUKS,
  process, network, virtual workflow, and logical-preflight data. Each of the
  eight required fixtures now has explicit state rather than a name-only
  placeholder.
- Required named tests listed before execution: test-backend schema, physical,
  logical/network, workflow, and control tests; root scenario runtime/control
  tests; and no-feature scenario rejection tests.
- Commands executed and exit status: `cargo test -p test-backend --locked`
  (0); `just ui-scenario-check` (0); root scenario/control/no-feature test
  commands (0); format check pending final review.
- Generated artifacts and stable locations: none committed. Scenario control
  uses a private Unix socket and optional overlay only when explicitly passed
  to a `test-backend` launch.
- Manual review observations: scenario runtime now has a visible test marker;
  virtual clock, atomic overlay replacement, reload, and diagnostics are
  exposed through an authenticated length-prefixed private control protocol.
  `shared()` no longer constructs a production adapter lazily, so a scenario
  task cannot fall back to host operations. The remaining compatibility cell
  and call-site migration are still tracked technical debt; R2 is not recorded
  as fully complete until those clients carry `Arc<StorageOperations>` directly.
- Deviations/decisions (links): R3 uses the pinned Sway replacement recorded in
  [the E2E revision plan](e2e-revision-plan.md#11-r3-compositor-capability-determination--2026-08-01).
- Result: partial — the v2 fixture/control foundation and locked compositor
  capability gate are complete. The obsolete Python runner and JSON-only
  goldens have been removed, while the semantic v2 case executor and reviewed
  PNG/accessibility goldens remain outstanding. Global-client removal also
  remains outstanding.
- Date: 2026-08-01

## Final acceptance record

- Required CI run URLs and job names:
- Deliberately failing semantic-case run URL and artifact:
- Deliberately failing pixel-baseline run URL and artifact:
- Branch-protection confirmation and responsible maintainer:
- Clean workflow run URL and artifact manifest:
- Final reviewer sign-off:

# Phase record template

Create one section per completed phase. A phase is incomplete until every
required field is present; `N/A` is not permitted without a linked decision
record explaining why the requirement moved to a later phase.

## Phase 0 — pre-implementation baseline

> Historical testing-infrastructure note superseded by [Testing V2](../5-testing-v2/spec.md); [original record](../5-testing-v2/legacy-harness-history.md#h036).

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

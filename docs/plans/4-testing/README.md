# UI scenario testing

**Status:** Phase 0 baseline recorded; Phase 0a is blocked on completing the
closed DTO/transition catalog described in
[scenario-schema-v1.md](scenario-schema-v1.md). Feature implementation begins
only after that catalog, `python3 tools/ui-testing/assert_tests.py --plan-only`,
and the Phase-0a manual review pass on the intended implementation commit.

## Purpose

This plan adds a safe, editable storage backend for manual and automated UI/UX
testing. A versioned scenario file describes the application-visible storage
world; the application is then launched against that world instead of UDisks,
local Btrfs tooling, rclone, or host filesystems.

The new layer complements, and deliberately does not replace,
[`storage-testing`](../../../tools/storage-testing/README.md). The existing
harness remains the only place where disposable loop devices and real backend
adapters are exercised. The scenario backend makes UI state, failure paths,
and mutation flows repeatable without touching the host.

## Decisions

- Add a non-published `test-backend` workspace crate at `crates/test-backend`.
  It implements
  the same `storage-contracts` traits that production adapters implement.
- Define UI worlds in human-editable, versioned TOML files under
  `tests/ui/scenarios/`. A scenario has typed state, deterministic operation
  behaviour, and optional fault/event scripts.
- Make backend selection explicit at the application composition root. The
  normal binary uses real backends; a feature-gated `--backend scenario
  --scenario <file>` launch uses the test backend. Test mode has no fallback
  to a real adapter.
- Remove the remaining global-operation and direct-host-storage seams from UI
  paths so every storage-facing action reaches an injected contract.
- Run reducer/composition tests without a desktop session, then run an
  accessibility-driven Wayland smoke and visual-regression suite in CI using
  the same scenario files.

## Plan documents

- [Specification](spec.md) defines the boundary, backend behaviour, scenario
  format, and CI contract.
- [Scenario schema v1](scenario-schema-v1.md) is the normative, versioned
  fixture and overlay grammar. It must be complete before implementation code
  begins.
- [Schema descriptor](schema-v1.json) and
  [contract-surface inventory](contract-surface-v1.toml) are the bootstrap
  machine-readable Phase-0a schema and method-operation lock. They are not
  ratified until the closed-catalog completion gate passes.
- [Traceability matrix](traceability.md) maps every UI entry point and contract
  method to its scenario operation, state model, fixture, and automated gate.
- [Implementation plan](implementation-plan.md) breaks the work into safe,
  reviewable delivery phases.
- [Validation](validation.md) lists the required test names, commands, CI
  artifacts, and manual testing trace.
- [Phase record](phase-record.md) is the required evidence template for each
  completed phase.
- [E2E environment lock contract](e2e-environment-v1.md) fixes the exact lock
  format that Phase 8 must materialize before it creates goldens.
- [E2E revision plan](e2e-revision-plan.md) records the required remediation
  for the current placeholder runner, fixtures, and metadata-only goldens. It
  is the implementation authority for replacing them with a Rust AT-SPI and
  pixel-visual test system.
- [Application-workflow v2 revision plan](application-workflow-v2-revision-plan.md)
  defines the deterministic, headless layer that exercises `AppModel`-owned
  workflow reducers and selected scenario adapters without a renderer,
  compositor, or accessibility action path. It is deliberately distinct from
  both backend-contract coverage and accessibility/visual E2E.

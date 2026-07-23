# 2 — LVM Refactor and `079-lvm-support` Integration

**Status:** Planned

This plan-set brings `origin/079-lvm-support` onto `main` after the storage
service removal and workspace/dependency upgrade. It is the authority for the
rebase and merge. The older logical-volume and testing plans on the feature
branch are historical inputs, not merge targets.

## Documents

- [baseline.md](baseline.md) records the exact branch relationship, observed
  rebase conflicts, and constraints inherited from `main`.
- [spec.md](spec.md) defines the resulting logical-storage architecture and
  scope.
- [implementation-plan.md](implementation-plan.md) gives the ordered port and
  commit strategy.
- [reconciliation.md](reconciliation.md) defines the mandatory three-way
  preservation record for changes made independently on `main` and the feature
  branch.
- [source-fidelity.md](source-fidelity.md) defines the literal-transplant
  standard and the mandatory deviation ledger.
- [validation.md](validation.md) defines the automated and desktop-session
  acceptance gates.

## Non-negotiable rule

`main`'s in-process architecture wins. The merged result must not restore
`storage-service`, `storage-macros`, D-Bus clients/proxies, project D-Bus
signals, systemd units, project Polkit policies, `sudo`, or a privilege helper.
Logical operations run through typed contracts and application operations; any
operation unavailable to the desktop user is disabled with an actionable reason
rather than being routed through a replacement daemon.

## Source-fidelity rule

For feature-branch UI, application state, update logic, and harness behaviour,
the feature branch is the source implementation. Its additions must be
transplanted literally wherever the serviceless boundary permits; a new
implementation that merely resembles the old UI or logic is not acceptable.
That rule never authorizes overwriting an independently changed `main` range:
the three-way preservation record in [reconciliation.md](reconciliation.md) is
required first. The only permitted deviations are the narrowly-defined
current-main layout, service transport, opaque-identity/native-semantics,
fixture-boundary, harness-execution, and async-concurrency adaptations in
[source-fidelity.md](source-fidelity.md); each must be recorded there with
equivalence evidence.

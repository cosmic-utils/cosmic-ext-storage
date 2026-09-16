# Non-rendered closing pass

Date: 2026-09-16. Branch: `4-ui-testing`. Validation in progress.

## Approved acceptance policy

The user explicitly approved an honest no-regression baseline after this pass.
This document supersedes outstanding statements in the other Testing V2 plans
that near-100% coverage must block merging during the rendered-UI pause. It does
not alter historical results or declare the original full-UI plan complete.

Default non-rendered acceptance requires passing host and native tests, matching
source/build/profile provenance, all production packages, and line/function
coverage at or above a committed measured floor for each package and workspace.
The baseline records executed evidence; CI cannot update it automatically.
Repeated native measurements may differ: floors must be observed ratios from
valid runs, not arbitrary tolerances or invented percentages.

All production Rust sources remain inventoried. New production sources need
review. Previously unmapped Rust and unmeasured Python/shell sources remain
explicit debt; new/changed such sources require explicit baseline review, using
content hashes. These hashes are a review guard, **not coverage measurement**.
No claim is made that the mapped Rust percentage measures support scripts.

The strict 98–100% and changed-code results remain in `long_term_failures`.
`--policy target` enforces them. Full-UI mode only permits that strict policy
and still requires all eight real UI cases. Baseline policy cannot waive a
failed test, missing package/profile, stale evidence, or disabled-UI mismatch.
Execution captures the baseline hash before instrumentation; reports cannot be
relabeled from target to baseline or reused against a changed baseline.

## Fixes and scope

These are application fixes, not additional libcosmic/iced patches.

- **Network configuration safety:** rename creates the destination before
  deleting the original, preserving the original on a name conflict. A failed
  source deletion attempts to remove only the newly created destination and
  reports cleanup failure. Selection follows a successful rename. Tests cover
  successful rename, conflict preservation and duplicate completion; the
  compensation-failure branch remains uncovered.
- **Network ordering and scope:** loads carry request IDs; stale success/error
  and duplicate loads cannot overwrite current state. Confirmed deletion checks
  its reviewed dialog and in-flight identity, removes only the matching scope,
  and invalidates outstanding pre-delete loads. Same-name other-scope entries
  survive. Tests cover success, backend failure and stale completions.
- **Volume forms:** partition delete/edit/resize and mount-options operations
  now use the application's selected backend context, not global native clients.
  Submission validates resize bounds and selected delete targets before setting
  running state. Busy unmount results prevent encrypted partition deletion.
  Completion IDs protect replacement dialogs and duplicate submission. Actual
  scenario-adapter unsupported results are tested as errors, not invented
  successful operations; native tests exercise supported mutations separately.
- **Passphrase change:** selected backend context and completion IDs are shared
  by normal application and tests; empty/mismatched/valid submissions, cancel,
  unsupported errors and late completion are tested. Debug output for this
  dialog/message redacts secrets. The native LUKS test also verifies wrong-current
  rejection, password rotation, old-secret rejection and new-secret unlock/lock.
  This is not a blanket secret-redaction audit or rendered secret transport.
- **Value boundaries:** 57 rstest cases cover storage aliases, partition flags,
  nested volume state, byte ranges and size conversions. Tests exposed silent
  float-to-integer saturation: size parsing now rejects negative/non-finite/
  overflowing values and extra tokens. Valid units remain supported.

The new tests execute normal handlers under a no-global-backend guard and use
rstest fixtures. They do not introduce another workflow reducer or mock runner.
The rename conflict and numeric parser regressions were demonstrated failing
before their fixes. Exact required names are registered in the existing
manifest, traceability contract and validation list.

## Deliberately not claimed complete

Rendered behavior, accessibility, compositor lifetime and visual review remain
deferred. Pins, quarantine and upstream work are unchanged. Native-only volume
handlers not migrated here, some asynchronous form-opening races and rename
compensation failures remain follow-up targets. Support-script coverage needs
real subprocess/container collection before it can be reported as measured.
Aggregate coverage is not a proof of all storage safety behaviors.

## Final evidence

Pending fresh host/native measurement, recorded baseline, and implementation-head
CI. Do not interpret this document's existence as a green merge recommendation.

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

Source checkpoint: `9553a6f9e199304febaba15c9048f76682a0438f`.
Fresh non-rendered run `run-3dbfptjx`: 448 host tests and all 17 selected native
bridge cases passed, including the expanded LUKS test and private-container
shell contract. The native bridge's one non-selected host contract is not a
missing native case. Host-only runs intentionally leave opt-in/native tests
ignored; the collector runs the selected native cases separately.

There are now 77 real production-handler cases (61 before this pass) and 57 new
storage-value cases. All 213 required names resolve against built test targets.
Strict all-feature/all-target Clippy, formatting and the normal no-default-feature
build pass. Python policy/support regressions: 45 pass, two explicit container-only
checks skipped in host invocation; the lab runs its container shell contract.

Repeat `run-7h35tjgf` passed the same 448 host / 17 native cases. Both runs used
the actual PR base `0ba27cc2caac19acb7a8d98747f8b65058dab877`, no display/bus
environment, and `UI_E2E_ENABLED=0`. Both have successful host/lab evidence,
deferred UI, valid provenance and only expected long-term acceptance failures.
The target-policy exit code is 1 because those targets are unmet, not a test failure.

| Mapped Rust scope | Lines | Functions |
| --- | --- | --- |
| Workspace | 12,424/28,518 (43.57%) | 1,467/3,358 (43.69%) |
| Application | 4,871/15,930 (30.58%) | 482/1,533 (31.44%) |
| storage-types | 661/1,056 (62.59%) | 113/182 (62.09%) |

The previous workspace/app line results were 40.04% / 25.61%. The repeat adds
one covered UDisks line (3,243 versus 3,242); all other counters are identical.
The committed baseline takes the lower observed ratio for each package and
workspace, with no rounding allowance. `coverage-baseline.json` records every
scope, source inventory, reviewed hashes and both acceptance-report digests.
47 unmapped Rust files and 15 unmeasured support scripts remain visible.

Saved local reports: `target/coverage/baseline-evidence/run-3dbfptjx/` and
`target/coverage/non-rendered/` for the repeat, with raw evidence in their
respective `run-*` directories. The first is an artifact copy, not a new run.
Fresh CI will execute using the committed baseline; target-policy evidence is
not relabeled as a passing baseline-policy run.

Pending implementation-head CI. This checkpoint is not yet a green merge recommendation.

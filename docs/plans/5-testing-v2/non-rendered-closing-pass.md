# Non-rendered closing pass

Date: 2026-09-16. Branch: `4-ui-testing`. Implementation and baseline calibration complete;
merge requires green checks on [PR #117](https://github.com/cosmic-utils/cosmic-ext-storage/pull/117).

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
workspace across local and CI runs, with no rounding allowance.
`coverage-baseline.json` records every scope, source inventory, reviewed hashes
and all three acceptance-report digests.
47 unmapped Rust files and 15 unmeasured support scripts remain visible.

Saved local reports: `target/coverage/baseline-evidence/run-3dbfptjx/` and
`target/coverage/non-rendered/` for the repeat, with raw evidence in their
respective `run-*` directories. The first is an artifact copy, not a new run.
Fresh CI will execute using the committed baseline; target-policy evidence is
not relabeled as a passing baseline-policy run.

### CI calibration and merge condition

[CI calibration run 35133799269](https://github.com/cosmic-utils/cosmic-ext-storage/actions/runs/35133799269)
at `d442432832048de0d3a6cfcd2747d3222d620112` passed every functional check,
448 instrumented host tests and all 17 native cases. Coverage provenance passed;
the only acceptance failures were UDisks/workspace line/function floors calibrated
solely on local runs. App coverage was exactly identical. The CI source and build
input hashes match both local runs, and report/policy hashes were checked.

CI observed UDisks 3,209/5,231 lines and 369/767 functions; workspace
12,422/28,518 lines (43.56%) and 1,466/3,358 functions (43.66%). CI exercised more
fixture device-node/capability setup (897 lab lines versus 869 locally) and three
additional scanner lines. The local run exercised encrypted-partition discovery,
NVMe/nonrotating-drive branches and transient vanished-LVM-group recovery absent
from CI. These are environment/ordering-dependent paths, not a loss of application
test execution. In particular, incidental host encrypted-partition discovery is
**not** a substitute for a dedicated owned encrypted-partition discovery test;
retain that as a follow-up gap.

The baseline now records the lower observed per-scope ratios across these three
runs. This is explicit initial local/CI calibration, not permission to lower a
future baseline whenever tests regress. No test failure or provenance error is
waived, and the historical CI result remains failed. Raw CI profiles/ELFs were
validated by the checker in CI; the downloaded report artifact is not claimed to
contain those large raw files for independent local revalidation.

Merge condition: a **fresh** CI run must pass with this final captured baseline.
The PR checks are the authoritative result for the pushed head. A documentation-
only follow-up does not change the recorded build/test inputs; any source, test,
runner or baseline change requires fresh matching evidence. No main-branch merge
or repository protection change is part of this pass.

### Deterministic hardware classification follow-up

[CI run 35135871365](https://github.com/cosmic-utils/cosmic-ext-storage/actions/runs/35135871365)
at `3811033` again passed all tests and provenance, but UDisks/workspace line
coverage differed by two lines. Comparing LCOV—not guessing from totals—showed
the runner switched from ATA/unknown-rotation branches to NVMe/nonrotating
branches, while transient vanished-LVM recovery ran this time.

Rather than lower the baseline again, ten rstest cases now directly exercise
the existing pure `infer_connection_bus` function: loop precedence, case-insensitive
NVMe, MMC/MMC block paths, optical paths/flags, USB model/vendor, ATA and empty
fallback. No production behavior changed. The hardware-dependent classifier is
now covered deterministically without a disk or D-Bus service. Rstest reuses the
existing locked workspace version; no dependency version was upgraded.
These cases are registered in the required-test manifest and validation list.
Rotation-property and transient-discovery paths remain native variability; the
baseline remains exactly the previous reviewed three-run floor. Fresh coverage
and CI must validate it with the additional tests, not relabel earlier evidence.

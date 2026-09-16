# Non-rendered regrouping: execution record

2026-09-16, branch `4-ui-testing`. Implementation/source revision:
`1b3aec6ddc2a6cc67eb5d9472f728df926a11bec`. PR #117 comparison base:
`0ba27cc2caac19acb7a8d98747f8b65058dab877` (`main`, not this branch).
Subsequent documentation-only commits do not change the captured build/test
inputs. The collector validates their content hashes, not a guessed branch name.

## Result and scope

The default-off rendered-test boundary and production-handler migration are
implemented. Backend tests and actual application business logic execute without
a window, compositor or accessibility service. The separate workflow reducers,
WorkflowHarness, duplicate state and misleading facade tests are removed.
Rstest fixtures and the existing private Testcontainers native lab remain.
See [per-family traceability](production-logic-traceability.md) for each app fix,
its scope, intentional red/green checks and retained native-only paths.

This is **not** full Testing V2 acceptance. Numerical and support-source coverage
gates remain red; rendered/visual acceptance is deferred. No thresholds or source
exclusions were relaxed. Libcosmic/iced pins, fork patches and quarantine remain
unchanged. No upstream PR, dependency upgrade or additional branch was created.

## Executed verification

Fresh command, with `DISPLAY`, `WAYLAND_DISPLAY` and `DBUS_SESSION_BUS_ADDRESS`
removed from the environment:

```sh
UI_E2E_ENABLED=0 python3 tools/testing/run_coverage.py --base origin/main --mode non-rendered
```

Evidence: `target/coverage/non-rendered/run-cb4tommj`; reports in its parent:
`evidence.json`, `acceptance.json`, `summary.json`, `lcov.info`, `html/index.html`.
The run records 89 profiles in 11 matching build groups. Host exit 0, lab exit 0,
UI exit null/status deferred, acceptance exit 1. Provenance, report hashes,
captured mode/base and mandatory host/native test-source checks passed; the
remaining failures are numerical, unmapped-source and support-measurement gates.
The scoped JSON report is 23,712,082 bytes, not a dependency-graph export.

| Check | Local result |
| --- | --- |
| All-feature workspace tests, instrumented, headless | 375 passed, 0 failed; 36 explicit lab/child-process-only ignored entries |
| Application library tests | 120 passed, including 61 real-handler cases |
| Private native bridge | 17 passed; 1 unselected ordinary host-only bridge test |
| Native container shell contract | Passed separately inside the owned lab image |
| Required-name inventory | 140 exact named checks; planned UI inventory is not executed UI evidence |
| Python execution/coverage contracts | 42 discovered: 40 passed, 2 explicit container-only skips; native skip exercised separately above |
| Strict workspace Clippy, all features/targets | Passed with `-D warnings` |
| Normal app build without default features | Passed |
| Rustfmt and patch whitespace | Passed |

Native evidence includes real partition/filesystem/image/LUKS/Btrfs/LVM/MD/SFTP
outcomes, owned cleanup, actual root application registry dispatch, deliberate
inner-test failure propagation and stale-selector rejection. Host ignores are
not silently treated as native success: the separate bridge is mandatory.

Local logs: `/tmp/non-rendered-verified-coverage.log`,
`/tmp/non-rendered-final-{lib,python,names,clippy,normal}.log`.
These and `target` artifacts are local/ignored, not committed portable reports.
CI uploads its own coverage and storage-lab artifacts, including on gate failure.
[CI for the tested source revision](https://github.com/cosmic-utils/cosmic-ext-storage/actions/runs/35126072915)
and [current PR checks](https://github.com/cosmic-utils/cosmic-ext-storage/pull/117/checks)
are the remote evidence, not an inference from local success.

That source-revision CI run completed: all seven non-coverage checks passed.
The UI capability context reports deferred; its setup/launch/case/upload steps
were skipped. Coverage run `run-glwphbap` recorded host exit 0, lab exit 0,
UI exit null/deferred, acceptance exit 1. Its saved acceptance artifact contains
only the expected numerical, unmapped-source and support-measurement failures,
not an execution/provenance failure. CI workspace coverage is 11,368/28,416 lines
(40.01%) and 1,368/3,363 functions (40.68%); application counts match the local
table exactly. Native lab/sys/UDisks paths account for the small observed
local/CI differences; the reports are kept separate, not merged across runs.

## Fresh mapped Rust coverage

| Scope | Lines | Functions |
| --- | --- | --- |
| Workspace | 11,379 / 28,416 (40.04%) | 1,369 / 3,363 (40.71%) |
| Application | 4,055 / 15,834 (25.61%) | 420 / 1,538 (27.31%) |
| storage-btrfs | 133 / 319 (41.69%) | 24 / 56 (42.86%) |
| storage-contracts | 276 / 351 (78.63%) | 35 / 42 (83.33%) |
| storage-lab-tests | 871 / 1,085 (80.28%) | 80 / 110 (72.73%) |
| storage-sys | 940 / 1,510 (62.25%) | 116 / 194 (59.79%) |
| storage-types | 460 / 1,050 (43.81%) | 86 / 182 (47.25%) |
| storage-udisks | 3,230 / 5,231 (61.75%) | 369 / 767 (48.11%) |
| test-backend | 973 / 1,357 (71.70%) | 182 / 294 (61.90%) |
| ui-e2e-runner | 441 / 1,679 (26.27%) | 57 / 180 (31.67%) |

At checkpoint `95b4f66`, mapped workspace/application line coverage was
31.69%/12.45%. Application covered lines grew by 1,945. The denominator also
changed as duplicate executable harness/reducer code was deleted; this is not a
like-for-like fixed-source benchmark. Production views remain in scope.

There are 47 inventory files without LLVM mappings, reported explicitly. Many
are module/trait/type declarations; an audited mapping review remains necessary.
They have not been assigned invented executable-line counts or silently dropped.
The gate reports 6,189 failures including individual changed-code gaps; that is
not a count of failed tests. No package meets its unchanged 98% or 100% target.

## Remaining work (not waived by UI deferral)

- Coverage of production business logic is still partial: the root update module
  has 538 unhit mapped lines, volume partition handlers 355, encryption 348,
  mount-options handlers 287 and network handlers 261. Add real handler/native
  error, ordering and lifecycle tests at those layers, not another reducer.
  Network delete/rename transactionality and overlapping configuration reloads,
  legacy native image create/attach and native usage progress remain explicit gaps.
- Views are large visible gaps: app 1,293 unhit lines, logical 953, network 845,
  partition dialogs 584 and sidebar 577. Pausing rendered runs does not prove
  those paths or authorize removing them from the denominator.
- Complete Python/shell support coverage with provenance is not integrated.
  The acceptance report explicitly lists it as `unmeasured`. A diagnostic
  coverage.py run of the 42 unit contracts observed 458/848 Python executable
  lines (54.01%) and 213/400 branches (53.25%); it is not subprocess/container or
  function-coverage acceptance. A pinned kcov guard-only probe preserved exit
  codes 2/0/64 for disabled/enabled/invalid inputs without launching UI, but is
  not whole-support coverage and its placeholder branch rate is not evidence.
- Review the 47 unmapped files explicitly; keep changed-code and package gates
  failing until their real obligations are satisfied.
- Full rendered acceptance and further dependency repair remain paused. Follow
  the plan's explicit compatibility, quarantine and human-review checklist before
  re-enabling them. `UI_E2E_ENABLED=1` is permission to execute, not a gate waiver.

The earlier `run-zusipsok` at `e9b0e04` failed a Btrfs fixture assertion. It is
not the fresh baseline. The invalid prefix and duplicate image-start completion
fix are documented in the traceability ledger; both were retested before this run.

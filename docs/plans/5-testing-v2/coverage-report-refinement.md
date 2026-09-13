# First-party coverage export refinement

## Measured result (2026-09-13)

The LLVM JSON file inventory was already scoped by `--sources`, but its
function list still contained dependency definitions and instantiations.
Of 1,164,913 exported function records, only 23,366 belonged to our source
boundary. These are instantiations across build groups, not 23,366 distinct
source functions.

`run_coverage.py` now filters each JSON build group through the **same exact
source-path rule as the acceptance checker**, before retaining it in the
combined document or persisting the group report. Classification is cached
per filename. Uncovered definitions remain included. A function's defining
file comes from its first coverage region's file ID, not blindly from the
first filename or its mangled symbol name. Retained filename tables, region
indices, counts and per-file data are preserved.

| Reporting measurement | Before | After |
| --- | ---: | ---: |
| Combined JSON | 757,273,122 bytes | 28,870,576 bytes |
| Persisted group JSON total | 654,331,634 bytes | 28,871,246 bytes |
| Export wall time | 205.77 s | 68.44 s |
| Export plus coverage-analysis wall time | 363.58 s | 70.89 s |
| CPU time, Python plus subprocesses | 360.27 s | 70.80 s |
| Python peak RSS | 4,560,860 KiB | 1,328,000 KiB |

The final JSON is **96.2% smaller**; export plus analysis is **5.13 times
faster** and uses **80.3% less CPU time** in this controlled local sample.
Python peak RSS drops **70.9%**. LLVM still has to read the same instrumented
ELFs and produce a temporary raw group export; the RSS comparison is for Python,
not a measurement of simultaneous memory use across the entire process tree.
This is not a claim that instrumentation, compilation or tests became faster.

## Verification and reproduction

The before/after runs replayed the same **80 raw profiles in 11 build groups**
from `run-ss7vwdom`, with source, profile and ELF hashes checked before each
measurement. Runs were sequential on the same machine, using the same LLVM
tools. Both included actual LLVM profile merging, JSON/LCOV export, HTML
generation and a second JSON/LCOV read for coverage analysis. This is a single
local performance comparison, not a statistically controlled CI benchmark.

The benchmark snapshot, worker, reports and detailed metrics are retained at:

```text
target/coverage/export-benchmark-16EDXx/
  measure.py
  run_coverage.py                  # pre-refinement exporter snapshot
  coverage.py                     # unchanged checker snapshot
  evidence.json
  acceptance.json
  baseline/metrics.json
  scoped/metrics.json
```

The worker accepts `baseline` or `scoped` and refuses to overwrite its output
directory. Use a fresh copied benchmark directory to repeat a measurement.
Logs are `/tmp/coverage-export-baseline.log` and
`/tmp/coverage-export-scoped.log`.

Both reports have identical normalized **per-path, per-line and per-function
counts**, not merely identical percentages. Their coverage projection SHA-256
is `4463786506b3572a0eb0cf499dfdfaee26fea93f88f6b43a7506253991709672`.
Their byte-identical LCOV SHA-256 is
`8083140fea6180e7c9e7189cca2d5fa4a96c86802cf3d8ead33e84b2282cf688`.
The same 5,660 threshold/changed-code failures remain. No exceptions, source
exclusions or thresholds were relaxed.

Three new regression tests cover unchanged first-party counts (including
uncovered code and nonzero macro file IDs), scoped JSON persistence with
unchanged LCOV, and classification caching: 10,100 function records require
only two source-path classifications. All **23 Python tooling tests pass**.

This saved-profile performance replay is **not new test-execution or acceptance
evidence**. Changed collector inputs deliberately invalidate the old run's
acceptance provenance; they are not relabelled as current. A separate fresh
normal pipeline run verifies integration. Missing UI cases and the 98–100%
targets remain separate, unfinished gates.

## Fresh integrated run

`python3 tools/testing/run_coverage.py --base origin/main` completed as
`run-zuv3m23j`, with 213 host tests and all 16 selected native tests passing
(Nextest `1db64d91-67a8-40a3-abce-498b4c9c1e54`, 204.645 s). The one excluded
bridge helper belongs to the host suite, not the selected native case matrix.
The reload UI case passed all nine semantic steps and its exact known shutdown
failure was quarantined. Both pre-close app/runner profiles were retained and
accepted by the collector; 80 raw profiles were collected across all sources.
The case artifacts are in
`ui-artifacts/executed/live_scenario_reload-12-1789335895213354873`.

The default `target/coverage/summary.json` is now **28,869,656 bytes**. Its
current input, ELF and report hashes validate, and every retained function
record is first-party. Host, lab and UI stage exit codes are all zero.
The command's final exit code remains **1** because seven required UI cases
and the existing threshold/changed-code gaps remain—not because of stale
inputs, lost profiles or report generation failure.

Fresh measured coverage is 11,556/29,351 lines (**39.37%**) and 1,337/3,472
functions (**38.51%**). Relative to the earlier execution, two lab lines were
not hit and three quarantine-reporting lines were hit; this is fresh execution
variation. The controlled same-profile comparison above remains exactly
identical at every line/function count.

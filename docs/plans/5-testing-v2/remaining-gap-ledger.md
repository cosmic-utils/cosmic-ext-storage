# Testing V2 remaining gap ledger

Baseline: `5ff5e049756c5389f74a2bb0622b9612c219730f`, fresh run
`run-eu4hrjun`, 2026-09-14. This ledger directs implementation; it is not
acceptance evidence for later source changes. Detailed missing line/function
identities are retained with the execution record's baseline artifacts.

| Scope | Missing lines | Missing functions | Primary next test layer |
| --- | ---: | ---: | --- |
| Application | 13,416 | 1,309 | Scenario reducers/models, semantic views and eight executed AT-SPI flows |
| storage-btrfs | 186 | 32 | Native owned Btrfs fixtures plus parsing/error unit tests |
| storage-contracts | 76 | 7 | Contract validation and error/availability value cases |
| storage-lab-tests | 216 | 30 | Safe fixture ownership, rejection, timeout and cleanup/failure paths |
| storage-sys | 570 | 78 | Pure parsing/configuration plus private SFTP/LVM/MD integration |
| storage-types | 620 | 106 | Named independent value/serialization/boundary cases |
| storage-udisks | 2,005 | 398 | Private transport native outcomes, value mapping and error tests |
| test-backend | 423 | 133 | Owned scenarios covering validation, world mutations and workflow errors |
| ui-e2e-runner | 289 | 42 | Parser/selector/report regressions and real case execution |

No scope currently meets its 98–100% acceptance requirement. Function counts
include instrumented closures/definitions; a row of rstest inputs is not an
automatic function-coverage gain. Confirm uncovered definitions against the
actual source before writing tests.

## Highest-volume application gaps

Prioritize coherent behavior, not raw line-count padding:

| File | Missing lines / functions | Work |
| --- | ---: | --- |
| `src/update/mod.rs` | 1222 / 83 | Startup/messages, errors and completion ordering through injected runtime |
| `src/views/app.rs` | 1052 / 54 | Selected/unselected states, tabs, wizard/action availability and AT-SPI flows |
| `src/views/logical.rs` | 953 / 81 | LVM/MD/Btrfs states, forms, preflight and confirmation |
| `src/views/network.rs` | 803 / 56 | Provider/schema forms and mounted/error states |
| `src/update/network.rs` | 655 / 56 | Validation, failure/recovery and stale network completions |
| `src/views/dialogs/partition.rs` | 579 / 48 | Create/format/edit/resize steps and enabled/disabled/error variants |
| `src/update/volumes/encryption.rs` | 454 / 48 | Secret handling, unlock/lock errors and completion ordering |
| `src/state/logical.rs` | 368 / 49 | Candidate selection, form transitions and stale reviews |
| `src/views/sidebar.rs` | 356 / 33 | Empty/multiple groups, selections and available actions |
| `src/update/volumes/partition.rs` | 354 / 39 | Partition lifecycle, validation and failed/stale outcomes |
| `src/operations/filesystems.rs` | 352 / 51 | Injected adapter/tool discovery, mount/unmount and usage failures |

## Outcome and evidence ledger rules

For each mutating operation, record: success and actual state; validation or
authorization error; runtime failure and cleanup; cancellation/stale completion
where applicable; idempotent teardown. Link the exact selected test names and
execution artifacts. A scenario test cannot substitute for native state checks,
and a native test cannot substitute for an actual UI action.

First batch: `volume_models` replaces two silent D-Bus skips and adds five
assertion-bearing scenario model cases. Targeted execution passes; a fresh
combined coverage comparison is pending. All other gap rows remain open.

Non-Rust inventory currently comprises `tools/testing/{coverage,run_coverage}.py`,
`tools/ui-testing/{assert_tests,debug-app}.py`,
`tools/storage-lab/{entrypoint,run-tests,collect-evidence}.sh` and
`tools/ui-testing/{run-case,run-capability,container-runner,input-probe}.sh`.
The runtime child-shell strings have been extracted into named Bash scripts.
Also audit executable recipes in
Just/Containerfiles when defining the final support boundary; merely counting
the outer command would omit their scripts. Collector tests themselves are
test sources, not a way to dilute the executable-support denominator.

The shell interpreter/inline-code measurement decision is documented in
`remaining-execution-record.md`. Python unit measurements are partial; no
complete non-Rust line/function metric or threshold pass exists yet.

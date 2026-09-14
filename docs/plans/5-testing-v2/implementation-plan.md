# Testing V2 implementation plan

This plan implements [the Testing V2 specification](spec.md) in dependency
order. Phases 0–4 retain their ordered safety and migration gates. Phase 5's
coverage collection/checker tooling precedes Phase 6's executed UI cases;
Phase 5's final combined coverage gate necessarily follows those UI cases.
Neither phase is complete until that combined gate passes. A passing build
cannot substitute for named tests, cleanup evidence, coverage, or review.

## Execution rules

- Keep `test-backend` and its deterministic workflow tests. They are not
  replaced by the storage lab.
- Use Testcontainers as the one lifecycle mechanism for local and CI storage
  integration tests. Do not add a second custom case runner.
- Every test runs a native Rust assertion. A report, a case listing, or a
  placeholder `Passed` outcome is never test evidence.
- The only allowed destructive devices are the lab fixture's ledgered,
  file-backed loop devices. A safety failure is a hard test failure.
- Do not begin the implementation phases until Phase 0's prototype PR has
  supplied an actual GitHub-hosted-runner result.
- Keep each phase independently reviewable and use a separate commit for each
  phase. The prototype is a separate PR; all later work targets the #117
  branch only after that prototype is accepted.

## Phase 0 — Testcontainers capability prototype PR

**Outcome: passed on 2026-09-13.** The GitHub-hosted `Storage lab prototype`
job created `/dev/loop0` from a container-local sparse file, reached private
D-Bus/UDisks, detached it, and recorded an empty post-cleanup `losetup` list.
The final run took 4m51s. The prototype PR and branch are intentionally
disposable; its lessons are retained here and in the specification, while the
production implementation begins afresh in Phase 1.

### Recorded implementation constraints

1. Use `testcontainers` with its `blocking` feature when the fixture uses
   `SyncRunner`.
2. Start the image as privileged and network-disabled, but create only the
   required `/dev/loopN` device nodes inside the container. They are not
   reliably supplied by a privileged Docker container.
3. Ubuntu's packaged daemon was `/usr/libexec/udisks2/udisksd`; resolve the
   daemon path in the entrypoint instead of assuming `PATH`.
4. Linux loop detach is asynchronous. After `losetup --detach`, poll the
   backing-file mapping for a bounded interval. Do not assert that the sysfs
   loop node vanished and do not use `losetup --wait`, which is unavailable on
   the hosted runner's util-linux.
5. Capture container command output into a host-side target artefact directory;
   do not mount the workspace simply to obtain logs. Keep the image ID,
   container ID, loop ledger, probe exit code, and post-cleanup `losetup`
   result in machine-readable evidence.
6. Keep this low-level capability test in `storage-lab-tests` so that it does
   not compile the root GUI package. Maintain Cargo caching and a five-minute
   job timeout for a cold hosted runner.

### Branch and pull request

Starting from the current `4-ui-testing` tip, create and push:

```text
branch: codex/testing-v2-testcontainers-spike
base:   4-ui-testing
title:  spike(testing): prove Testcontainers storage lab on hosted CI
```

Open the PR against `4-ui-testing`, not `main`. This makes the existing CI
workflow run on the prototype without adding a speculative/dummy capability
commit to `main`.

### Allowed prototype paths

```text
Cargo.toml
Cargo.lock
.github/workflows/ci.yml
tests/storage_lab_capability.rs
tools/storage-lab/Containerfile
tools/storage-lab/entrypoint.sh
tools/storage-lab/run-tests.sh
tools/storage-lab/README.md
```

The prototype must not alter production storage code, `test-backend`,
`tools/storage-testing`, existing UI tests, or release/package configuration.

### Prototype implementation

1. Add `testcontainers` only as a test dependency and create a single named
   test: `github_hosted_runner_supports_private_storage_lab`.
2. Build the smallest pinned image that has `losetup`, D-Bus, UDisks2, and the
   lab entrypoint. The entrypoint starts a private system bus and UDisks,
   emits a readiness record, then waits for the probe command.
3. The Rust test starts that image through Testcontainers with external network
   disabled and the minimum capability set needed for loop-device setup. It
   runs a probe inside the container; the probe must:
   - create a sparse file under a unique `LAB_ROOT`;
   - attach and record a loop device;
   - confirm that the private D-Bus/UDisks object manager is reachable;
   - detach the loop device and remove the backing file; and
   - write JSON evidence and service logs to a host-side artifact directory.
4. Extend the existing CI workflow with a clearly named
   `storage-lab-prototype` job that runs only this test and uploads its
   artifacts on both success and failure. The job is informational while the
   prototype PR is under review.
5. Add a negative unit test proving the prototype safety helper rejects a
   physical-device name before a command can run.

### Prototype gates

```sh
cargo test --locked --test storage_lab_capability -- --list
cargo test --locked --test storage_lab_capability
cargo test --locked --test storage_lab_capability -- --ignored --nocapture
docker build -f tools/storage-lab/Containerfile -t cosmic-storage-lab:spike .
```

The pushed PR must show the `storage-lab-prototype` Actions result. Required
evidence is: image digest, container ID, loop-device ledger, `losetup -a`,
UDisks/D-Bus logs, cleanup result, and the test's JUnit/native output.

### Go/no-go rule

- **Pass:** the hosted job creates and removes a loop device, reaches private
  UDisks, and leaves no ledgered resource. Merge the prototype PR into
  `4-ui-testing`; proceed to Phase 1.
- **Fail:** do not begin the migration. Keep the PR as diagnostic evidence or
  close it after recording the failure in this plan/spec, then revise the
  architecture. Do not add VM infrastructure as an unreviewed workaround.

## Phase 1 — Freeze lab interface and image contract

### Files

```text
tools/storage-lab/Containerfile
tools/storage-lab/entrypoint.sh
tools/storage-lab/polkit/**
tools/storage-lab/fixtures/**
tools/storage-lab/README.md
crates/storage-lab-tests/Cargo.toml
crates/storage-lab-tests/src/lib.rs
crates/storage-lab-tests/tests/capability.rs
Cargo.toml
Cargo.lock
```

### Work

1. Add non-published workspace package `storage-lab-tests` and the pinned,
   multi-stage lab image described in the spec. Its builder compiles the inner
   Rust test binary with `cargo test --no-run`; its runtime contains that
   binary, its dynamic libraries, and only runtime lab dependencies.
2. Define only typed lab capabilities: `LabRoot`, `LabBackingFile`,
   `LabLoopDevice`, `LabMount`, `LabMapper`, `LabVolumeGroup`, and `LabArray`.
   No mutation helper accepts a raw device path or command text.
3. Implement a persistent ledger plus idempotent teardown. It records all
   resource creation before a later operation can use the resource, including
   container-created loop nodes. After detach, teardown polls the loop backing
   mapping until it disappears rather than checking for a vanished sysfs node.
4. Start private D-Bus, UDisks, Polkit policy, udev support, and the local
   SFTP service inside the container. Resolve the packaged `udisksd` path and
   add explicit health checks for each service needed by a selected test.
5. Make the lab image/network/root/mount restrictions testable. The image has
   no host D-Bus or block-device mount and execution has no external network.
6. Add `run-tests.sh`, which passes an exact Rust test-harness filter from
   Testcontainers to the baked inner binary. Do not introduce a case registry,
   host `target/` mount, or host-built binary copy. Split Docker layers so
   dependency compilation is cached separately from workspace sources.

### Required tests

- `capability_starts_private_dbus_udisks_and_sftp`
- `ledger_rejects_non_owned_and_physical_device_patterns`
- `teardown_is_idempotent_after_partial_setup`
- `failing_case_still_removes_all_ledgered_resources`

### Gates

```sh
cargo test -p storage-lab-tests --locked --test capability
STORAGE_LAB=1 cargo test -p storage-lab-tests --locked --test capability
```

### Implementation findings (2026-09-13)

- The pinned runtime must start `systemd-udevd` before `udisksd`. A private
  D-Bus ping alone is insufficient: without udev, an attached lab loop is not
  emitted as a UDisks object and the production adapter correctly cannot
  discover it.
- The host-side test must be `#[ignore]` by default and fail if invoked without
  `STORAGE_LAB=1`; a successful no-op outer test would recreate the old
  harness's false-green failure mode. Nextest enables only those ignored bridge
  tests for the lab profile.
- Building the real adapter in the inner image needs builder-only
  `libclang-dev` and `libbtrfsutil-dev` for the native Btrfs binding. They are
  not runtime dependencies and remain out of the runtime image.
- The earlier local loop-allocation failure is resolved. The same bridge now
  executes locally as well as on GitHub-hosted runners; failures preserve
  artifacts and are never treated as skips.
- The original device-mapper diagnosis was premature. The entrypoint created
  `/dev/dm0` while UDisks accessed `/dev/dm-0`, and the observed failure was
  a missing path. The authorised VM spike in PR #119 exposed this naming bug.
  Correct the node names before drawing conclusions about kernel support, and
  compare the corrected tests in a guest and directly on a hosted runner.
  LUKS format automatically unlocks the new volume; close it before testing a
  wrong secret. LVM and MDRAID still require their own executed capability tests;
  a successful LUKS case cannot stand in for either. Preserve service logs and
  artifacts before asserting success, including when VM startup fails.
  PR #119 subsequently passed all five storage tests (four existing plus LUKS,
  zero skips) both directly on GitHub-hosted Docker and inside QEMU/KVM:
  [source commit 6a84f95, run 34767576483](https://github.com/cosmic-utils/cosmic-ext-storage/actions/runs/34767576483).
  Also resolve device-mapper's major number from `/proc/devices`: the guest uses
  `252` for device mapper and `253` for its virtual disks, so a hardcoded major
  can alias the wrong device. The VM works but is unnecessary for these tested
  cases. Carry the container fixes and meaningful LUKS lifecycle assertions
  into implementation, and execute LVM/MDRAID capability gates separately.
  A warm local Docker build also exposed a stale-binary bug: selecting the first
  `capability-*` file can bake an old cached executable, and Rust exits zero
  when an exact test filter matches nothing. Select the executable from Cargo's
  JSON build messages and reject missing test names before execution. Validate
  both cold CI and warm local builds, and inspect inner execution counts.
- Fixture ledgers live outside the removable backing-file root and are copied
  into the outer artifacts. Multiple-loop unwind cleanup is now executed, not
  just inferred from `Drop`. Query the complete post-cleanup backing mapping:
  querying an individual device node after removing that node legitimately
  fails even when detach succeeded. Failed cleanup retains backing files and
  ledger entries for retry; it must never hide a busy loop by unlinking its file.

## Phase 2 — Production transport seam

### Files

```text
crates/storage-udisks/src/**
crates/storage-udisks/tests/lab_transport.rs
crates/storage-contracts/**             # only if a typed transport value is needed
crates/storage-lab-tests/tests/transport.rs
```

### Work

1. Refactor `UdisksBackend` construction to accept an explicit, typed D-Bus
   connection/address supplied by the composition root.
2. Preserve the normal production constructor and its system-bus semantics.
   Do not use a process-global environment override.
3. Construct the exact production adapter registry against the private lab bus
   from the baked container-internal test binary. No direct D-Bus probe can
   substitute for the adapter-level test, and no host D-Bus bridge is allowed.
4. Prove authentication/authorization failures travel through the production
   storage error mapping.

### Required tests

- `production_registry_discovers_only_lab_owned_devices`
- `explicit_lab_transport_does_not_change_production_constructor`
- `authorization_failure_maps_to_typed_storage_error`

### Gates

```sh
cargo test -p storage-udisks --locked --test lab_transport
STORAGE_LAB=1 cargo test -p storage-lab-tests --locked --test transport
```

## Phase 3 — Real storage-family cases

### Transport implementation evidence

The adapter's selected connection is now passed through partition, filesystem,
encryption, image, and SMART/power helper calls. A private peer that always
denies discovery verifies 15 adapter operations cannot silently use the system
bus. Additional tests verify permission-error mapping, simultaneous independent
adapters, clone identity, and unchanged normal-constructor behaviour. These are
transport contract tests, not substitutes for the real storage-family matrix.
The six existing container cases also pass after this refactor. The application
composition root can build its unchanged production registry from an explicitly
constructed `UdisksBackend` without installing a global runtime.

Port cases as native Rust tests; do not retain the old harness catalog.

| Commit | Tests to add | Required assertions |
| --- | --- | --- |
| `test(lab): cover disks and partitions` | discovery; create/delete; name/type/flags; invalid range; event refresh | API result, UDisks state, event order, cleanup |
| `test(lab): cover filesystems and encryption` | format; mount/options/unmount; busy retry; usage scan; LUKS options/unlock/lock/invalid secret | state, typed error/redaction, cleanup |
| `test(lab): cover image and btrfs operations` | attach/copy/cancel; Btrfs member/subvolume/default-subvolume cases | state, cancellation/error cleanup |
| `test(lab): cover logical storage` | LVM lifecycle; MDRAID lifecycle; discovery/stale reference/event refresh | topology, typed rejection, cleanup |
| `test(lab): cover local network storage` | container-local SFTP create/test/mount/status/unmount/failure | production rclone path, no external connection, cleanup |

Every row has one success path, one relevant failure path, and one teardown
assertion. Add application-level `AppRuntime` tests for flows whose adapter
result is transformed by application operations; adapter-only tests do not
cover the application mapping.

### Gates for every family

```sh
STORAGE_LAB=1 cargo test -p storage-lab-tests --locked --test <family>
cargo test --workspace --all-features --locked
```

## Phase 4 — Replace custom harness execution

### Files

```text
Cargo.toml
Cargo.lock
.config/nextest.toml
crates/storage-lab-tests/tests/bridge.rs
tests/storage_lab.rs                    # selected AppRuntime cases only
justfile
.github/workflows/ci.yml
tools/storage-testing/**                # delete only at phase completion
README.md
docs/plans/{2-lvm-refactor,3-logical-ui,4-testing,5-testing-v2}/**
```

### Work

1. Add the low-level outer Testcontainers lifecycle bridge in
   `crates/storage-lab-tests/tests/bridge.rs`. It starts the baked runtime
   image, executes the selected compiled inner test binary, maps non-zero exit
   status to a Rust test failure, and collects artifacts. Add
   `tests/storage_lab.rs` only for application-composition cases that require
   the root package.
2. Add Nextest with a single-slot `storage-lab` group, hard timeout, JUnit
   output, and no automatic retries. The low-level bridge has a five-minute
   cold-run timeout. A test target that is ignored locally is enabled in the
   CI job with `STORAGE_LAB=1`.
3. Add `just test-lab`. It builds the image, uses the bridge, and passes a
   native Rust test filter through to the inner binary; it never invokes an
   old harness binary or parses a catalog report.
4. Promote the CI `storage-lab` job from informational to required only after
   all Phase-3 family cases are present and passing.
5. In the same commit, delete `tools/storage-testing`, remove it from the
   workspace, remove `harness-nondestructive`, `harness`, `harness-logical`,
   and `lab` recipes, remove its CI step, and update every documentation
   reference. Do not leave compatibility shims or a stale catalog.

### Required tests

- `inner_test_failure_fails_outer_test_and_preserves_artifacts`
- `storage_lab_group_is_serial_and_has_no_retries`
- `ci_local_command_uses_identical_image_digest_and_test_selection`

### Gates

```sh
STORAGE_LAB=1 just test-lab
cargo nextest run --workspace --all-features
git grep -nE 'storage-testing|harness-nondestructive|NondestructiveExecutor|FullLabExecutor' -- \
  ':!docs/plans/5-testing-v2/**'
```

The final command must have no output.

## Phase 5 — Coverage to near 100%

### Resume checkpoint and remaining execution order (2026-09-14)

This section and Phases 6–7 are the active remaining plan. Phases 0–4 and
historical specifications/records are not being reopened. Resume from
`4-ui-testing` at or after `5ff5e049756c5389f74a2bb0622b9612c219730f`, which
merged [rstest PR #120](https://github.com/cosmic-utils/cosmic-ext-storage/pull/120).
[Final-head CI](https://github.com/cosmic-utils/cosmic-ext-storage/actions/runs/34880749004)
passed all seven jobs. This is migration evidence, not Testing V2 acceptance.

Already implemented; verify and extend rather than recreate:

- The isolated Testcontainers lab, production transport seams, native family
  cases, Nextest execution and legacy-harness retirement.
- Dev-only pinned rstest composition, owned scenario/workflow/lab fixtures,
  exact-selector rejection and fixture failure/cleanup regressions. See the
  [adoption record](rstest-execution-record.md) and
  [discovered assertion map](rstest-test-mapping.md).
- Scoped LLVM collection/export/checking, the empty exception manifest,
  source/input/ELF provenance, and durable instrumented UI checkpoints.
- The real `live_scenario_reload` UI case (nine steps), separate functional
  and shutdown evidence, and narrowly scoped teardown quarantine.

The last migration measurement is 39.38% Rust lines and 38.49% Rust functions,
with no covered retained-source behavior lost. Its host suite passed 282
tests, the lab passed 17 outer cases, Python passed 26 tests, and the required
inventory validated 80 entries. These are dated comparison points, not frozen
test-count gates or proof of non-Rust coverage. Obtain fresh evidence after
new test, fixture, source, dependency, image or collector changes.

Execute in this order, recording progress in a new
`docs/plans/5-testing-v2/remaining-execution-record.md` during implementation:

1. Create a `codex/` implementation branch from current `4-ui-testing`, record
   its starting SHA, discover current tests, and run the baseline gates below.
   Preserve unrelated work. Read the linked rstest records before composing
   new tests; do not rerun the completed adoption project.
2. Audit the current report and build a per-package/file gap ledger: uncovered
   lines/functions, missing operation outcome assertions, owning test layer,
   planned tests and evidence. Inventory non-Rust support separately and run
   the collector feasibility proof in work item 5 before large coverage-gap
   batches, so an unsupported metric is found early. Reproduce existing
   acceptance failures; do not mistake them for new regressions.
3. Close deterministic and native gaps in reviewable batches under Phase 5,
   and implement the seven remaining executed UI cases under Phase 6. Run
   targeted tests after each batch, retaining all existing regression bodies.
4. Complete non-Rust measurement and combined host/lab/UI collection; repeat
   the gap-test-report loop until every required threshold and source passes.
5. Validate the coverage CI job and all ordinary jobs, complete the Phase 7
   failure probes and review evidence, then obtain the separate approvals
   required for visual acceptance, repository policy and any final merge.

Do not stop at a higher aggregate percentage, a green host suite, completed
rstest adoption or a capability screenshot. A genuine blocker must identify
the failed gate, attempted safe remedies and exact authority/input needed;
it does not permit a skip, expanded quarantine or threshold waiver.

### rstest contract for all remaining Rust tests

- Reuse `tests/common/mod.rs` for root scenario/workflow fixtures,
  `crates/test-backend/tests/common/{mod.rs,paths.rs}` for backend fixtures,
  and `crates/storage-lab-tests/tests/common/mod.rs` for the owned native lab.
  Add `rstest.workspace = true` only to a consuming package's dev dependencies;
  retain `=0.27.0`, disabled default features and existing dependency pins.
- Use named `#[case::...]` rows for independent inputs/outcomes. Keep whole
  stateful, cancellation and resource-lifecycle sequences together where that
  preserves their assertions. Do not create a generic case dispatcher,
  forwarding wrapper family or fixture-injection framework.
- Await owned async fixtures with the existing Tokio/current-thread pattern.
  Inject renamed workflow arguments immutably, then shadow with `let mut`
  inside the body; type case-mutation closure parameters explicitly. Fixture
  dependencies are factories, not automatically cached instances: pass the
  same owned runtime explicitly when a test requires one shared world.
- Use per-test owned temporary directories (short `/tmp/cs-` roots for Unix
  sockets). Release servers/runtimes before bounded socket-removal checks and
  directory teardown. Keep resource ownership and destructive safety logic in
  the measured implementation; rstest only composes tests. No shared mutable
  `#[once]` fixtures, PID-named roots or process-global environment mutation.
- Extend the native bridge's named target/exact-filter table for genuinely
  new cases. Keep Testcontainers as resource owner, Nextest as scheduler,
  serial native execution, zero retries and existing hard deadlines. Avoid
  Cartesian case expansion and splitting one lifecycle into separate labs.
- Discover actual identities with Cargo and Nextest after adding/reordering
  cases; rstest indexes can change and may be zero-padded. Update affected
  required-test entries and their active validation references atomically.
  Preserve exact names in JUnit, artifact and coverage evidence; select a new
  generated case individually and prove a stale/parent filter cannot pass.
- Keep fixture isolation, setup failure, panic cleanup and stale native
  selector regressions passing. Generated fixture/test bodies stay test-only;
  do not relocate operational code under excluded test paths or count extra
  parameter rows as a production coverage gain.
- Keep Python orchestration and TOML/AT-SPI case execution. rstest is not a
  replacement for either. Community-toolkit extraction remains future work.

### Dependency clarification

Prepare the collector, checker, and real-adapter/host gap tests here, then
implement Phase 6's interactive UI execution before closing this phase's
combined coverage gate. Requiring executed UI profiles before implementing
their tests would be a circular dependency. This changes sequencing only:
missing UI profiles remain a hard failure and no threshold is relaxed.

### Files

```text
justfile
.github/workflows/ci.yml
tools/testing/coverage.py
docs/plans/5-testing-v2/coverage-exceptions.toml
.config/nextest.toml
tests/**
src/**
crates/**
```

### Work

1. Retain the existing empty `coverage-exceptions.toml` and audited schema.
   Any proposed exception must meet specification section 10.2 and be
   explicitly reviewed; no broad or convenience exception is allowed.
2. Extend the existing `tools/testing/coverage.py` and `run_coverage.py` as
   needed, with regression tests. The checker parses LLVM JSON/LCOV, limits analysis
   to first-party executable code, compares changed lines/functions with the
   PR base, validates the exception manifest, and enforces every section-10
   threshold.
   Scope JSON function records per build group before combining/persisting
   them: LLVM's `--sources` can leave dependency functions in its export.
   Reuse the gate's exact source boundary, retain uncovered definitions and
   macro file-ID mappings, and verify identical line/function counts and LCOV.
   See the [measured refinement](coverage-report-refinement.md).
3. Preserve and extend `just coverage`; it builds the same lab runtime image
   with the inner binary LLVM-instrumented, instruments host tests, and has the outer bridge
   archive the container-only `.profraw` directory through Testcontainers even
   after an inner failure. It extracts and merges those profiles, writes
   JSON/LCOV/HTML, then runs the checker. It must not use a bind mount or
   Docker CLI copy for profile collection.
   Instrument the UI app and runner from the same pinned image and collect
   their exact ELFs together with each executed case's profiles. After the
   final semantic assertion and pre-close captures, use the authenticated
   test-backend control channel to request `__llvm_profile_write_file()`;
   normal builds must reject this command. Keep normal exit flushing enabled
   (no counter reset or dump-complete flag). This retains executed counters
   across the quarantined teardown crash without pretending shutdown code
   executed successfully. Require an acknowledged checkpoint, nonempty app
   and runner profiles, matching ELF/case hashes, and every declared step.
   Use a project-specific `storage_ui_coverage` cfg: globally passing generic
   `--cfg coverage` activates nightly-only code in the pinned tiny-xlib.
   Hash build inputs, tests, fixtures and collector tools as well as production
   source; any change invalidates report-only acceptance. Never reuse an older
   run after adding tests or merely count case inventory as execution.
4. Add or refactor tests until every package and changed line/function meets
   the 100%/98% thresholds. Make UI/update/view code testable through reducer,
   semantic-view, and executed AT-SPI tests rather than excluding it.
   For every mutating operation, track success, validation/authorization error,
   runtime failure and cleanup, cancellation/stale completion where applicable,
   and idempotent teardown. Scenario assertions do not replace real-adapter
   state checks. Compare covered retained-source sets, not just percentages;
   explain removed definitions and source shifts without hiding lost paths.
5. Close the explicit non-Rust support-coverage gap. Inventory executable
   Python/shell support under `tools/testing`, `tools/storage-lab` and
   `tools/ui-testing`, including subprocess/container paths. Prove suitable
   established language collectors against success and deliberate-failure
   probes before integrating them into the existing coverage command; do not
   build another test runner. Record collector versions, source boundaries,
   executable line/function definitions and hit semantics. Require matching
   source/run provenance, retain uncovered definitions, and test missing,
   stale, partial and malformed reports. Enforce the specification's 100%
   support-code and changed-code requirements separately from LLVM Rust
   metrics; Rust percentages cannot stand in for scripts. If a collector
   cannot measure a required metric, report that gate blocked rather than
   inventing a percentage or treating it as zero executable code.
6. Add the `coverage` CI job using the existing command and actual PR base.
   Fetch enough history to resolve the base; retain JSON/LCOV/HTML, acceptance,
   provenance and relevant failure artifacts. Test report/profile merging
   while informational, then remove any advisory/failure-tolerant behavior
   before acceptance. Missing UI profiles or failed thresholds must fail the
   job. Configuring it as a required repository check is the separate Phase 7
   administration step, not a YAML setting or implicit authority to edit policy.

### Required checker tests

Retain or extend existing tests for these behaviors; these labels are
requirements, not presumed exact libtest names:

- `changed_uncovered_line_fails`
- `expired_or_broad_exception_fails`
- `lab_profile_is_required_for_final_report`
- `third_party_and_test_source_are_excluded_by_exact_path_rule`
- `threshold_regression_fails`
- Exact rstest-generated identities survive report/selection handling; stale,
  parent-only, duplicate and missing selections fail before false-green execution.
- Non-Rust source/metric omissions and stale or mismatched profiles fail.
- All eight UI cases require real completed steps and matching app/runner
  profiles; a passing capability probe or shutdown quarantine is insufficient.

### Gates

```sh
cargo fmt --all --check
cargo clippy --workspace --all-features --all-targets --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo nextest run --workspace --all-features --locked
python3 -m unittest discover -s tools/testing -p 'test_*.py'
python3 tools/ui-testing/assert_tests.py --phase all
STORAGE_LAB=1 just test-lab
just coverage origin/4-ui-testing
python3 tools/testing/coverage.py --base origin/4-ui-testing --summary target/coverage/summary.json
```

The commands above assume the implementation PR targets `4-ui-testing`.
Fetch and record its exact base SHA; use that SHA consistently if the base
moves during a run. For the later `4-ui-testing` to `main` PR, run fresh
acceptance against its actual `origin/main` base too. Never compare a branch
to itself to erase changed-code obligations. Initial diagnostic coverage may
fail on recorded gaps; final acceptance must exit zero, including the added
non-Rust checks. Preserve default/scenario-disabled and no-default-feature
build contracts from the rstest gate, not just all-feature success.

## Phase 6 — Honest UI E2E execution and documentation

### Work

1. Preserve the existing capability probe and executed reload job step.
   Keep capability, functional execution, shutdown, instrumentation and visual
   approval distinct in commands, reports and CI labels. `just ui-e2e` alone
   is capability evidence, not an executed-case acceptance gate.
2. Preserve the truthful `required-tests.toml` inventory. Update case status
   only with executable steps and observed evidence; keep the validator,
   coverage-required inventory and active validation references in agreement.
3. Convert the seven remaining schema-v1 inventories to executable schema-v2
   cases through the existing Rust TOML/AT-SPI runner. Do not replace them
   with rstest rows or treat them as unit tests. Implement and validate one
   case at a time, with explicit preconditions, bounded actions, observable
   postconditions, per-case artifacts and isolated scenario state:

   | Case ID | Minimum behavior to observe through the UI |
   | --- | --- |
   | `physical_partition_format` | Create/format flow, visible resulting state and validation/failure behavior. |
   | `busy_unmount` | Busy error, usable retry path and successful final unmount state. |
   | `luks_unlock` | Wrong-secret rejection, successful unlock and resulting state; no secret-bearing artifacts. |
   | `logical_preflight_confirmation` | Preflight/review, cancellation without mutation and confirmation with expected state. |
   | `network_mount` | Input validation, mount failure and successful mounted state. |
   | `image_usage_progress` | Usage/image workflow and observable progress/terminal state, including failure or cancellation. |
   | `keyboard_accessibility` | Keyboard-only navigation/activation, exact focus assertions and cancellation. |

   Preserve each case's existing scenario and acceptance obligations; the
   table is a minimum, not permission to remove assertions. Reuse owned
   rstest fixtures for companion Rust reducer/view/runner tests and scenario
   data for UI processes. Keep `live_scenario_reload`'s nine steps intact.
   For dependency blockers, inspect existing upstream fixes first. If no
   suitable release/fix exists, backport the minimal regression-tested repair
   onto the project's exact dependency pin, preserving submodule revisions.
   Do not substitute unvalidated upstream master. The user has prohibited a
   libcosmic PR; publish only the authorized fork branch and exact app pin.
   See [the button-routing evidence](ui-action-routing-blocker.md).
   The user-approved shutdown adjustment separates functional acceptance from
   clean exit. Save functional evidence before close; attempt bounded normal
   shutdown; quarantine only the exact, expiring pinned Wayland teardown stack.
   Early crashes, unknown failures, missing diagnostics and timeouts still
   fail. Report the known defect explicitly, without treating it as a clean
   run or as LLVM profile evidence. Do not expand into iced lifecycle repairs
   for this pass. See `tools/ui-testing/shutdown-quarantine.toml`.
   That policy currently binds only `live_scenario_reload`, its exact hashes
   and expiry (2026-10-13). It does not automatically cover the seven new
   cases. New or expired mismatches fail closed; obtain explicit review of
   fresh diagnostics for any proposed scope change. Do not silently refresh
   hashes, extend expiry or repair iced lifecycle code as an incidental fix.
4. Add the resulting UI paths to the coverage report and close their remaining
   application/view/update coverage gaps.
   Run all eight cases normally and instrumented through the same image/case
   mechanism, with fresh matching profiles and ELFs. Ensure CI executes all
   eight, not just reload; one failed case must fail the job and retain its
   evidence. Keep failure handling bounded with no automatic retries.
5. Complete the existing visual-approval contract: produce the required
   screenshots/baselines and reviewer evidence without self-approving generated
   output. Record semantic success separately while visual approval is pending.
   Request review as each case's evidence becomes available while continuing
   independent implementation work; do not defer all review until the end.
   Do not turn the currently disabled golden-update command into automatic
   acceptance or waive pixel review by renaming a status.
6. Update active README/runner documentation and remaining-plan status to
   describe the three truthful layers:
   deterministic scenario tests, executable Testcontainers real-adapter tests,
   and executed UI E2E tests.

### Gates

```sh
just ui-e2e
bash tools/ui-testing/run-case.sh tests/ui/cases/live_scenario_reload.toml
just coverage origin/4-ui-testing
python3 tools/ui-testing/assert_tests.py --phase all
```

After `just ui-e2e` builds the normal image, execute `run-case.sh` once for
each of the seven other exact paths in the table as well. Validate that the
required set equals the completed set and inspect every functional/shutdown
result. The coverage command discovers schema-v2 cases; its acceptance checker
must still reject an absent or unconverted mandatory case. Semantic success,
instrumented success and reviewed visual acceptance are separate exit gates.

## Phase 7 — Final acceptance

Before declaring Testing V2 complete:

1. Run all required CI jobs on a PR targeting `4-ui-testing` and retain their
   artifact links. Check the exact final head SHA, not a superseded green run.
   Keep existing job coverage and add the complete UI/coverage gates; do not
   weaken or bypass checks. Re-run after any code/test/input change.
2. Verify the section-10 coverage thresholds and empty/valid exception
   manifest in the uploaded report.
3. Review every destructive test helper against the device safety contract.
4. Force one inner-lab failure and one cleanup failure in a non-merge PR to
   verify failure propagation and artifact retention. The passing rstest
   parent regressions are not a substitute for a deliberately red hosted job.
   Use a separate temporary branch and two identifiable runs so one failure
   cannot mask the other. Inject faults only after lab ownership is established;
   ensure actual safe cleanup still occurs, no host device is targeted, the
   intended CI job fails and diagnostic artifacts survive. Never merge the
   fault patches. Restore/close the probe and remove its branch only under
   the publication/cleanup authority granted for that execution.
5. Verify no `tools/storage-testing` path or legacy harness reference remains
   outside historical-plan context.
6. Record commands, image digest, test names, artifact links, coverage
   results, and reviewer sign-off in a new phase record under
   `docs/plans/5-testing-v2/`. Account for every new/renamed generated identity
   and intended native launch; retain the rstest assertion map as the dated
   migration record rather than rewriting it. Verify no superseded composition
   helper, stale required name, broad dependency upgrade, source exclusion or
   generated artifact was introduced. Record Rust and non-Rust acceptance,
   all eight UI outcomes and visual approvals separately.
7. In a separate repository-administration change after the jobs are green,
   update the GitHub ruleset/branch protection to require `storage-lab` and
   `coverage`. Do not combine this external policy change with a functional
   migration commit. Resolve the actual reported check contexts (currently
   the native job is displayed as `Storage lab`) rather than guessing from
   YAML job IDs, preserve unrelated rules and verify enforcement afterward.
   Obtain explicit administration authority before changing repository policy.

This plan update authorizes no publication, repository-policy change, visual
sign-off or merge by itself. At execution, use the user's explicit authority
for those actions; otherwise finish safe local work, prepare exact evidence
and request the missing approval. Do not label an approval-pending gate done.
Only after every gate passes and merge is authorized may the completed work
be merged from `4-ui-testing` into `main`; revalidate its actual PR base and
final-head checks. Green rstest PR #120 alone never authorizes that merge.

### Remaining completion checklist

- [ ] Fresh starting-revision inventory, regression baseline and gap ledger.
- [ ] Every required operation outcome covered at its correct test layer.
- [ ] Seven remaining UI cases executable; all eight pass normal and
      instrumented execution, with separate reviewed visual acceptance.
- [ ] Rust package/aggregate/changed-code thresholds pass with valid provenance
      and no covered retained behavior lost.
- [ ] Non-Rust support coverage is measured and its required metrics pass.
- [ ] Exact generated selections, fixture cleanup, native safety and legacy
      retirement audited; no replacement home-grown composition layer.
- [ ] Both deliberately red hosted failure probes retain evidence and are
      kept out of the implementation branch.
- [ ] Final-head CI, including fail-closed coverage and all executed UI cases,
      is green and linked in the new execution record.
- [ ] Human visual/reviewer sign-off and separately authorized required-check
      enforcement recorded; no outstanding acceptance gate hidden by status.
- [ ] Authorized final integration uses the actual PR base and tested head.

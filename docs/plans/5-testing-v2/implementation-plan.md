# Testing V2 implementation plan

This plan implements [the Testing V2 specification](spec.md) in strictly
ordered phases. A phase may not be folded into the next one merely because the
workspace compiles: its named tests, cleanup evidence, coverage gate, and
review conditions must pass first.

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
- Local nested Docker still cannot allocate a loop device in this workspace.
  The same Testcontainers bridge passed both private adapter discovery and GPT
  partition-table creation on a GitHub-hosted runner; local failure preserves
  its captured artifacts and is never treated as a skip.

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

1. Add an empty `coverage-exceptions.toml` using the exact audited schema in
   the specification.
2. Add `tools/testing/coverage.py`. It parses LLVM JSON/LCOV, limits analysis
   to first-party executable code, compares changed lines/functions with the
   PR base, validates the exception manifest, and enforces every section-10
   threshold.
3. Add `just coverage`; it builds the same lab runtime image with the inner
   binary LLVM-instrumented, instruments host tests, and has the outer bridge
   archive the container-only `.profraw` directory through Testcontainers even
   after an inner failure. It extracts and merges those profiles, writes
   JSON/LCOV/HTML, then runs the checker. It must not use a bind mount or
   Docker CLI copy for profile collection.
4. Add or refactor tests until every package and changed line/function meets
   the 100%/98% thresholds. Make UI/update/view code testable through reducer,
   semantic-view, and executed AT-SPI tests rather than excluding it.
5. Add an informational coverage CI job first only long enough to verify
   report/profile merging. In the same PR once it passes, make it required;
   do not merge a threshold waiver.

### Required checker tests

- `changed_uncovered_line_fails`
- `expired_or_broad_exception_fails`
- `lab_profile_is_required_for_final_report`
- `third_party_and_test_source_are_excluded_by_exact_path_rule`
- `threshold_regression_fails`

### Gates

```sh
just coverage
python3 tools/testing/coverage.py --base origin/4-ui-testing --summary target/coverage/summary.json
```

## Phase 6 — Honest UI E2E execution and documentation

### Work

1. Rename the present capability-only CI job and documentation to
   `ui-e2e-capability`.
2. Change `required-tests.toml` so it does not present the eight named UI
   cases as executed while they are only inventory entries.
3. Implement each declared UI case with real AT-SPI actions, semantic
   assertions, and artifacts. Reuse scenario fixtures where applicable.
4. Add the resulting UI paths to the coverage report and close their remaining
   application/view/update coverage gaps.
5. Update README and plan documents to describe the three truthful layers:
   deterministic scenario tests, executable Testcontainers real-adapter tests,
   and executed UI E2E tests.

### Gates

```sh
just ui-e2e
just coverage
python3 tools/ui-testing/assert_tests.py --phase all
```

## Phase 7 — Final acceptance

Before declaring Testing V2 complete:

1. Run all required CI jobs on a PR targeting `4-ui-testing` and retain their
   artifact links.
2. Verify the section-10 coverage thresholds and empty/valid exception
   manifest in the uploaded report.
3. Review every destructive test helper against the device safety contract.
4. Force one inner-lab failure and one cleanup failure in a non-merge PR to
   verify failure propagation and artifact retention.
5. Verify no `tools/storage-testing` path or legacy harness reference remains
   outside historical-plan context.
6. Record commands, image digest, test names, artifact links, coverage
   results, and reviewer sign-off in a new phase record under
   `docs/plans/5-testing-v2/`.
7. In a separate repository-administration change after the jobs are green,
   update the GitHub ruleset/branch protection to require `storage-lab` and
   `coverage`. Do not combine this external policy change with a functional
   migration commit.

Only then may the completed work be merged from `4-ui-testing` into `main`.

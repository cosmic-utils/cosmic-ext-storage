# Storage integration lab

This image is a private, network-disabled Testcontainers fixture. It starts a
private system D-Bus, UDisks, Polkit, and a loopback-only SFTP service. Tests
are compiled by the pinned builder stage and run inside the runtime image; the
host workspace, Docker socket, host D-Bus, and host block devices are never
mounted.

The only mutable block targets are loop devices created from sparse files below
`/tmp/storage-lab`. The fixture records them before use and verifies their
backing mappings disappear during cleanup.

`storage-lab-run-tests` accepts one exact Rust test name through
`STORAGE_LAB_TEST_FILTER`. The outer Testcontainers bridge is responsible for
starting the container, passing that filter, and collecting its artifacts.

Run the same suite locally and in CI with `STORAGE_LAB=1 just test-lab`.
The entrypoint, exact-test launcher and named `collect-evidence.sh` helper use
the pinned image's Bash in normal and instrumented execution. The Rust
Testcontainers bridge still owns lifecycle and collects the profile archive
over container exec before teardown, including after failed inner tests; no
profile bind mount or Docker CLI copy was added. The recipe also checks archive
success, non-instrumented rejection and missing-binary failure in a separate
unprivileged, network-disabled disposable container. These fixture-byte checks
do not claim to be real LLVM profiles; the combined coverage run validates those.

The bridge covers private-service readiness; partition lifecycle and events;
filesystem formatting, labels, mount options, busy retry, read-only protection
and usage scanning; image backup/restore/attach/cancellation; LUKS lifecycle and
unwind cleanup; Btrfs subvolumes; LVM and MDRAID lifecycle; local SFTP/rclone;
and selected application-registry mappings. The opt-in runner
executes the normally ignored tests and retains Nextest JUnit output plus
device, service, and cleanup diagnostics under `target/`.

Device-mapper nodes use `/dev/dm-N` and the major number registered by the
running kernel in `/proc/devices`. Never hardcode that major: it can identify
another block driver. UDisks automatically unlocks a newly formatted encrypted
volume, so the LUKS test locks it before checking wrong credentials.

PR #119 proved the original five tests on a GitHub-hosted Docker runner and
inside QEMU/KVM. The expanded fifteen-case suite passed locally on 2026-09-13;
hosted CI must independently validate the expanded matrix. These results do
not establish the near-100% coverage acceptance gate or interactive UI E2E.

Partition and MD nodes can be missing even when UDisks sees their objects. A
fixture worker materializes only verified descendants, taking major/minor from
sysfs. MD aliases require the exact reserved member set; transient partial
assembly does not grant an alias. Worker failures are reported but do not
prevent cleanup of other independently verified resources. Ledger files remain
under `/tmp/storage-lab-evidence` after backing files have been removed.

UDisks replies can precede updated discovery properties. Tests wait for bounded
observable postconditions, never retry a destructive operation to hide a race.
Each bridge invocation writes its exact target/filter, exit code, stdout/stderr,
service logs, ledger, and post-test loop list before checking the outcome.

The Btrfs member case also needs the running kernel's libkmod metadata. The
bridge makes bounded copies of its real module indexes and Btrfs module (when
not built in), records their SHA-256 hashes, and verifies the container kernel
release. No host module directory is mounted. This requires a Linux Docker
daemon using the same kernel as the bridge; a mismatched remote daemon fails.

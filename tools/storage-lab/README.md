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
The five required bridge tests cover private-service readiness, GPT creation,
filesystem formatting and labels, fixture cleanup, and the LUKS lifecycle
(format, lock, reject a wrong secret, unlock, lock, cleanup). The opt-in runner
executes the normally ignored tests and retains Nextest JUnit output plus
device, service, and cleanup diagnostics under `target/`.

Device-mapper nodes use `/dev/dm-N` and the major number registered by the
running kernel in `/proc/devices`. Never hardcode that major: it can identify
another block driver. UDisks automatically unlocks a newly formatted encrypted
volume, so the LUKS test locks it before checking wrong credentials.

PR #119 proved these five tests on a GitHub-hosted Docker runner and inside
QEMU/KVM. A VM is not required for this suite. LVM and MDRAID still need their
own capability tests before making equivalent claims about them.

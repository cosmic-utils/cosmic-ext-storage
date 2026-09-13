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

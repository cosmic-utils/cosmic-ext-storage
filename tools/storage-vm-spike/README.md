# VM storage-lab capability spike

This is a temporary, deliberately isolated experiment. It asks one narrow
question that the normal private Testcontainers lab cannot answer on a
GitHub-hosted Docker runner: can the same Testcontainers image perform a LUKS
round trip when Docker runs inside a disposable guest kernel?

The workflow builds `cosmic-storage-lab:local` and the `vm_bridge` test on the
runner, then boots an Ubuntu cloud image with QEMU. The guest mounts source
read-only, loads that exact image into its own Docker daemon, and invokes the
Testcontainers bridge. The bridge in turn starts the same privileged,
network-isolated storage-lab container and runs only the LUKS test.

The guest writes serial output, kernel/device-mapper preflight output, and
inner storage-lab evidence to a separate artifact mount. The runner records
whether `/dev/kvm` is usable and lets QEMU fall back to TCG emulation, because
nested virtualization on GitHub-hosted runners is experimental rather than a
supported CI capability.

This is not a second permanent test mechanism. It must either demonstrate a
stable, acceptable execution path that can be folded into the testing plan, or
be deleted with its prototype workflow.

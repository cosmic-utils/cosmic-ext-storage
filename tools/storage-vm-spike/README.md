# VM storage-lab capability spike

This is a temporary, deliberately isolated experiment. It asks one narrow
question: can the same Testcontainers image perform a LUKS
round trip when Docker runs inside a disposable guest kernel?

The workflow builds `cosmic-storage-lab:local` and the `vm_bridge` test on the
runner, then boots an Ubuntu cloud image with QEMU. The guest mounts source
read-only, loads that exact image into its own Docker daemon, and invokes the
Testcontainers bridge. The bridge in turn starts the same privileged,
network-isolated storage-lab container and runs the existing four lab tests
plus the LUKS lifecycle test, sequentially. The VM target imports the regular
bridge and its evidence capture rather than duplicating its implementation.

Before booting the guest, the workflow runs that identical suite directly on
the runner as a diagnostic control. Its outcome is recorded separately; a
failure there does not prevent the guest experiment. This checks whether the
corrected `/dev/dm-0` naming alone is enough to enable LUKS without a VM.

The guest writes serial output, kernel/device-mapper preflight output, and
inner storage-lab evidence to a separate artifact mount. The runner records
whether `/dev/kvm` is usable and lets QEMU fall back to TCG emulation, because
nested virtualization on GitHub-hosted runners is experimental rather than a
supported CI capability.

This is not a second permanent test mechanism. It must either demonstrate a
stable, acceptable execution path that can be folded into the testing plan, or
be deleted with its prototype workflow.

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

## Executed result — 13 September 2026

[Run 34767576483](https://github.com/cosmic-utils/cosmic-ext-storage/actions/runs/34767576483)
tested source commit `6a84f95` on `ubuntu-24.04`:

| Environment | Result | Test execution |
| --- | --- | --- |
| Runner Docker directly | 5 passed, 0 failed, 0 ignored | 17.69 seconds |
| Docker inside QEMU/KVM guest | 5 passed, 0 failed, 0 ignored | 19.62 seconds |

The guest ran Linux `6.8.0-139-generic`, with two vCPUs and 4 GiB RAM.
Downloading, booting, provisioning, loading the image, testing, and shutting
down the VM took 2 minutes 27 seconds. The complete job, including host builds,
the direct comparison, artifact upload, and cache saving, took 7 minutes
19 seconds. KVM was used; the TCG fallback was not exercised.

The earlier LUKS failures were setup bugs, not evidence of missing runner
kernel support:

- The entrypoint created `/dev/dm0` rather than `/dev/dm-0`.
- Hardcoding block major `253` was incorrect in the guest: device mapper used
  `252`, while `253` was `virtblk`. Resolve the driver from `/proc/devices`.
- UDisks auto-unlocks after format. Lock first before checking a bad password,
  then verify successful unlock, lock, and loop cleanup.
- Cloud-init must mount the source share before executing its script. Copy
  diagnostics on failure as well as success, before checking the result marker.

The existing four tests and LUKS now use the same bridge and passed in both
environments. This proves VM feasibility for this suite, but supplies no reason
to require a VM for these cases. Prefer the simpler direct Testcontainers route
for them. LVM, MDRAID, other planned operations, repeatability, and coverage still
need their own evidence; they are not established by this spike.

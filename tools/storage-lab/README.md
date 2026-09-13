# Storage lab prototype

This is a Phase 0 Testcontainers capability probe, not the final storage test
harness. It runs UDisks2 and a private system D-Bus inside a Docker container
with no external network.

The probe creates a sparse backing file below `/tmp/storage-lab`, attaches it
to a loop device, reaches UDisks over the private bus, and then detaches and
removes every fixture it created. Its only Docker capability escalation is
`--privileged`, which is needed for loop setup; it neither mounts host block
devices nor talks to the host D-Bus.

Run the exact CI probe locally after building its image:

```sh
docker build --tag cosmic-storage-lab:spike --file tools/storage-lab/Containerfile .
cargo test --locked --test storage_lab_capability -- --ignored --nocapture
```

Artifacts are written under `target/storage-lab-prototype-artifacts/`. A
failure on a nested or restricted local Docker daemon is useful diagnostic
evidence; the go/no-go result is the GitHub-hosted runner job in the prototype
pull request.

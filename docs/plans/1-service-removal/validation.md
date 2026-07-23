# Service Removal Validation

**Branch:** `feature/remove-storage-service`
**Recorded:** 2026-07-22

## Automated verification

The following use the documented local workaround for the unavailable
`RUSTC_WRAPPER`:

| Check | Result |
| --- | --- |
| `env -u RUSTC_WRAPPER cargo fmt --all -- --check` | Pass |
| `env -u RUSTC_WRAPPER cargo test --workspace --all-features --locked` | Pass — 71 tests |
| `env -u RUSTC_WRAPPER cargo clippy --workspace --all-features --locked` | Pass |
| `env -u RUSTC_WRAPPER cargo build --workspace --release --locked` | Pass |
| `cargo metadata --no-deps --format-version=1` | Pass — root app plus five `crates/*` libraries |

Focused compile checks performed while implementing the migration:

- `cargo check -p storage-contracts`
- `cargo check -p storage-udisks -p storage-btrfs -p cosmic-ext-storage-storage-sys`
- `cargo check --workspace`

All focused checks completed successfully with `RUSTC_WRAPPER` unset.

## Clean desktop session matrix

Not run in this environment: it has no COSMIC Wayland desktop session or
disposable UDisks2 VM fixtures. These cases require execution before a
release is accepted:

- cold start, disk refresh, add/remove events, and no project daemon;
- filesystem and partition mutation, busy unmount, LUKS, Btrfs, and image
  workflows as the desktop user;
- on-demand multi-mount usage scanning and guarded file deletion;
- per-user rclone configuration, mount state, and mount-on-login;
- frosted and opaque COSMIC appearance smoke tests.

The absence of this desktop matrix is recorded as a release-validation gate,
not treated as a passing result.

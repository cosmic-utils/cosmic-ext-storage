# Service Removal Baseline

**Captured:** 2026-07-22
**Starting commit:** `ba61146b294360595ee24839994e2903fac23eb8` (`main`, `origin/main`)
**Implementation branch:** `feature/remove-storage-service`

## Workspace state

- Rust toolchain: `1.95.0-x86_64-unknown-linux-gnu`, selected by `rust-toolchain.toml`.
- The starting worktree was clean apart from the untracked plan directory
  `docs/plans/1-service-removal/`, which contains the canonical plan and its
  derived specification.
- `cargo metadata --no-deps --format-version=1` succeeded and reported these
  eight workspace packages:
  `storage-udisks`, `storage-types`, `cosmic-ext-storage` (at
  `storage-app/`), `storage-btrfs`, `storage-service`, `storage-macros`,
  `cosmic-ext-storage-storage-sys`, and `storage-contracts`.

## Baseline checks

All commands were run with `RUSTC_WRAPPER` unset, as required by the plan.

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --all -- --check` | Pass | Exit 0 |
| `cargo test --workspace --all-features --locked` | Pass | Exit 0; all unit and doc tests passed |
| `cargo clippy --workspace --all-features --locked` | Pass | Exit 0; no diagnostics |

No source diagnostic or host-library failure was present at baseline.

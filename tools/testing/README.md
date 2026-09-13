# Testing V2 coverage tools

Status: collection and acceptance tooling is under validation. This is **not**
a completed near-100% coverage migration. Eight interactive UI cases and their
profiles are still required; a capability probe does not satisfy that gate.

## Run

Use the repository's pinned Rust toolchain with `llvm-tools-preview`,
`cargo-llvm-cov` 0.9.1, Nextest, Just, Docker, and the normal native build
dependencies. Storage tests require a local Linux Docker daemon using the same
kernel as the host; see [the lab contract](../storage-lab/README.md).

```sh
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --version 0.9.1 --locked
python3 -m unittest discover -s tools/testing -p 'test_*.py'
just coverage origin/main
```

Use the actual PR base, not the implementation branch itself. `just coverage`
executes real host tests and the same `just test-lab` entry point as storage
CI. It instruments both the outer bridge and the baked inner tests. Inner
profiles and matching executables are archived through Testcontainers,
including after a failing inner test; there is no profile bind mount or
Docker CLI copy step.

Outputs are under `target/coverage`: `summary.json`, `lcov.info`,
`html/index.html`, `evidence.json`, `acceptance.json`, and a fresh `run-*`
directory containing the raw profiles and execution/build evidence. A failed
test or missing source of coverage never becomes a successful acceptance run.
The checker still reports diagnostic counts when acceptance fails.

ELFs from different build roots/features are exported in matching groups.
Combining every ELF in a single LLVM export can select one build's line
mapping and lose coverage from another. The final reports union executable
source lines and function definitions across groups; the HTML uses that same
union, not a single-build approximation. Third-party sources and test targets
are excluded by explicit path boundaries; production source files are not
blanket-excluded.

To regenerate reports without repeating a successful host/lab run:

```sh
python3 tools/testing/run_coverage.py --report-only --base origin/main
```

This checks source inventory, source hashes, executable hashes, and raw profile
hashes first. It refuses changed sources or a failed underlying test run.
Regeneration is not another test execution and cannot supply missing UI cases.

Do not lower thresholds, create broad exceptions, or interpret an absent
profile/package as zero executable code. The exception manifest remains
empty; final coverage acceptance is still gated by the Testing V2 spec,
including test-support coverage outside Rust.

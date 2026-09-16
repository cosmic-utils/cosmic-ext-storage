# Testing V2 coverage tools

Status: collection and acceptance tooling is under validation. This is **not**
a completed near-100% coverage migration. Default `non-rendered` coverage requires
real host and native-lab evidence, with rendered UI explicitly deferred.
Opt-in `full-ui` coverage still requires all eight UI cases and their profiles;
a capability probe does not satisfy that gate. Both modes retain the same
production source inventory and numerical thresholds.

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

Outputs are under `target/coverage/<mode>`: `summary.json`, `lcov.info`,
`html/index.html`, `evidence.json`, `acceptance.json`, and a fresh `run-*`
directory containing the raw profiles and execution/build evidence. A failed
test or missing source of coverage never becomes a successful acceptance run.
The checker still reports diagnostic counts when acceptance fails.
Each run captures a hashed execution policy before instrumentation; report
reuse validates that policy and the expected mode. UI-off is not a successful
UI run, and a non-rendered report cannot satisfy full-UI acceptance. Sources
without LLVM mappings are listed as unmeasured and fail pending mapping review;
this includes distinguishing declaration-only files from missing executable
code without inventing line counts.

ELFs from different build roots/features are exported in matching groups.
Combining every ELF in a single LLVM export can select one build's line
mapping and lose coverage from another. The final reports union executable
source lines and function definitions across groups; the HTML uses that same
union, not a single-build approximation. Third-party sources and test targets
are excluded by explicit path boundaries; production source files are not
blanket-excluded.

Unit-test bodies live in package `tests/unit` paths and retain their original
logical modules through `#[path]`. Fixture and outer-bridge implementations live
under `src` and remain measured; moving those helpers under `tests` would
incorrectly improve the denominator. Older reports from before this layout
change are invalid for current acceptance.

To regenerate reports without repeating a successful host/lab run:

```sh
python3 tools/testing/run_coverage.py --report-only --base origin/main
```

This checks source inventory, source hashes, executable hashes, and raw profile
hashes and the captured PR-base commit first. It refuses changed sources, a
changed comparison base or a failed underlying test run.
Regeneration is not another test execution and cannot supply missing UI cases.

For explicitly requested rendered coverage (currently blocked/incomplete):

```sh
UI_E2E_ENABLED=1 just coverage origin/main full-ui
```

The flag accepts only `0` or `1` and defaults to `0`. Full mode rejects a disabled
flag before building images or executing tests. The Rust runner, shell launchers
and local recipes independently guard their launch boundaries. `--all-features`
does not opt in. Default CI keeps the UI check context with a deferred summary;
manual workflow dispatch offers the explicit rendered diagnostic opt-in.

Do not lower thresholds, create broad exceptions, or interpret an absent
profile/package as zero executable code. The exception manifest remains
empty; final coverage acceptance is still gated by the Testing V2 spec,
including test-support coverage outside Rust.
The acceptance report explicitly inventories Python/shell support sources as
unmeasured and fails that gate until complete line/function collection and
subprocess/container provenance are integrated. Standalone collector probes
and Python unit-suite measurements do not satisfy that obligation.

name := 'cosmic-ext-storage'
appid := 'com.cosmic.ext.Storage'
rootdir := ''
prefix := '/usr'
base-dir := absolute_path(clean(rootdir / prefix))

bin-src := 'target' / 'release' / name
bin-dst := base-dir / 'bin' / name
desktop-src := 'resources' / appid + '.desktop'
desktop-dst := clean(rootdir / prefix) / 'share' / 'applications' / appid + '.desktop'
appdata-src := 'resources' / appid + '.metainfo.xml'
appdata-dst := clean(rootdir / prefix) / 'share' / 'appdata' / appid + '.metainfo.xml'
icon-src := 'resources/icons/hicolor/scalable/apps/' + appid + '.svg'
icon-dst := clean(rootdir / prefix) / 'share/icons/hicolor/scalable/apps/' + appid + '.svg'

# Build and launch the desktop app directly. UDisks2 authorizes privileged
# operations through its native Polkit integration; no project daemon exists.
default: run

build *args:
    cargo build --workspace --locked {{ args }}

release *args:
    cargo build --workspace --release --locked {{ args }}

check:
    @case "${UI_E2E_ENABLED-0}" in 0) echo 'Rendered UI: deferred (disabled)' ;; 1) echo 'Rendered UI: opted in; run ui-e2e separately' ;; *) echo 'UI_E2E_ENABLED must be exactly 0 or 1' >&2; exit 64 ;; esac
    cargo fmt --all -- --check
    cargo clippy --workspace --all-features --locked
    cargo test --workspace --all-features --locked

# Validate the versioned scenario contract without needing a desktop server.
ui-plan-check:
    python3 tools/ui-testing/assert_tests.py --plan-only

# Verify that every required, named target can be selected before executing a
# phase gate.  Use phase=all for the complete non-graphical inventory.
ui-assert-tests phase='all':
    @phase_value="{{ phase }}"; python3 tools/ui-testing/assert_tests.py --phase "${phase_value#phase=}"

# Run deterministic application workflow tests without a compositor, desktop
# session, or accessibility stack.
app-workflow-check:
    just ui-assert-tests phase='workflow-v2'
    cargo test -p cosmic-ext-storage --features test-backend --locked --test application_workflows

# Build and execute the same isolated Testcontainers storage lab used by CI.
# The private lab, rather than the host, owns every loop-backed mutation.
test-lab:
    @test "${STORAGE_LAB:-}" = "1" || { echo "STORAGE_LAB=1 is required to run the privileged disposable storage lab" >&2; exit 1; }
    docker build --build-arg "VERGEN_GIT_SHA=$(git rev-parse HEAD)" --build-arg "VERGEN_GIT_COMMIT_DATE=$(git show -s --format=%cI HEAD)" --build-arg "STORAGE_LAB_COVERAGE=${STORAGE_LAB_COVERAGE:-0}" --tag cosmic-storage-lab:local --file tools/storage-lab/Containerfile .
    @docker image inspect --format 'storage-lab image={{"{{"}}.Id{{"}}"}}' cosmic-storage-lab:local
    @echo 'storage-lab suite=bridge (including LUKS) artifacts=target/storage-lab-artifacts'
    cargo nextest run --locked --profile storage-lab -p storage-lab-tests --features outer-bridge --test bridge --run-ignored ignored-only
    STORAGE_SCRIPT_CONTAINER_TESTS=1 python3 -m unittest tools.testing.test_shell_contract.LabContainerShellContractTests

# Host+native coverage by default; unchanged thresholds may still fail.
coverage base='origin/main' mode='non-rendered':
    python3 tools/testing/run_coverage.py --base {{ quote(base) }} --mode {{ quote(mode) }}

ui-scenario-check:
    python3 tools/ui-testing/assert_tests.py --plan-only
    @find tests/ui/scenarios -name '*.toml' -print0 | sort -z | xargs -0 -n1 cargo run -p test-backend --locked --bin ui-scenario -- validate

ui-test scenario:
    bash tools/ui-testing/require-enabled.sh
    cargo run --features test-backend --locked -- --backend scenario --scenario {{ scenario }}

ui-e2e:
    bash tools/ui-testing/require-enabled.sh
    docker build --build-arg "VERGEN_GIT_SHA=$(git rev-parse HEAD)" --build-arg "VERGEN_GIT_COMMIT_DATE=$(git show -s --format=%cI HEAD)" --file tools/ui-testing/Containerfile --tag cosmic-storage-ui-e2e:local .
    bash tools/ui-testing/run-capability.sh
    STORAGE_SCRIPT_CONTAINER_TESTS=1 python3 -m unittest tools.testing.test_shell_contract.UiContainerShellContractTests

ui-e2e-update:
    @echo "PNG golden updates stay disabled until the Rust case runner is implemented after this capability gate." >&2
    @exit 1

# Execute semantic actions/assertions from a v2 case. Pixel acceptance remains
# a separate reviewed gate; this command cannot approve its own screenshots.
ui-e2e-case case:
    bash tools/ui-testing/require-enabled.sh
    docker build --build-arg "VERGEN_GIT_SHA=$(git rev-parse HEAD)" --build-arg "VERGEN_GIT_COMMIT_DATE=$(git show -s --format=%cI HEAD)" --file tools/ui-testing/Containerfile --tag cosmic-storage-ui-e2e:local .
    bash tools/ui-testing/run-case.sh {{ quote(case) }}

package-check:
    cargo build --release --locked
    @output=$(target/release/cosmic-ext-storage --help 2>&1 || true); if printf '%s\n' "$output" | rg -Fq 'scenario'; then echo 'release binary exposes scenario mode' >&2; exit 1; fi

run *args:
    env RUST_BACKTRACE=full cargo run --locked {{ args }}

install: release
    install -Dm0755 {{ bin-src }} {{ bin-dst }}
    install -Dm0644 {{ desktop-src }} {{ desktop-dst }}
    install -Dm0644 {{ appdata-src }} {{ appdata-dst }}
    install -Dm0644 {{ icon-src }} {{ icon-dst }}

uninstall:
    rm -f {{ bin-dst }} {{ desktop-dst }} {{ appdata-dst }} {{ icon-dst }}

clean:
    cargo clean

clean-vendor:
    rm -rf .cargo vendor vendor.tar

vendor:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p .cargo
    cargo vendor --sync Cargo.toml | head -n -1 > .cargo/config.toml
    echo 'directory = "vendor"' >> .cargo/config.toml
    tar pcf vendor.tar .cargo vendor
    rm -rf .cargo vendor

vendor-extract:
    rm -rf vendor
    tar pxf vendor.tar

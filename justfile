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

ui-scenario-check:
    python3 tools/ui-testing/assert_tests.py --plan-only
    @find tests/ui/scenarios -name '*.toml' -print0 | sort -z | xargs -0 -n1 cargo run -p test-backend --locked --bin ui-scenario -- validate

ui-test scenario:
    cargo run --features test-backend --locked -- --backend scenario --scenario {{ scenario }}

ui-e2e:
    docker build --build-arg "VERGEN_GIT_SHA=$(git rev-parse HEAD)" --build-arg "VERGEN_GIT_COMMIT_DATE=$(git show -s --format=%cI HEAD)" --file tools/ui-testing/Containerfile --tag cosmic-storage-ui-e2e:local .
    docker run --rm --network none -v "{{ invocation_directory() }}:/workspace" -w /workspace cosmic-storage-ui-e2e:local sh -ec 'ui_e2e_uid=$(stat -c %u /workspace); ui_e2e_gid=$(stat -c %g /workspace); printf "ui-e2e:x:%s:%s:UI E2E:/tmp/ui-e2e-home:/usr/sbin/nologin\n" "$ui_e2e_uid" "$ui_e2e_gid" >> /etc/passwd; mkdir -p ui-artifacts /tmp/ui-e2e-home /tmp/ui-e2e-config /tmp/ui-e2e-cache; chown "$ui_e2e_uid:$ui_e2e_gid" ui-artifacts /tmp/ui-e2e-home /tmp/ui-e2e-config /tmp/ui-e2e-cache; exec setpriv --reuid="$ui_e2e_uid" --regid="$ui_e2e_gid" --clear-groups env HOME=/tmp/ui-e2e-home XDG_CONFIG_HOME=/tmp/ui-e2e-config XDG_CACHE_HOME=/tmp/ui-e2e-cache dbus-run-session -- /opt/ui-test/bin/ui-e2e-runner capability --app /opt/ui-test/bin/cosmic-ext-storage --scenario /workspace/tests/ui/scenarios/empty.toml --sway-config /workspace/tools/ui-testing/sway.conf --environment-lock /workspace/tools/ui-testing/environment.lock.toml --artifacts /workspace/ui-artifacts/capability'

ui-e2e-update:
    @echo "PNG golden updates stay disabled until the Rust case runner is implemented after this capability gate." >&2
    @exit 1

package-check:
    cargo build --release --locked
    @output=$(target/release/cosmic-ext-storage --help 2>&1 || true); if printf '%s\n' "$output" | rg -Fq 'scenario'; then echo 'release binary exposes scenario mode' >&2; exit 1; fi

# Run all safe, required-execution harness scenarios. The helper creates a
# marker-bearing artifact directory; the runner refuses an arbitrary directory.
harness-nondestructive:
    @artifact_dir=$(cargo run --quiet -p storage-testing --bin lab -- create-artifact --label harness-nondestructive); STORAGE_TESTING_ARTIFACT_DIR="$artifact_dir" cargo run --quiet -p storage-testing --bin harness -- --profile nondestructive --require-executed

# Full fixture mutation is intentionally available only to the disposable lab.
harness:
    @test "${STORAGE_TESTING_ENABLE_DESTRUCTIVE:-}" = "1" || { echo "STORAGE_TESTING_ENABLE_DESTRUCTIVE=1 is required in the disposable fixture VM" >&2; exit 1; }
    @artifact_dir=$(cargo run --quiet -p storage-testing --bin lab -- create-artifact --label harness); STORAGE_TESTING_ARTIFACT_DIR="$artifact_dir" cargo run --quiet -p storage-testing --bin harness -- --profile full-lab --require-executed

# Execute every logical-storage scenario in the disposable fixture VM.  The
# profile gate deliberately remains explicit so this recipe cannot touch host
# disks by accident.
harness-logical:
    @test "${STORAGE_TESTING_ENABLE_DESTRUCTIVE:-}" = "1" || { echo "STORAGE_TESTING_ENABLE_DESTRUCTIVE=1 is required in the disposable fixture VM" >&2; exit 1; }
    @artifact_dir=$(cargo run --quiet -p storage-testing --bin lab -- create-artifact --label harness-logical); STORAGE_TESTING_ARTIFACT_DIR="$artifact_dir" cargo run --quiet -p storage-testing --bin harness -- --profile full-lab --suite logical --require-executed

lab:
    @test "${STORAGE_TESTING_ENABLE_DESTRUCTIVE:-}" = "1" || { echo "STORAGE_TESTING_ENABLE_DESTRUCTIVE=1 is required in the disposable fixture VM" >&2; exit 1; }
    @artifact_dir=$(cargo run --quiet -p storage-testing --bin lab -- create-artifact --label lab); STORAGE_TESTING_ARTIFACT_DIR="$artifact_dir" cargo run --quiet -p storage-testing --bin harness -- --profile full-lab --require-executed

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

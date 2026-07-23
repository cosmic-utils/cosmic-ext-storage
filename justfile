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
    cargo build --workspace --locked {{args}}

release *args:
    cargo build --workspace --release --locked {{args}}

check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-features --locked
    cargo test --workspace --all-features --locked

run *args:
    env RUST_BACKTRACE=full cargo run --locked {{args}}

install: release
    install -Dm0755 {{bin-src}} {{bin-dst}}
    install -Dm0644 {{desktop-src}} {{desktop-dst}}
    install -Dm0644 {{appdata-src}} {{appdata-dst}}
    install -Dm0644 {{icon-src}} {{icon-dst}}

uninstall:
    rm -f {{bin-dst}} {{desktop-dst}} {{appdata-dst}} {{icon-dst}}

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

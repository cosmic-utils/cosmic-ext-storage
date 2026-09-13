#!/usr/bin/env bash
set -euo pipefail
test "$#" = 1 || { echo "usage: run-case.sh <repository-relative v2 case>" >&2; exit 64; }
repo_root=$(git rev-parse --show-toplevel)
case_relative=$1
case "$case_relative" in tests/ui/cases/*.toml) ;; *) echo "case must be under tests/ui/cases" >&2; exit 64 ;; esac
test -f "$repo_root/$case_relative"
mkdir -p "$repo_root/ui-artifacts"
docker run --rm --network none \
  --mount "type=bind,src=$repo_root,dst=/workspace,readonly" \
  --mount "type=bind,src=$repo_root/ui-artifacts,dst=/workspace/ui-artifacts" \
  -w /workspace cosmic-storage-ui-e2e:local sh -ec '
    ui_case_uid=$(stat -c %u /workspace/ui-artifacts)
    ui_case_gid=$(stat -c %g /workspace/ui-artifacts)
    printf "ui-e2e:x:%s:%s:UI E2E:/tmp/ui-e2e-home:/usr/sbin/nologin\n" "$ui_case_uid" "$ui_case_gid" >> /etc/passwd
    mkdir -p /tmp/ui-e2e-home /tmp/ui-e2e-config /tmp/ui-e2e-cache
    chown "$ui_case_uid:$ui_case_gid" /tmp/ui-e2e-home /tmp/ui-e2e-config /tmp/ui-e2e-cache
    exec setpriv --reuid="$ui_case_uid" --regid="$ui_case_gid" --clear-groups \
      env HOME=/tmp/ui-e2e-home XDG_CONFIG_HOME=/tmp/ui-e2e-config XDG_CACHE_HOME=/tmp/ui-e2e-cache \
      dbus-run-session -- /opt/ui-test/bin/ui-e2e-runner execute \
      --app /opt/ui-test/bin/cosmic-ext-storage --case "$1" --root /workspace \
      --sway-config /workspace/tools/ui-testing/sway.conf \
      --environment-lock /workspace/tools/ui-testing/environment.lock.toml \
      --artifacts /workspace/ui-artifacts/executed
  ' ui-case "$case_relative"

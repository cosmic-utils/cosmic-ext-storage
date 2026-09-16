#!/bin/bash
# Container-only bootstrap shared by capability and executed-case runs.
# Keep normal and instrumented execution on this same Bash code path.
set -eu

case "${1:-}" in
    capability)
        test "$#" = 1 || { echo 'capability takes no additional arguments' >&2; exit 64; }
        runner_args=(capability
            --scenario /workspace/tests/ui/scenarios/empty.toml
            --artifacts /workspace/ui-artifacts/capability)
        ;;
    execute)
        test "$#" = 3 || { echo 'execute requires a case path and coverage boolean' >&2; exit 64; }
        [[ "$2" =~ ^tests/ui/cases/[a-zA-Z0-9_-]+\.toml$ ]] || { echo 'invalid case path' >&2; exit 64; }
        case "$3" in true|false) ;; *) echo 'invalid coverage boolean' >&2; exit 64 ;; esac
        runner_args=(execute --case "$2" --root /workspace
            --artifacts /workspace/ui-artifacts/executed --coverage "$3")
        ;;
    *) echo 'expected capability or execute' >&2; exit 64 ;;
esac

bash "$(dirname "${BASH_SOURCE[0]}")/require-enabled.sh"
ui_runner_uid=$(stat -c %u /workspace/ui-artifacts)
ui_runner_gid=$(stat -c %g /workspace/ui-artifacts)
printf 'ui-e2e:x:%s:%s:UI E2E:/tmp/ui-e2e-home:/usr/sbin/nologin\n' \
    "$ui_runner_uid" "$ui_runner_gid" >> /etc/passwd
mkdir -p /tmp/ui-e2e-home /tmp/ui-e2e-config /tmp/ui-e2e-cache
chown "$ui_runner_uid:$ui_runner_gid" /tmp/ui-e2e-home /tmp/ui-e2e-config /tmp/ui-e2e-cache
exec setpriv --reuid="$ui_runner_uid" --regid="$ui_runner_gid" --clear-groups \
    env HOME=/tmp/ui-e2e-home XDG_CONFIG_HOME=/tmp/ui-e2e-config XDG_CACHE_HOME=/tmp/ui-e2e-cache \
    dbus-run-session -- /opt/ui-test/bin/ui-e2e-runner "${runner_args[@]}" \
    --app /opt/ui-test/bin/cosmic-ext-storage \
    --sway-config /workspace/tools/ui-testing/sway.conf \
    --environment-lock /workspace/tools/ui-testing/environment.lock.toml

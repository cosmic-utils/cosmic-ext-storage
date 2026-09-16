#!/usr/bin/env bash
# Execution policy only; never starts a container or compositor.
set -eu
test "$#" = 0 || { echo 'usage: require-enabled.sh' >&2; exit 64; }
case "${UI_E2E_ENABLED-0}" in
    1) ;;
    0) echo 'Rendered UI is deferred. Opt in explicitly with UI_E2E_ENABLED=1.' >&2; exit 2 ;;
    *) echo 'UI_E2E_ENABLED must be exactly 0 or 1.' >&2; exit 64 ;;
esac

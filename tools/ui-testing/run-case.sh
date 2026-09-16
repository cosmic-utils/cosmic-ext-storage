#!/usr/bin/env bash
set -euo pipefail
test "$#" = 1 || { echo "usage: run-case.sh <repository-relative v2 case>" >&2; exit 64; }
repo_root=$(git rev-parse --show-toplevel)
case_relative=$1
[[ "$case_relative" =~ ^tests/ui/cases/[a-zA-Z0-9_-]+\.toml$ ]] || { echo "case must be a TOML file directly under tests/ui/cases" >&2; exit 64; }
test -f "$repo_root/$case_relative"
case "${UI_COVERAGE:-0}" in 0) coverage=false ;; 1) coverage=true ;; *) exit 64 ;; esac
bash "$repo_root/tools/ui-testing/require-enabled.sh"
mkdir -p "$repo_root/ui-artifacts"
docker run --rm --network none --env UI_E2E_ENABLED=1 \
  --mount "type=bind,src=$repo_root,dst=/workspace,readonly" \
  --mount "type=bind,src=$repo_root/ui-artifacts,dst=/workspace/ui-artifacts" \
  -w /workspace cosmic-storage-ui-e2e:local \
  /bin/bash /workspace/tools/ui-testing/container-runner.sh execute "$case_relative" "$coverage"

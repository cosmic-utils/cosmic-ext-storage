#!/usr/bin/env bash
set -euo pipefail
test "$#" = 0 || { echo 'usage: run-capability.sh' >&2; exit 64; }
repo_root=$(git rev-parse --show-toplevel)
bash "$repo_root/tools/ui-testing/require-enabled.sh"
mkdir -p "$repo_root/ui-artifacts"
docker run --rm --network none --env UI_E2E_ENABLED=1 \
    --mount "type=bind,src=$repo_root,dst=/workspace,readonly" \
    --mount "type=bind,src=$repo_root/ui-artifacts,dst=/workspace/ui-artifacts" \
    -w /workspace cosmic-storage-ui-e2e:local \
    /bin/bash /workspace/tools/ui-testing/container-runner.sh capability

#!/usr/bin/env bash
set -euo pipefail
test "$#" = 0 || { echo 'usage: run-capability.sh' >&2; exit 64; }
repo_root=$(git rev-parse --show-toplevel)
mkdir -p "$repo_root/ui-artifacts"
docker run --rm --network none \
    --mount "type=bind,src=$repo_root,dst=/workspace,readonly" \
    --mount "type=bind,src=$repo_root/ui-artifacts,dst=/workspace/ui-artifacts" \
    -w /workspace cosmic-storage-ui-e2e:local \
    /bin/bash /workspace/tools/ui-testing/container-runner.sh capability

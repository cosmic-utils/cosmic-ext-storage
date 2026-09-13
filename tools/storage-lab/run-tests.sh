#!/bin/sh
set -eu

: "${STORAGE_LAB_TEST_FILTER:?a Rust test-harness filter is required}"
exec /opt/storage-lab/bin/storage-lab-capability \
    --ignored \
    --exact "$STORAGE_LAB_TEST_FILTER" \
    --nocapture

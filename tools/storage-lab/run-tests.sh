#!/bin/sh
set -eu

: "${STORAGE_LAB_TEST_FILTER:?a Rust test-harness filter is required}"
# Rust's test harness exits successfully when a filter selects zero tests.
# Refuse a stale image or misspelled name instead of reporting a false pass.
if ! /opt/storage-lab/bin/storage-lab-capability \
    --ignored --exact "$STORAGE_LAB_TEST_FILTER" --list \
    | grep -Fqx -- "$STORAGE_LAB_TEST_FILTER: test"; then
    printf 'Required storage-lab test not found: %s\n' "$STORAGE_LAB_TEST_FILTER" >&2
    exit 64
fi
exec /opt/storage-lab/bin/storage-lab-capability \
    --ignored \
    --exact "$STORAGE_LAB_TEST_FILTER" \
    --nocapture

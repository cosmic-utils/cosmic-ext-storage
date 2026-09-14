#!/bin/bash
set -eu

: "${STORAGE_LAB_TEST_FILTER:?a Rust test-harness filter is required}"
test_target=${STORAGE_LAB_TEST_TARGET:-capability}
case "$test_target" in
    ''|*[!a-zA-Z0-9_-]*) printf '%s\n' 'Invalid Rust test target' >&2; exit 64 ;;
esac
test_binary="/opt/storage-lab/bin/storage-lab-$test_target"
# Rust's test harness exits successfully when a filter selects zero tests.
# Refuse a stale image or misspelled name instead of reporting a false pass.
if ! LLVM_PROFILE_FILE=/dev/null "$test_binary" \
    --ignored --exact "$STORAGE_LAB_TEST_FILTER" --list \
    | grep -Fqx -- "$STORAGE_LAB_TEST_FILTER: test"; then
    printf 'Required storage-lab test not found: %s\n' "$STORAGE_LAB_TEST_FILTER" >&2
    exit 64
fi
exec "$test_binary" \
    --ignored \
    --exact "$STORAGE_LAB_TEST_FILTER" \
    --nocapture

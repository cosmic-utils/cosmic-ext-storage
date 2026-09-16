#!/bin/bash
# Test double for one external command, not an alternate test runner.
printf '%s\0' "$@"
exit "${SCRIPT_TEST_EXIT:-0}"

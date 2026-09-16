#!/bin/bash
# Container-only evidence transport. stdout is the artifact, never a status log.
set -eu

case "${1:-}" in
    devices)
        test "$#" = 1 || exit 64
        cat /proc/devices
        ls -l /dev/dm-* /dev/mapper/control
        dmsetup info -c
        lsblk -o NAME,MAJ:MIN,TYPE,MOUNTPOINTS
        ;;
    services)
        test "$#" = 1 || exit 64
        # Best-effort diagnostics; cleanup failures in ledgers are checked by Rust.
        cat /tmp/storage-lab/polkitd.log /tmp/storage-lab/udisksd.log \
            /tmp/storage-lab/sshd.log /tmp/storage-lab-evidence/*.ledger 2>/dev/null || true
        ;;
    profiles)
        test "$#" = 2 || exit 64
        case "$2" in ''|*[!a-zA-Z0-9_-]*) echo 'invalid test target' >&2; exit 64 ;; esac
        test "$(cat /opt/storage-lab/bin/coverage-mode)" = 1
        exec tar -czf - -C / tmp/storage-lab-profiles "opt/storage-lab/bin/storage-lab-$2"
        ;;
    *) echo 'expected devices, services or profiles <target>' >&2; exit 64 ;;
esac

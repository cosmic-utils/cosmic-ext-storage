#!/usr/bin/env bash
# Runs inside the QEMU guest, as root via cloud-init.
set -Eeuo pipefail

readonly workspace=/mnt/workspace
readonly artifacts=/mnt/artifacts
result=1

finish() {
    local command_status=$?
    if [[ $result -eq 0 && $command_status -ne 0 ]]; then
        result=$command_status
    fi
    mkdir -p "$artifacts" || true
    printf '%s\n' "$result" > "$artifacts/result-code.txt" || true
    touch "$artifacts/result-$result" || true
    sync || true
    /sbin/poweroff || true
    exit 0
}
trap finish EXIT

mkdir -p "$artifacts"
mount -t 9p -o trans=virtio,version=9p2000.L artifacts "$artifacts"
exec > >(tee -a "$artifacts/guest-run.log") 2>&1

echo '--- guest kernel ---'
uname -a
echo '--- guest device mapper preflight ---'
modprobe dm_crypt || true
dmsetup version || true
ls -l /dev/mapper /dev/dm-* 2>&1 || true

systemctl start docker
for _ in {1..30}; do
    docker info >/dev/null 2>&1 && break
    sleep 1
done
docker info

readonly image_archive="$workspace/target/vm-storage-lab-prototype/storage-lab-image.tar"
test -f "$image_archive"
docker load --input "$image_archive"

readonly bridge_binary="$(find "$workspace/target/debug/deps" -maxdepth 1 -type f -name 'vm_bridge-*' -perm -111 | head -n 1)"
test -n "$bridge_binary"

set +e
STORAGE_LAB=1 STORAGE_LAB_ARTIFACT_ROOT="$artifacts" \
    "$bridge_binary" --ignored --exact luks_runs_in_a_guest_vm_through_testcontainers --nocapture \
    2>&1 | tee "$artifacts/vm-bridge.log"
result=${PIPESTATUS[0]}
set -e

echo "vm bridge result=$result"
exit 0

#!/bin/sh
set -eu

mkdir -p /run/dbus /run/sshd /tmp/storage-lab /tmp/storage-lab-evidence
touch /run/storage-lab-private
# A privileged container still may not receive device-mapper nodes. UDisks
# creates mapper devices for LUKS/LVM, so provide only the conventional
# container-local control node and a small numbered range. The container is
# never given a host block-device mount and removal is its final backstop.
mkdir -p /dev/mapper
if [ ! -e /dev/mapper/control ]; then
    mknod /dev/mapper/control c 10 236
fi
# Block major numbers are assigned by the running kernel. In a VM, hardcoding
# 253 can alias a different driver (including the guest's boot disk).
dm_major=$(awk '$2 == "device-mapper" { print $1 }' /proc/devices)
if [ -n "$dm_major" ]; then
    for index in $(seq 0 15); do
        device="/dev/dm-$index"
        if [ ! -e "$device" ]; then
            mknod "$device" b "$dm_major" "$index"
        fi
    done
fi

dbus-daemon --system --fork --nopidfile

# UDisks observes block-device changes through udev. Start the daemon before
# UDisks so loop devices attached by an inner test become ObjectManager events.
/usr/lib/systemd/systemd-udevd &
udevd_pid=$!
udevadm trigger --action=add --subsystem-match=block || true
udevadm settle --timeout=10 || true

/usr/lib/polkit-1/polkitd --no-debug >/tmp/storage-lab/polkitd.log 2>&1 &
polkitd_pid=$!
/usr/libexec/udisks2/udisksd >/tmp/storage-lab/udisksd.log 2>&1 &
udisksd_pid=$!
ssh-keygen -A >/tmp/storage-lab/ssh-keygen.log 2>&1
/usr/sbin/sshd -D -p 2222 >/tmp/storage-lab/sshd.log 2>&1 &
sshd_pid=$!

for attempt in $(seq 1 100); do
    if kill -0 "$udisksd_pid" "$polkitd_pid" "$udevd_pid" "$sshd_pid" 2>/dev/null \
        && ss -ltn | grep -q ':2222 ' \
        && dbus-send --system --dest=org.freedesktop.PolicyKit1 --print-reply \
        /org/freedesktop/PolicyKit1/Authority org.freedesktop.DBus.Peer.Ping >/tmp/storage-lab/polkit-ready.log 2>&1 \
        && dbus-send --system --dest=org.freedesktop.UDisks2 --print-reply \
        /org/freedesktop/UDisks2 org.freedesktop.DBus.Peer.Ping \
        >/tmp/storage-lab/udisks-ready.log 2>&1; then
        printf '%s\n' 'STORAGE_LAB_READY'
        exec sleep infinity
    fi
    if ! kill -0 "$udisksd_pid" 2>/dev/null \
        || ! kill -0 "$polkitd_pid" 2>/dev/null \
        || ! kill -0 "$udevd_pid" 2>/dev/null \
        || ! kill -0 "$sshd_pid" 2>/dev/null; then
        cat /tmp/storage-lab/*.log >&2 || true
        exit 1
    fi
    sleep .1
done

cat /tmp/storage-lab/*.log >&2 || true
exit 1

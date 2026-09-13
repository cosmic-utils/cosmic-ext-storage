#!/bin/sh
set -eu

mkdir -p /run/dbus /run/sshd /tmp/storage-lab
dbus-daemon --system --fork --nopidfile

# UDisks observes block-device changes through udev. Start the daemon before
# UDisks so loop devices attached by an inner test become ObjectManager events.
/usr/lib/systemd/systemd-udevd &
udevd_pid=$!
udevadm trigger --action=add --subsystem-match=block || true
udevadm settle --timeout=10 || true

polkitd >/tmp/storage-lab/polkitd.log 2>&1 &
polkitd_pid=$!
/usr/lib/udisks2/udisksd >/tmp/storage-lab/udisksd.log 2>&1 &
udisksd_pid=$!
ssh-keygen -A >/tmp/storage-lab/ssh-keygen.log 2>&1
/usr/sbin/sshd -D -p 2222 >/tmp/storage-lab/sshd.log 2>&1 &
sshd_pid=$!

for attempt in $(seq 1 100); do
    if dbus-send --system --dest=org.freedesktop.UDisks2 --print-reply \
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

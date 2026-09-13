#!/bin/sh
set -eu

mkdir -p /run/dbus /run/udisks2 /tmp/storage-lab
dbus-daemon --system --fork --nopidfile

udisksd_path=/usr/libexec/udisks2/udisksd
"$udisksd_path" --no-debug >/tmp/storage-lab/udisksd.log 2>&1 &
udisksd_pid=$!

for attempt in $(seq 1 50); do
    if dbus-send --system --dest=org.freedesktop.UDisks2 --print-reply \
        /org/freedesktop/UDisks2 org.freedesktop.DBus.Peer.Ping \
        >/tmp/storage-lab/udisks-ready.log 2>&1; then
        printf '%s\n' 'STORAGE_LAB_READY'
        exec sleep infinity
    fi
    if ! kill -0 "$udisksd_pid" 2>/dev/null; then
        cat /tmp/storage-lab/udisksd.log >&2 || true
        exit 1
    fi
    sleep 0.1
done

cat /tmp/storage-lab/udisksd.log >&2 || true
exit 1

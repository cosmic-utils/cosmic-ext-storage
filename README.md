# COSMIC Storage

> [!WARNING]
> Storage operations can destroy data. Verify the selected device before formatting, restoring an image, or changing partitions.

COSMIC Storage is a desktop storage utility for the COSMIC desktop. It runs as the logged-in desktop user and uses the system `udisks2` daemon for device discovery, device events, and its native Polkit-authorized operations. This project does not install or run a project-owned service, socket, D-Bus policy, or Polkit policy.

## Runtime dependencies

- `udisks2` for local storage discovery and operations.
- Filesystem tools appropriate to the filesystems you use, such as `e2fsprogs`, `xfsprogs`, `btrfs-progs`, `dosfstools`, `ntfs-3g`, and `exfatprogs`.
- Optional: `rclone` for per-user network-drive configurations. Configurations live under the desktop user’s `~/.config/rclone/`; mounts and mount-on-login units are also user-scoped.

The application uses the backend-neutral `storage-contracts` API. The currently shipped block-storage adapter is `UdisksBackend`; additional local or network adapters can be registered at the application composition root without making UI code depend on their implementation.

## Development

```sh
just          # build and launch the app
just check    # fmt, clippy, and tests
just release  # release workspace build
```

`just install` installs the application binary, desktop entry, metainfo, and icon. It does not install service, policy, or socket files.

## Logging

Logs go to stdout/stderr and daily files in `$XDG_STATE_HOME/cosmic-ext-storage/logs/`, or `~/.local/state/cosmic-ext-storage/logs/` if `XDG_STATE_HOME` is unset. Use `RUST_LOG` to control verbosity.

## Translators and packagers

Fluent translations are in [i18n](i18n). The root [justfile](justfile) includes vendoring helpers for distribution builds.

![Screenshot of COSMIC Storage](resources/screenshots/cosmic-ext-storage.png)

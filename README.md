# COSMIC Storage

> [!WARNING]
> Storage operations can destroy data. Verify the selected device before formatting, restoring an image, or changing partitions.

COSMIC Storage is a desktop storage utility for the COSMIC desktop. It runs as the logged-in desktop user and uses the system `udisks2` daemon for device discovery, device events, and its native Polkit-authorized operations. This project does not install or run a project-owned service, socket, D-Bus policy, or Polkit policy.

### Prerequisites
You will need the following packages/services:
 - `udisks2` (system service) - required for device enumeration, events, and native Polkit-authorized operations
 - `just` (task runner) - install via `cargo install just` or your package manager
 
For partition type support:
Recommended:
 - `ntfs-3g` / `ntfsprogs` - NTFS Support
 - `exfatprogs` - exFAT Support
 - `dosfstools` - FAT32 Support
 - `rclone` - SMB, FTP, S3, etc. per-user mount support

- `udisks2` for local storage discovery and operations.
- Filesystem tools appropriate to the filesystems you use, such as `e2fsprogs`, `xfsprogs`, `btrfs-progs`, `dosfstools`, `ntfs-3g`, and `exfatprogs`.
- Optional: `rclone` for per-user network-drive configurations. Configurations live under the desktop user’s `~/.config/rclone/`; mounts and mount-on-login units are also user-scoped.

The application uses the backend-neutral `storage-contracts` API. The currently shipped block-storage adapter is `UdisksBackend`; additional local or network adapters can be registered at the application composition root without making UI code depend on their implementation.

## Development

**Quick Start:**
```bash
just
```
This single command builds and launches the UI directly. UDisks2 handles authorization through its native Polkit integration; no project storage service, D-Bus policy, or project Polkit policy is installed or run.

**Other useful commands:**
```bash
just build              # Build workspace only
just release            # Build workspace in release mode
just check              # Run fmt, clippy, and tests
just run                # Build and run the app
just install            # Install the app binary and desktop assets
just uninstall          # Remove installed app files
```

`just install` installs the application binary, desktop entry, metainfo, and icon. It does not install service, policy, or socket files.

## Logging

#### v0.1 - ⌛ WIP
- ✅ Feature Parity with Gnome Disks
   - **Deferred until v0.2**: Benchmark Disk/Partition
   - **Deferred until v0.2**: ATA Drive settings
- 🎯 Performance improvements
- 🎯 LVM/Logical container support
- ✅ Detailed Usage tool
- ⌛ BTRFS support - Partial implementation complete.
   - Subvolumes Management
   - Snapshot Management & Scheduling
   - Optional Usage breakdown (requires enablement of quotas)
- ✅ Rclone configuration
   - Setup wizard for common mount types
   - Mount on boot option
   - Supports all providers/types
   - Supports per-user mounts
- ✅ Automatic "Resource Busy" resolution on unmount
   - List processes that are holding the mount open, and give you the option to kill them.
- ⌛ Detection for required packages:
    - rclone detection missing currenty.
- 🎯 Full test of all drive, volume, and mount types. 
- 🎯 Documentation - Docs/Readme/Code comments & summaries
- 🎯 Packaging for package managers/flathub 

## Translators and packagers

Fluent translations are in [i18n](i18n). The root [justfile](justfile) includes vendoring helpers for distribution builds.


![Screenshot of Storage App](https://github.com/cosmic-utils/cosmic-ext-storage/blob/main/resources/screenshots/cosmic-ext-storage.png)


### Notes on use of AI
AI has been used as a ***tool*** for development of this project, and has not been treated as a self-sufficient engineer.

I have been a professional software engineer since 2012, and I am very much against AI slop and the existential threat it imposes on our industry.

That being said, I believe when it's used correctly, it is an invaulable tool for a sole developer on a FOSS project as large as this; Especially when the threat of taking somebody's job isn't a concern.

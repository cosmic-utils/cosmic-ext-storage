# Service Removal and Root Workspace Implementation Plan

**Status:** Implemented; clean-desktop validation remains pending
**Authority:** This is the canonical implementation plan. [spec.md](spec.md) is a derived implementation specification; if the documents differ, this plan governs.
**Execution order:** Complete the tasks in order. Delete the application client wrappers only in Task 4 after their operation replacements are wired. Do not delete the service, policy files, or service-only crates until Task 5's gate passes; perform those deletions only in Task 6.

## Implementation record — 2026-07-22

- Captured the baseline in [baseline.md](baseline.md) and implemented the work on
  `feature/remove-storage-service`.
- Replaced the project D-Bus boundary with typed contracts, adapters, and the
  application `operations` module. The app receives block-device events from
  `DeviceEventSource`; it does not subscribe to project-owned D-Bus signals.
- Removed `storage-service`, `storage-macros`, project policy/systemd files,
  and the caller-identity model. The root package is now the application and
  the five retained libraries are under `crates/`.
- Updated dependency pins, resource ownership, task recipes, README, VS Code
  launch configuration, and release/version workflows for the root workspace.
- Automated verification and the remaining clean-desktop VM matrix are
  recorded in [validation.md](validation.md). The desktop matrix has not been
  represented as passed from this non-graphical environment.

## Outcome

Replace the project-owned root D-Bus service with typed in-process application operations, then make the application the root package and move retained libraries to crates/.

The final workspace contains six packages only:

| Package | Final location | Role |
| --- | --- | --- |
| cosmic-ext-storage | . | Application executable |
| storage-btrfs | crates/storage-btrfs/ | Btrfs library and optional test CLI |
| cosmic-ext-storage-storage-sys | crates/storage-sys/ | System and rclone helpers |
| storage-contracts | crates/storage-contracts/ | Backend-neutral traits, errors, operation IDs, and events |
| storage-types | crates/storage-types/ | Typed models and catalog data |
| storage-udisks | crates/storage-udisks/ | UDisks2 integration |

Delete storage-service, storage-macros, the service-only caller model, and every project D-Bus/systemd/Polkit artifact. Retain and move storage-contracts because it becomes the required backend abstraction layer. UDisks2 remains the default required system daemon, but the application-facing storage API must not require it.

## Non-negotiable implementation decisions

- The application runs as its desktop user. UDisks2 native Polkit is the only supported authorization path. Do not add a helper daemon, socket, system unit, D-Bus policy, or project Polkit policy.
- A full filesystem scan is a user action only. Startup may enumerate disks and collect inexpensive usage data, but must not call storage_sys::usage::scan_paths_with_progress.
- Application operations return storage-types values and one app operation error type. They must not serialize JSON, generate zbus proxies, or recreate the removed service protocol.
- Retain the protected-path guard for force-unmount and validation around usage deletion. Remove checks that existed solely because a root daemon acted on behalf of another user.
- rclone is strictly per-user only. The final workspace has no ConfigScope, system rclone configuration, or system rclone unit support; the UI never offers a scope choice. Temporary legacy rclone code may remain only long enough for the unchanged storage-service to compile, and is deleted in Task 6.
- Btrfs, image, and ownership operations run as the desktop user. Surface a UDisks2 or filesystem permission failure; do not work around it with hidden elevation.
- The app consumes backend contracts, never a UDisks2-specific API. UDisks2 is one adapter registered at the composition root; adding a different local-storage or network-drive backend must not require changes to the app state, views, messages, or operation orchestration.
- The existing storage-contracts crate has no consumer today, which is precisely why this migration must make the contract boundary real and tested before removing the service.

## Starting-state facts and baseline

The current workspace has eight members: the app, five libraries (including the currently unused contracts crate), and the two crates to remove. cargo metadata --no-deps --format-version=1 succeeds.

The first baseline test command fails before compiling because the current RUSTC_WRAPPER points to a missing sccache executable. That is a local environment issue and must not cause a source or configuration change in this migration. Use env -u RUSTC_WRAPPER for local validation until the developer fixes their toolchain setup.

### Task 0: Capture a reproducible baseline

**Files**

- Create: docs/plans/1-service-removal/baseline.md

**Steps**

1. Run:

   ~~~sh
   git status --short
   cargo metadata --no-deps --format-version=1
   ~~~

   Record the commit, package list, Rust toolchain, and worktree state. The only expected untracked path before this work starts is docs/plans/1-service-removal/.

2. Run:

   ~~~sh
   env -u RUSTC_WRAPPER cargo fmt --all -- --check
   env -u RUSTC_WRAPPER cargo test --workspace --all-features --locked
   env -u RUSTC_WRAPPER cargo clippy --workspace --all-features --locked
   ~~~

   Record every exit status and pre-existing diagnostic. If a host library is absent, record it and install it only in the validation environment; do not repair unrelated Rust code in this task.

**Gate:** Visually review the baseline record for the starting commit, package list, toolchain, worktree, and pre-existing diagnostics before implementation changes begin.

## Target in-process API

Create this application-owned module tree in Task 3:

~~~text
src/operations/
├── mod.rs                 # StorageOperations construction
├── error.rs               # OperationError
├── disk.rs                # discovery, SMART, power, safe removal
├── partition.rs           # table and partition mutations
├── filesystem.rs          # filesystem, usage, mount options
├── protected_paths.rs     # force-unmount safety invariant
├── luks.rs                # LUKS operations/options
├── btrfs.rs               # typed disks_btrfs adapter
├── image.rs               # operation registry/progress
└── network.rs             # backend-neutral network-drive operations
~~~

StorageOperations is asynchronously constructed once during application startup. It owns a BackendRegistry, cached filesystem capability data, an ImageOperationManager, and local usage-scan state. The registry owns one selected block-storage adapter and zero or more network-drive adapters. Models, tasks, and subscriptions get Arc clones of this context. They must not build a per-operation D-Bus connection or call a concrete backend.

OperationError is the sole application-operation error type. Give it explicit InvalidInput, Unavailable, PermissionDenied, Unsupported, MissingOperation, and a conversion from the transport-neutral StorageError supplied by storage-contracts. It must not have service-not-available, proxy, JSON, D-Bus-method, or caller-identity variants.

## Implementation tasks

### Task 1: Update the dependency baseline

**Files**

- Modify: Cargo.toml
- Modify: Cargo.lock
- Modify: workspace manifests only where a resolved API change requires it

**Steps**

1. Change workspace libcosmic revision from 2f02228 to the reviewed immutable revision ef162b8e16ba4493e05c169cd56c7b9f77f0fda5. Do not use a branch or the old v0.12 tag.

   ~~~sh
   cargo update -p libcosmic
   ~~~

   Confirm Cargo.lock contains that exact git revision and commit both files together.

2. Run cargo update once, review its manifest and lockfile diff, and retain stable SemVer-compatible updates only. Commit the resulting lockfile so the selected registry graph is reproducible. Do not select libc 1.0 prereleases or any unpinned git branch.

3. Centralize direct registry dependencies shared by multiple retained crates under workspace.dependencies and use workspace inheritance in consumers. Do not centralize dependencies used by only one retained crate just for formatting.

4. Perform these major-version changes as separate reviewable substeps:

   - update toml 0.9 to 1.x and run storage-types partition catalog tests;
   - update vergen 8.x to 10.x, adapt storage-app/build.rs if required, and verify VERGEN_GIT_SHA plus VERGEN_GIT_COMMIT_DATE still emit;
   - do not upgrade syn for storage-macros: that crate is deleted in Task 6, so adapting dead macro code is waste.

   After each substep, build the affected retained package and run the behavior-focused test that protects the changed area. For example, run the partition catalogue test after the toml update and the build-script metadata check after the vergen update. Do not run broad checks merely as a mechanical completeness signal.

5. On COSMIC Wayland, smoke-test the current app with frosted appearance enabled and with opaque fallback. Verify window, dialogs, menus, and popovers. Do not add unmerged frosted-glass patches.

**Gate:** Visually review the dependency and lockfile diff against every requirement in this task, and run the targeted catalogue/build-script tests needed by the changes. Do not change the service boundary until the reviewed dependency baseline is acceptable.

### Task 2: Make the backend contract real and backend-neutral

The current storage-contracts crate is unused. Retaining it unchanged would still couple a caller to UDisks2 through methods such as resolve_block_path_for_device, get_disk_info_for_drive_path, and caller_uid. Replace it with the stable application-facing contract before moving any service behavior.

**Files**

- Modify: storage-contracts/src/{lib.rs,protocol/{error.rs,id.rs,operation.rs},traits/{mod.rs,discovery.rs,disk.rs,filesystem.rs,image.rs,luks.rs,partition.rs}}
- Create: storage-contracts/src/traits/{backend.rs,btrfs.rs,network.rs}
- Modify: storage-types/src/{lib.rs,rclone/mod.rs}
- Create: storage-types/src/network.rs
- Modify: storage-udisks/Cargo.toml
- Modify: storage-udisks/src/lib.rs
- Create: storage-udisks/src/backend.rs
- Modify: storage-udisks/src/disk/{resolve.rs,power.rs,device_apis.rs,discovery.rs}
- Modify: storage-udisks/src/partition/{create.rs,delete.rs,edit.rs,resize.rs}
- Modify: storage-udisks/src/filesystem/{mount.rs,format.rs,check.rs,label.rs,config.rs,ownership.rs}
- Modify: storage-udisks/src/encryption/{list.rs,format.rs,unlock.rs,lock.rs,passphrase.rs,config.rs}
- Modify: storage-udisks/src/image/{backup.rs,loop_setup.rs}
- Modify: storage-udisks/src/smart/{info.rs,test.rs}
- Modify: storage-sys/Cargo.toml
- Modify: storage-sys/src/rclone/{mod.rs,systemd.rs,mount_state.rs}
- Create: storage-sys/src/rclone/backend.rs
- Modify: storage-btrfs/{Cargo.toml,src/lib.rs}
- Create: storage-btrfs/src/backend.rs

**Steps**

1. Retain StorageError, StorageErrorKind, OperationId, OperationKind, OperationProgress, and OperationEvent as transport-neutral domain contracts. Serialization derives may remain for persistence or UI state, but neither the app nor an adapter may serialize them to communicate with another in-workspace process.

2. Replace UDisks-shaped adapter traits with narrow domain traits. Keep distinct DiskDiscovery, DeviceEventSource, DriveOperations, PartitionOperations, FilesystemOperations, EncryptionOperations, ImageDeviceOperations, and BtrfsOperations traits. DeviceEventSource returns a boxed stream of typed storage_types::DeviceEvent values. Remove UDisks implementation details from every signature: no D-Bus object path, block-object-path resolution, zbus type, or caller UID belongs in a contract. Inputs and results use storage-types domain values and stable device identifiers only.

3. Add object-safe metadata and aggregate traits in traits/backend.rs. Put identity and capability methods on the metadata constituent so the aggregate traits can have valid blanket implementations:

   ~~~rust
   pub trait BackendMetadata: Send + Sync {
       fn id(&self) -> BackendId;
       fn capabilities(&self) -> StorageBackendCapabilities;
   }

   pub trait BlockStorageBackend:
       BackendMetadata
       + DiskDiscovery
       + DeviceEventSource
       + DriveOperations
       + PartitionOperations
       + FilesystemOperations
       + EncryptionOperations
       + ImageDeviceOperations
       + Send
       + Sync
   {}

   pub trait BtrfsBackend: BackendMetadata + BtrfsOperations {}

   impl<T> BlockStorageBackend for T where
       T: BackendMetadata + DiskDiscovery + DeviceEventSource + DriveOperations
          + PartitionOperations + FilesystemOperations + EncryptionOperations
          + ImageDeviceOperations + Send + Sync {}
   impl<T> BtrfsBackend for T where T: BackendMetadata + BtrfsOperations {}
   ~~~

   Add BackendId and StorageBackendCapabilities to storage-types; capabilities let the UI disable unsupported workflows without checking for UDisks2.

4. Add a separate generic NetworkDriveBackend in traits/network.rs. It identifies itself with NetworkBackendId and exposes capability metadata, a NetworkDriveConfigurationSchema, plus typed list, create, update, delete, test, mount, unmount, status, and mount-on-login operations. It must not mention RCloneCli, ConfigScope, /etc/rclone.conf, or systemd unit names.

5. Add generic storage_types::network models: NetworkBackendId, NetworkBackendAvailability, NetworkDriveConfig, NetworkDriveList, NetworkDriveStatus, NetworkDriveMount, NetworkDriveCapabilities, NetworkDriveConfigurationSchema, NetworkDriveProviderSchema, NetworkDriveField, NetworkDriveFieldInputKind, and NetworkDriveFieldExample. NetworkBackendAvailability is either Available or Unavailable with a user-safe reason. A configuration has a backend ID, stable config ID/name, provider ID, and backend-owned String option map. A configuration schema owns ordered provider schemas; each provider schema has a stable provider ID, label, description, and ordered fields. Every field has a stable option key, label, help text, section, explicit input-kind enum, default value, examples/choices, required, secret, advanced, and visible metadata. The input-kind enum must represent Boolean, Integer, Choice, and Text with an optional backend validation format, so every current rclone value type has a deterministic generic rendering. The app renders the supplied order and submits String option values; the adapter validates backend-specific formats before command execution. Update the app to use the generic network types in Task 4. System scope is not represented anywhere in the new app or backend API. The current storage_types::rclone models remain only as temporary source compatibility for the unchanged service; Task 6 deletes them and moves the provider catalogue into the rclone adapter.

6. Keep async-trait because these traits are stored behind dyn trait objects. Update trait documentation from service/tool contracts to backend contracts. Add contract tests that compile mock block and network backends and prove every app-required operation is object-safe.

7. Implement UdisksBackend in storage-udisks/src/backend.rs. It owns exactly one DiskManager and implements BlockStorageBackend plus DeviceEventSource by delegating to private or crate-visible connection-aware helpers. Move every current Connection::system call behind helpers that receive the backend's connection. The app is never allowed to call those helpers directly. UdisksBackend is the only retained component that knows UDisks object paths, proxy builders, and UDisks capability fields. Keep legacy zero-connection entry points only while storage-service still compiles; delete them in Task 6.

8. Implement BtrfsUtilBackend in storage-btrfs/src/backend.rs. It implements BtrfsBackend by owning the existing disks_btrfs behavior, so app operation code never calls SubvolumeManager directly.

9. Implement RcloneNetworkBackend in storage-sys/src/rclone/backend.rs. It implements NetworkDriveBackend with ID rclone, owns RCloneCli, maps the provider catalogue into NetworkDriveConfigurationSchema, and exposes only per-user configuration and user mount-on-login behavior. Its public API accepts generic network models only: it has no ConfigScope input, does not read or write /etc/rclone.conf, and does not manage system units. Other network backend crates will implement the same contract without modifying app orchestration.

10. Update the root workspace dependencies so storage-contracts depends only on storage-types, while storage-udisks, storage-sys, and storage-btrfs depend on storage-contracts. This preserves dependency direction:

   ~~~text
   storage-types  <-  storage-contracts  <-  storage-udisks / storage-sys / storage-btrfs / future adapters
                                             <-  application composition root
   ~~~

11. Add adapter tests with mockable helpers. Test UdisksBackend construction reuses its one DiskManager connection, BtrfsUtilBackend returns typed Btrfs values, and RcloneNetworkBackend issues configuration and mount-on-login commands only for the desktop user’s config and user-unit locations. Keep pure resolver and normalization tests in storage-udisks.

**Gate:** Review every public contract signature and adapter boundary against this task, then run the mock-contract and adapter tests that exercise the retained behavior. The app-facing API must be trait-based, and no app module may import a concrete UDisks function except the composition root that constructs UdisksBackend.

### Task 3: Create typed application operations

**Files**

- Create: storage-app/src/operations/{mod.rs,error.rs,disk.rs,partition.rs,filesystem.rs,protected_paths.rs,luks.rs,btrfs.rs,image.rs,network.rs}
- Modify: storage-app/src/main.rs
- Modify: storage-app/Cargo.toml

**Steps**

1. Implement a BackendRegistry and StorageOperations::new. BackendRegistry contains one Arc<dyn BlockStorageBackend>, an optional Arc<dyn BtrfsBackend>, a map from NetworkBackendId to Arc<dyn NetworkDriveBackend>, and a NetworkBackendAvailability record for each known optional network backend. The composition root constructs UdisksBackend once and registers it as the selected block backend. Rclone availability means only that RCloneCli can resolve an executable: if absent, do not register the adapter and record Unavailable with the resolver reason; if present, register it without reading user configuration. A missing user config lists as an empty configuration set; a malformed user config returns InvalidInput from the affected list or mutation while keeping the adapter registered; any other rclone construction error records Unavailable without failing app initialization. Detect filesystem tools through the selected backend and initialize ImageOperationManager. The rest of the app sees only traits and typed contract models.

2. Add an app dependency on storage-contracts and retain direct storage-udisks and storage-btrfs dependencies only in the composition root that constructs concrete adapters. Keep storage-sys available for backend-neutral local image-copy support, but no app module outside the composition root may import storage_sys::rclone. All backend calls outside the composition root go through storage-contracts and storage-types. Retain the app's current feature names and move feature checks from service policies into the adapter or operation module using them.

3. Implement operations/disk.rs:

   - list disks, volumes, a disk, and a volume through dyn BlockStorageBackend;
   - preserve the service disk match rule for /dev names, basename, and disk ID;
   - map SMART data to SmartStatus and SmartAttribute;
   - accept short and extended SMART tests only;
   - preserve eject, power-off, standby, wake, and safe-remove behavior using backend capability fields.

4. Implement operations/partition.rs:

   - list partitions through PartitionOperations;
   - normalize gpt directly and dos, mbr, or msdos to dos;
   - create tables, raw partitions, CreatePartitionInfo partitions, delete, resize, set type, flags, and name.

   Return storage-types values directly. Neither module may import serde_json.

5. Implement operations/filesystem.rs:

   - copy tool detection and format/check capability rules from policies/filesystem.rs, returning typed errors and FilesystemToolInfo;
   - implement listing, format, mount, unmount, blocking process lookup, check/repair, label, summary usage, mount options, and ownership through FilesystemOperations;
   - preserve busy-unmount resolution. Resolve the mount point and call is_protected_path before any kill request. A protected path returns the existing refusal result and must never call kill_processes;
   - move storage-service/src/protected_paths.rs verbatim to operations/protected_paths.rs with its tests;
   - retain absolute-path, root-path, exists, and regular-file validation for usage deletion, then call std::fs::remove_file as the current process and capture each OS error in UsageDeleteResult;
   - do not copy is_owned_tree, caller_can_unlink, path_requires_admin_delete, cross-user resolve_caller_groups, or any Polkit branch;
   - retain only current_process_groups, adapted from uid_groups.rs, and use the actual effective UID plus groups in ScanConfig.

6. Implement a UsageScanManager in operations/filesystem.rs, keyed by the UI scan_id. Its synchronous `start_usage_scan(scan_id, request, on_progress, on_complete)` validates selected mounts, computes the estimate, and spawns scan_paths_with_progress in spawn_blocking. `on_progress` receives typed local progress updates and `on_complete` receives the one typed success or error result; both callbacks run off the UI thread and must forward messages rather than mutate UI state. Reject a duplicate active scan_id as InvalidInput. Publish summed byte deltas every 120 ms, apply the current System/Packages filtering when show_all_files is false, set total_free_bytes, and publish a final progress update before invoking completion exactly once. The manager retains only active scans and removes the ID immediately after completion; it exposes no subscribe, cancel, or forget API. Show all files means files accessible to the current process only; no authorization probe remains.

7. Implement operations/luks.rs through EncryptionOperations for LUKS list, format, unlock, lock, passphrase, and option APIs. Preserve normalization: empty or luks2 becomes luks2; luks1 remains luks1; every other string is invalid input.

8. Implement operations/btrfs.rs through the optional BtrfsBackend. The initial adapter uses disks_btrfs::SubvolumeManager, but the app module must not. Preserve feature availability, typed SubvolumeList, DeletedSubvolume, FilesystemUsage, and every existing mutation. There are no signals; callers refresh after a successful mutation.

9. Move the useful image handler logic into operations/image.rs:

   - backup and restore validate paths, generate UUID IDs, obtain file descriptors through ImageDeviceOperations, and use the existing storage-sys copy code inside spawn_blocking;
   - each active ID holds kind, source, destination, cancel token, progress, and a local completion watch channel;
   - status returns a typed status; subscribe returns local progress/completion; cancel marks the token; forget removes only a completed operation after UI consumption;
   - preserve the actual cancellation contract: it is checked before and after the current non-interruptible blocking copy. Do not claim immediate I/O interruption;
   - loop_setup validates the path and delegates to ImageDeviceOperations without interpreting a UDisks object path.

10. Replace operations/rclone.rs with operations/network.rs. It routes generic NetworkDriveConfig and NetworkDriveMount operations through BackendRegistry by NetworkBackendId. The initial rclone adapter preserves per-user configuration, test, mount/status/unmount, mount-on-login, and provider-field validation. No generic operation, adapter API, app state, or message accepts ConfigScope. A future network-drive backend registers the same NetworkDriveBackend trait and appears through capability metadata; it must not require an app state or operation-module rewrite.

11. Add unit tests before UI wiring:

   - disk matching and partition-table normalization;
   - LUKS normalization;
   - filesystem tool detection and unsupported format;
   - protected mounts never producing a kill request;
   - selected mount validation, category filtering, invalid delete paths, and scan callback lifecycle (duplicate IDs, final progress, and exactly one completion);
   - backend-registry routing, mock non-UDisks block backend behavior, mock network backend behavior, and rclone’s per-user command/path behavior;
   - image validation and operation lifecycle helpers without requiring a physical device.

**Gate:** Review each operations module against its corresponding service behavior and run its behavior-focused unit tests while the service is still present. No operation imports a service crate.

### Task 4: Rewire state, tasks, subscriptions, and scope UI

**Files**

- Modify: storage-app/src/{app.rs,state/app.rs,message/app.rs,subscriptions/app.rs}
- Modify: storage-app/src/models/{load.rs,ui_drive.rs,ui_volume.rs,helpers.rs}
- Modify: storage-app/src/update/{mod.rs,drive.rs,smart.rs,btrfs.rs,network.rs}
- Modify: storage-app/src/update/image/{ops.rs,dialogs.rs}
- Modify: storage-app/src/update/volumes/{btrfs.rs,create.rs,encryption.rs,filesystem.rs,mount.rs,mount_options.rs,partition.rs}
- Modify: storage-app/src/{message/network.rs,state/network.rs,views/network.rs,controls/icons.rs}
- Delete: storage-app/src/client/connection.rs
- Delete: storage-app/src/client/error.rs
- Delete: storage-app/src/client/{disks.rs,partitions.rs,filesystems.rs,luks.rs,btrfs.rs,image.rs,rclone.rs,mod.rs}

**Steps**

1. Add operations: Option<Arc<StorageOperations>> to AppModel. In Application::init, dispatch only Task::perform(StorageOperations::new, ...), yielding a new OperationsInitialized(Result<Arc<StorageOperations>, OperationError>) message. UdisksBackend construction is mandatory: map its failure to Unavailable or PermissionDenied and retain an error state without attempting client construction. Btrfs tooling and rclone are optional registrations: unavailable tooling produces absent/disabled capabilities, not an initialization failure. On success, store the context, map each NetworkBackendAvailability record into the corresponding backend-generic NetworkState availability/status, launch initial drive and filesystem-tool loads, and launch network loads only for registered backends. A malformed user config is shown as that backend's editable InvalidInput error, while an unavailable backend shows its disabled reason and has no operation controls.

2. Replace every crate::client import and every Client::new call in the files above with a captured Arc<StorageOperations>. UiDrive and UiVolume store that context, not individual clients. Change load_all_drives to accept &StorageOperations.

3. Delete all service signal handling. In subscriptions/app.rs, subscribe through the selected BlockStorageBackend DeviceEventSource and map typed storage_types::DeviceEvent Added and Removed to the existing DriveAdded and DriveRemoved messages. Delete StorageEventsSubscription. Do not import DiskManager or storage_udisks from this module.

4. Make every successful format, mount, unmount, partition, LUKS, and Btrfs task schedule the current navigation refresh directly. Failed tasks do not schedule a success refresh.

5. When UsageScanLoad starts, call UsageScanManager::start_usage_scan with callbacks that forward typed progress and completion through the app-local message channel as the existing UsageScanProgress and UsageScanLoaded messages. Keep active_scan_id solely to discard stale callback messages; clear it when the matching completion is handled. There is no usage subscription, cancellation, or forget operation.

6. Keep the existing 400 ms image status cadence, but obtain status and completion from ImageOperationManager. After ImageOperationDialogMessage::Complete, forget the operation ID.

7. Delete UsageShowAllFilesAuthCompleted and UsageWizardShowAllFilesAuthCompleted plus their update arms. Both checkbox toggles update local state directly. Preserve selected-mount wizard behavior, scan ID generation, filtering, progress display, and deletion result handling.

8. Make the initial rclone-backed network UI User-only while keeping it backend-generic:

   - remove both User/System dropdowns from views/network.rs;
   - replace rclone RemoteConfig/RemoteConfigList/MountStatusResult state with generic NetworkDriveConfig, NetworkDriveList, NetworkDriveStatus, and NetworkBackendAvailability state keyed by NetworkBackendId;
   - initialize the rclone editor/wizard as User and show a non-editable User label only if needed;
   - remove scope icon and tooltip, System list partitioning, System sort branches, NetworkState::system_mounts, NetworkMessage::EditorScopeChanged, NetworkMessage::WizardSetScope, and their update arms;
   - remove ConfigScope and every legacy rclone state, message, helper, and system configuration/systemd-unit branch from the final app and adapter; the app state and messages must not expose it;
   - select a backend from the registry by NetworkBackendId and render its supplied NetworkDriveConfigurationSchema in the existing generic editor/wizard, so a newly registered network backend does not require a second app control flow or a backend-specific view.

9. Audit and remove direct application dependencies only after the rewiring. Visually review the changed module imports and manifests: `client`, generated proxy code, and service-only JSON usage are gone; concrete UDisks, Btrfs, and rclone imports remain limited to the composition root as required. serde remains needed for config.rs, anyhow remains used by task composition, and thiserror may remain for OperationError; remove only direct manifest dependencies proven unused.

**Gate:** Review the rewired state, task, subscription, and scope-UI flows against every step in this task, then run the unit tests that cover refresh scheduling, scan/image progress, and backend routing. The app must enumerate disks through a mockable backend while the service is stopped; no app module outside the composition root may import a concrete adapter.

### Task 5: Validate the direct app before deletion

Use one reusable validation fixture: a clean VM with UDisks2 but no project service artifacts; disposable loop-backed partitioned, LUKS, and Btrfs images; a controlled process holding a file descriptor or working directory in a mounted filesystem for the busy-unmount case; checksummed image-copy inputs; and a per-user rclone test remote backed by a disposable local directory. Enable the FUSE capability required for the rclone mount test. Do not use personal credentials or a pre-existing system rclone configuration.

1. In a clean VM or test installation, launch the app and confirm:

   ~~~sh
   busctl --system status org.cosmic.ext.Storage.Service
   pgrep -af cosmic-ext-storage-service
   ~~~

   both find no project service. Confirm startup discovers drives and does not launch a full usage scan.

2. Record the fixture identity, action, expected result, pass/fail, and actual user-visible error for each:

| Area | Required exercise |
| --- | --- |
| Device events | Startup, loop/physical device add and remove |
| Filesystems | Mount, unmount, busy prompt, protected refusal, format, check, label, options, ownership |
| Partition/LUKS | Table/create/edit/delete/resize, unlock/lock/options/passphrase |
| Usage | Multi-mount scan, incremental progress, filtering, accessible deletion, denied deletion |
| Btrfs | Read and each mutation available to the desktop user |
| Images | Backup, restore, loop setup, progress, completion, cancellation |
| rclone | Per-user config/test/mount/status/unmount/user mount-on-login; no system config or unit is read, written, or offered |

3. For any rejected workflow, either remove/disable that workflow or stop for an approved privilege-architecture decision. Do not restore the service, add a policy, or substitute a User operation for a System request.

**Gate:** Visually review the recorded desktop-session matrix and its user-visible outcomes. Every retained workflow is usable by the desktop user or explicitly removed with an approved decision.

### Task 6: Remove the service boundary

**Files**

- Delete: storage-service/
- Delete: storage-macros/
- Retain for Task 7 move: storage-contracts/
- Delete: resources/systemd/
- Delete: storage-types/src/caller.rs
- Delete after moving its provider catalogue: storage-types/src/rclone/
- Modify: storage-types/src/lib.rs
- Modify: storage-udisks/src/{lib.rs,backend.rs} and legacy wrapper modules
- Modify: storage-sys/src/rclone/ to retain only the generic-model, per-user adapter implementation
- Modify: Cargo.toml
- Modify: Cargo.lock

**Steps**

1. Delete the two whole crate directories and these exact artifacts:

   - resources/systemd/cosmic-ext-storage-service.service
   - resources/systemd/cosmic-ext-storage-service.socket
   - resources/systemd/org.cosmic.ext.Storage.Service.conf
   - resources/systemd/org.cosmic.ext.storage.service.policy

2. Remove caller.rs and the caller module/re-export from storage-types/src/lib.rs. Its only consumers are the deleted macro and service. Move the rclone provider catalogue into storage-sys/src/rclone/, then delete storage-types/src/rclone/ and its public module/re-exports. Delete ConfigScope and all legacy rclone system configuration and system-unit helpers rather than preserving a compatibility API.

3. Delete the legacy no-connection public helpers retained only for storage-service compatibility. Keep UdisksBackend and its private connection-aware helpers. Confirm all retained UDisks calls originate from the one UdisksBackend connection.

4. Remove the deleted workspace members but retain storage-contracts as a workspace dependency. Remove zbus_polkit, tokio-util, and macro-only syn, quote, and proc-macro2 dependencies after reviewing the retained manifests and direct uses. Retain async-trait for the object-safe backend contracts, and retain zbus/zbus_macros because storage-udisks uses UDisks2 D-Bus.

5. Let Cargo prune the lockfile with cargo check --workspace, then cargo update only if Cargo requires it. Never hand-edit Cargo.lock.

6. Visually review the retained source, manifests, packaging resources, and user-facing documentation. Confirm that the removed service, service binary, authorization attribute, project D-Bus name, and `zbus_polkit` are absent outside historical documentation. Do not rewrite historical plans merely to satisfy this review.

**Gate:** Inspect the deletion diff and retained UDisks boundary. The service-only caller model, service crates, policies, and no-connection compatibility helpers are gone; `storage-contracts` and `UdisksBackend` remain with the required dependency direction.

### Task 7: Move the app to the root and retained libraries to crates/

**Moves**

- storage-app/src/ → src/
- storage-app/build.rs → build.rs
- storage-app/i18n/ → i18n/
- storage-app/i18n.toml → i18n.toml
- storage-app/resources/{app.desktop,app.metainfo.xml,icons/} → resources/
- storage-btrfs/ → crates/storage-btrfs/
- storage-sys/ → crates/storage-sys/
- storage-contracts/ → crates/storage-contracts/
- storage-types/ → crates/storage-types/
- storage-udisks/ → crates/storage-udisks/
- resources/types/ → crates/storage-types/resources/types/
- resources/rclone/providers.json → crates/storage-sys/resources/rclone/providers.json

Use git mv for every tracked move.

**Steps**

1. Make root Cargo.toml both the package manifest and workspace manifest. Move the package, feature, dependency, and build-dependency sections from storage-app/Cargo.toml to the root. Retain package name cosmic-ext-storage. Let Cargo infer root build.rs unless it requires an explicit build key.

2. Set exactly:

   ~~~toml
   members = [
       ".",
       "crates/storage-btrfs",
       "crates/storage-sys",
       "crates/storage-contracts",
       "crates/storage-types",
       "crates/storage-udisks",
   ]
   default-members = ["."]
   ~~~

   Update workspace dependency paths to crates/*. Add root direct dependencies on storage-contracts, storage-udisks, storage-btrfs, storage-sys, and storage-types. Limit concrete backend imports to src/operations/mod.rs. Propagate an app feature only to a dependency that declares that feature.

3. Keep app assets at root resources. Move partition type TOML into storage-types resources and rclone provider JSON into storage-sys resources. Update exactly:

   - crates/storage-types/src/partition_types/catalog.rs to use ../../resources/types/<file>.toml;
   - crates/storage-sys/src/rclone/provider_catalog.rs to use ../../resources/rclone/providers.json;
   - src/views/settings.rs and src/controls/icons.rs retain their correct ../../resources includes after the move;
   - resources/app.metainfo.xml remote icon URL from storage-app/resources to resources.

   Keep src/i18n.rs folder = i18n/ and build.rs rerun-if-changed=i18n unchanged after the move.

4. Merge still-correct storage-app/README.md material into README.md, merge its vendor ignores into a new root .gitignore, then remove storage-app/Cargo.toml, README.md, justfile, .gitignore, and the now-empty storage-app/ directory.

5. Update every affected manifest path, include macro, RustEmbed path, just recipe, README link, workflow, release script, and editor config. Run:

   ~~~sh
   cargo metadata --no-deps --format-version=1
   cargo check --workspace --all-features --locked
   ~~~

   Metadata must show the root app and exactly five crates/* library packages.

**Gate:** Visually review the resulting tree, every moved manifest path, and each resource/include path. Run the focused catalogue/resource tests and a normal workspace build so broken move paths are caught before workflow documentation changes.

### Task 8: Replace service-oriented developer, package, and release workflows

**Files**

- Modify: justfile
- Create or modify: .gitignore
- Modify: README.md
- Modify: .github/workflows/{main.yml,update-version.yml}
- Modify: .vscode/launch.json
- Delete: storage-app/README.md
- Delete: storage-app/justfile
- Delete: storage-app/.gitignore

**Steps**

1. Make root justfile the only entry point. Retain build, release, check, app/run, install, uninstall, vendor, vendor-extract, build-vendored, clean-vendor, and clean-dist. The default recipe builds and launches the app directly without sudo.

   check must run in this order:

   ~~~sh
   cargo fmt --all -- --check
   cargo test --workspace --all-features --locked
   cargo clippy --workspace --all-features --locked
   ~~~

   Retain root application binary/desktop/metainfo/icon installation. Delete service start/stop/status/log/introspection, D-Bus monitoring, policy install, systemd install, and service binary installation. Remove unused Polkit/D-Bus development packages from install-deps while retaining UDisks2 and libraries needed by retained crates.

2. Rewrite root README as the canonical user/developer document. State that the currently shipped UdisksBackend requires UDisks2 and no project daemon, root command, D-Bus policy, or Polkit policy is installed. Describe the backend-contract boundary without promising unshipped backends, replace service quick-start commands, describe User-only rclone accurately, retain safety/early-beta notes and image-operation notes, and correct all resource links.

3. Repair main.yml:

   - read and compare the tag version against root Cargo.toml and every publishable crates/* manifest;
   - run cargo package --allow-dirty --no-verify for each package first, so unpublishable path dependencies fail before any publish;
   - publish in topological order: storage-types, storage-contracts, storage-btrfs and cosmic-ext-storage-storage-sys, storage-udisks, then cosmic-ext-storage. After every published dependency layer, poll `cargo info <package>@<version> --registry crates-io` every 15 seconds for at most 20 attempts and proceed only after every package in that layer resolves at the expected version; fail the release job with the unresolved package names on timeout;
   - use cargo publish --manifest-path <manifest> --locked for every package and the actual package names in workflow labels; do not preserve the stale cosmic-ext-storage-udisks label.

4. Repair update-version.yml so it changes root Cargo.toml, all five crates/* package versions, and every internal workspace dependency version together. Its pull-request add-paths must list exactly those manifests. Verify package names with cargo metadata before changing a publish command.

5. Replace the two stale VS Code configurations with one lldb launch entry for --package=cosmic-ext-storage --bin=cosmic-ext-storage. Remove references to hardware-disks-rs and cosmos-apxui.

**Gate:** Review the final justfile, installation recipes, README, release workflows, version workflow, and editor configuration against this task. The reviewed workflows must package and publish the intended dependency order without service-era behavior.

### Task 9: Final verification and acceptance

1. From the final root layout, run the normal release build and the behavior-focused unit and adapter tests added or changed by this migration. Use `env -u RUSTC_WRAPPER` locally while the recorded wrapper problem remains; CI should use its normal compiler configuration. Do not use duplicated aggregate recipes or bulk source-search scripts as proof of implementation completeness.

2. Inspect metadata: it must list six workspace packages, the root app plus exactly five retained libraries. Inspect the release target directory: it must never contain cosmic-ext-storage-service. storage-btrfs-cli is permitted only as its intentional optional utility target.

3. Review the final source tree, manifest dependency graph, app import boundaries, packaging resources, install recipes, documentation, and release configuration against Tasks 1–8. This visual review—not a bulk source-search script—is the completion check for those requirements.

4. Install only the final application package in a clean test environment. Confirm it installs no project unit, policy, or D-Bus configuration and repeat Task 5's matrix. Run the contract mock tests for a non-UDisks block backend and a second network backend as part of the same acceptance run. Record results in docs/plans/1-service-removal/validation.md.

The change is accepted only when:

- cargo reports the root application and exactly five retained crates under crates/;
- no project service, service binary, custom policy, system unit, or normal workflow reference remains;
- the UdisksBackend creates one UDisks2 system-bus connection and every UDisks operation uses it, while app modules consume only backend contracts;
- startup never starts a full scan, while user scans retain local scan-ID progress and results;
- hotplug, post-mutation refresh, image progress/completion, and usage progress work without project D-Bus signals;
- the rclone NetworkDriveBackend works only with per-user configuration and user mount-on-login behavior, and a mock second network backend proves registry routing without app-flow changes;
- automated and clean-desktop validation pass from the final package layout.

**Gate:** Complete a final visual review of this plan, task by task, against the implemented diff and validation record. Run only the release build and behavior-focused tests that exercise the retained functionality; resolve every unmet requirement before accepting the migration.

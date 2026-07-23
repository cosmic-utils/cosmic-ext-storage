# Remove the Storage Service and Flatten the App Workspace

**Status:** Derived implementation specification; implementation record in [validation.md](validation.md)
**Authority:** [implementation-plan.md](implementation-plan.md) is the canonical task order, implementation detail, and gate definition. This document records the intended architecture and acceptance state. If the documents differ, the implementation plan governs; update that plan before changing this specification.
**Scope:** Remove the project-owned root D-Bus service, run storage operations in the application process through a backend-neutral trait API, and make the application the root package while moving retained libraries to `crates/`.

## Outcome

The repository has one executable crate at its root: `cosmic-ext-storage`. It no longer starts, installs, owns, or calls `org.cosmic.ext.Storage.Service`. The application consumes typed backend contracts; its composition root constructs the currently shipped UDisks2 and rclone adapters. UDisks2 remains the default required system service, but it is not part of the application-facing API.

The final tree is intentionally shaped as follows (omitting ordinary source children):

```text
.
├── Cargo.toml                 # application package + workspace definition
├── build.rs
├── src/
├── i18n/
├── i18n.toml
├── resources/                 # application desktop metadata, icons, screenshots
├── crates/
│   ├── storage-btrfs/
│   ├── storage-sys/
│   ├── storage-contracts/
│   ├── storage-types/
│   └── storage-udisks/
├── docs/
├── .gitignore
├── justfile
└── README.md
```

`storage-service` and `storage-macros` are deleted. The first is the removed daemon and the latter only supplies its authorization attribute. `storage-contracts` is retained and moved to `crates/` as the backend-neutral application boundary; its current lack of consumers is corrected by this migration rather than used as a reason to delete it.

## Architectural decisions

1. **Remove only this project’s daemon.** UDisks2 remains a required system service. The shipped `UdisksBackend` adapter will continue using its system-bus API through `storage-udisks`.
2. **No replacement privileged helper.** The app runs as its desktop user.  Operations backed by UDisks2 use UDisks2’s native Polkit integration; the app must not install a custom D-Bus policy, a project Polkit policy, or a root systemd unit.
3. **The full filesystem scanner is on demand.** Startup may collect inexpensive UDisks2/statvfs data, but must not start `storage_sys::usage::scan_paths_with_progress`.  A full scan begins only after the user requests it.
4. **Use typed in-process backend contracts.** Do not reproduce the service protocol inside the app. Replace JSON-over-D-Bus client wrappers with object-safe `storage-contracts` traits, `storage-types` values, and one application-side operation error. Contracts must not expose D-Bus object paths, zbus types, proxy builders, caller identities, rclone CLI types, or systemd-unit details.
5. **Keep concrete adapters at the composition root.** The app owns a `BackendRegistry` containing `Arc<dyn BlockStorageBackend>`, optional `Arc<dyn BtrfsBackend>`, and network adapters keyed by `NetworkBackendId`. Only the composition root may construct or import `UdisksBackend`, `BtrfsUtilBackend`, or `RcloneNetworkBackend`; models, tasks, views, and operation orchestration consume contracts and typed models only. A future local-storage or network-drive backend must not require those app layers to change.
6. **Keep the safety checks, discard caller impersonation.** Path validation for usage deletion and the protected-path guard for force-unmount remain. Checks that existed only because a root daemon was acting on another user’s behalf are removed; normal filesystem permissions now apply to the app process itself.
7. **Rclone is strictly per-user.** The final workspace has no `ConfigScope`, system rclone configuration, or system rclone unit support. The app and `RcloneNetworkBackend` accept generic network models only and never offer a scope choice. Legacy rclone code remains only long enough for the unchanged service to compile and is deleted with that service.
8. **Do not promise elevation for direct Btrfs operations.** Exercise Btrfs, image, and ownership operations as an unprivileged desktop user during validation. If a required workflow cannot be authorized by UDisks2 or the user’s existing filesystem permissions, surface the underlying error and decide separately whether to drop that workflow or design a new privilege boundary. Reintroducing a hidden daemon is out of scope.

These decisions make the privilege change visible instead of leaving a partially functional service removal.

## Migration map

| Current boundary | Direct replacement | Required behavior |
| --- | --- | --- |
| `storage-app/src/client/disks.rs` | `dyn BlockStorageBackend`, initially `UdisksBackend` | Reuse the adapter’s one system-bus connection; preserve disk, volume, SMART, power, and removal actions. |
| `client/partitions.rs` | `dyn PartitionOperations` through `BlockStorageBackend` | Keep table creation, combined create-and-format, delete, resize, type, flag, and name operations. |
| `client/filesystems.rs` | `dyn FilesystemOperations` plus `storage-sys::usage` | Preserve tool detection, formatting, mount/unmount busy-process handling, checks, labels, mount options, ownership, scan, and guarded deletion. |
| `client/luks.rs` | `dyn EncryptionOperations` through `BlockStorageBackend` | Preserve unlock/lock, passphrase, and encryption-options flows. |
| `client/btrfs.rs` | `dyn BtrfsBackend`, initially `BtrfsUtilBackend` | Preserve the Btrfs UI’s typed `SubvolumeList` and usage results without serialization. |
| `client/image.rs` | App-side image manager using `dyn ImageDeviceOperations` + `storage-sys` | Preserve operation IDs, status, progress polling, completion, and cancellation semantics. |
| `client/rclone.rs` | Generic app network operation module using `dyn NetworkDriveBackend`, initially `RcloneNetworkBackend` | Preserve per-user config, mount, status, and mount-on-login behavior. No final app or adapter API accepts a scope or manages system configuration. |
| Service disk signals | `dyn DeviceEventSource` through `BlockStorageBackend` | Subscribe to typed device events and translate them to existing `DriveAdded`/`DriveRemoved` messages; only `UdisksBackend` knows UDisks2 `ObjectManager` details. |
| Service filesystem/LUKS signals | Completion messages from the initiating app task | Refresh navigation/state after a successful local mutation; no synthetic D-Bus signals remain. |
| Service usage progress signal | App-local progress channel | Run the blocking scan from the app, forward byte deltas to `UsageScanProgress`, and retain the scan-id correlation used by the UI. |

The unused service-only LVM interface has no corresponding app client.  Its handlers and policy are deleted with the service; UDisks-backed logical-volume display continues through the existing disk model.

## Canonical execution and implementation gates

The implementation plan is executed as Tasks 0–9 in order. A stage advances only after a reviewer visually compares the implemented diff, affected files, and user-visible behavior with that task’s requirements. Automated coverage is kept where it protects retained behavior—such as contract, adapter, and operation unit tests—but bulk source-search scripts, duplicated aggregate recipes, and test runs used only as a completeness signal are not acceptance evidence.

The sections below describe the same stages at specification level. Their gates defer to the corresponding canonical plan task when more detailed evidence is required.

## Specification by canonical stage

### 1. Upgrade libcosmic and the dependency baseline

First capture the reproducible Task 0 baseline: record the starting commit, package list, toolchain, worktree state, and existing diagnostics. The current local `RUSTC_WRAPPER` problem is an environment fact, not a source change; use the plan’s temporary `env -u RUSTC_WRAPPER` workaround for local validation until it is fixed.

Then do this before moving application code or removing the service boundary, so the migration is built and tested against the current COSMIC UI stack rather than adding a large dependency change late in the work.

Pin `libcosmic` to the reviewed upstream `master` commit `ef162b8e16ba4493e05c169cd56c7b9f77f0fda5`, then regenerate `Cargo.lock`.  Do not replace the Git dependency with the upstream `v0.12` tag: it predates the current 1.0 development line and does not contain the blurred-transparency implementation.  Keep the pin immutable and reproducible; do not follow a branch name.

Refresh every stable, SemVer-compatible direct and transitive registry dependency, then align the workspace dependency declarations with the resolved versions.  Centralize the remaining direct registry dependencies in `[workspace.dependencies]` where practical so the later structural move does not reintroduce version drift between crates.

Handle the direct major-version candidates in separate, reviewable substeps after the compatible refresh:

- update `toml` from 0.9 to 1.x and verify partition-type catalogue parsing;
- update `vergen` from 8.x to 10.x and verify the application build script still emits the Git SHA and commit date;
- do not upgrade `storage-macros` or adapt its `syn` dependency: the crate is deleted with the service boundary;
- do not select the `libc` 1.0 prerelease; remain on the latest stable 0.2 release.

After each substep, build the affected retained package and run the test that protects the changed behavior. On a Wayland COSMIC session, manually verify an app window, dialogs, menus, and popovers with frosted windows enabled in COSMIC appearance settings; verify the opaque fallback when the setting or compositor blur support is absent. Do not add either unmerged frosted-glass follow-up patch to this repository; revisit them only once upstream merges them.

**Gate:** Review the dependency and lockfile diff against the canonical Task 1 requirements, then confirm the relevant catalogue/build-script behavior before changing the service boundary.

### 2. Establish the backend contract and application operation boundary before deleting the service

Retain `storage-contracts` and replace its unused UDisks-shaped traits with the stable application-facing API. It retains the transport-neutral `StorageError`, operation IDs, and operation events, and defines narrow object-safe domain traits: `DiskDiscovery`, `DeviceEventSource`, `DriveOperations`, `PartitionOperations`, `FilesystemOperations`, `EncryptionOperations`, `ImageDeviceOperations`, and `BtrfsOperations`. Aggregate `BlockStorageBackend` and `BtrfsBackend` traits include a `BackendMetadata` constituent for a stable backend ID and capability metadata. No contract signature may expose a D-Bus object path, zbus type, caller UID, proxy builder, rclone CLI type, `/etc` path, or systemd unit name.

Define a separate `NetworkDriveBackend` contract and typed generic network models in `storage-types`: `NetworkBackendId`, `NetworkBackendAvailability`, `NetworkDriveConfig`, list/status/mount values, capabilities, `NetworkDriveConfigurationSchema`, provider schemas, fields, input kinds, and field examples. Availability is either available or unavailable with a user-safe reason. Configurations have stable IDs/names, provider IDs, backend IDs, and String option maps. Provider schemas and their fields are ordered; each field has a stable option key, label, help text, section, input kind, default, examples/choices, and required/secret/advanced/visible metadata. The app renders the schema order and submits String values; adapters validate backend-specific formats. The models must not encode `ConfigScope` or rclone-specific provider types.

Implement the initial adapters behind those contracts: `UdisksBackend` owns exactly one `DiskManager` and is the only retained UDisks2-aware component; `BtrfsUtilBackend` owns the existing Btrfs behavior; and `RcloneNetworkBackend` owns `RCloneCli`, maps its provider catalogue to the generic schema, and supports only per-user configuration and mount-on-login. Its public API has no `ConfigScope` input and never reads or writes system configuration or units. Preserve legacy zero-connection UDisks helpers only until the service is deleted, then remove them.

Create an application-owned `src/operations/` boundary with focused `disk`, `partition`, `filesystem`, `luks`, `btrfs`, `image`, `network`, and `error` modules. It owns a shared `StorageOperations` context containing a `BackendRegistry`, cached filesystem capability data, the in-process image-operation map, and local usage-scan state. Construct it once during app initialization and pass `Arc` clones to models/tasks. The registry holds the selected `Arc<dyn BlockStorageBackend>`, optional `Arc<dyn BtrfsBackend>`, registered `NetworkBackendId` to `Arc<dyn NetworkDriveBackend>` map, and an availability record for each optional network backend. Rclone is registered only when its executable resolves; a missing executable records an unavailable reason, missing config lists empty, malformed config is an editable `InvalidInput` operation error, and other construction failure records unavailable without failing app startup. The composition root alone constructs and registers the concrete adapters.

Port the non-D-Bus logic from these service areas through contracts while keeping user-visible validation and result semantics:

- `handlers/disk.rs`, `partition.rs`, `luks.rs`, and `btrfs.rs`: replace each proxy method with its corresponding contract operation and return typed results.
- `handlers/filesystem/mod.rs` and `handlers/filesystem/support/**`: retain filesystem tool detection, busy-unmount resolution, mount-option shaping, usage mount validation, deletion guards, and parallelism mapping. Replace `zbus::fdo::Error` conversion with the app operation error.
- `handlers/image.rs`: retain the operation registry, UUIDs, progress state, and cancellation token as app-local state. Remove its interface annotations and signal emissions; acquire image device file descriptors through `ImageDeviceOperations` without interpreting UDisks paths.
- `handlers/rclone/**`: migrate UI-facing behavior to generic network values. Keep rclone provider parsing inside `RcloneNetworkBackend`; the final app state, messages, and adapter API expose no scope selector or rclone-only configuration type.
- `protected_paths.rs`: move it into the app’s filesystem operation module (with its existing unit tests), because it is a safety invariant independent of D-Bus.

Do not copy the `#[interface]` methods, `authorized_interface` annotations, caller lookup, Polkit checks, JSON encoding, or signal-emitter parameters. Preserve successful result values and user-visible errors, but map library failures through the transport-neutral contract error into the app operation error.

Add contract tests with mock block and network backends to prove object safety and application routing. Add adapter tests showing `UdisksBackend` reuses its single connection, `BtrfsUtilBackend` returns typed values, and `RcloneNetworkBackend` uses only per-user configuration and user-unit locations. Retain focused tests for disk identifier matching, LUKS-version normalization, filesystem tool detection, protected mount decisions, usage mount/path validation, and image status transitions.

**Gate:** Visually review each public contract and adapter boundary against canonical Task 2, then run the contract and adapter tests that exercise the retained behavior.

### 3. Rewire application state, tasks, and subscriptions

Replace all imports and stored `*Client` fields in `src/models/**`, `src/update/**`, `src/app.rs`, and `src/subscriptions/app.rs` with the shared operation context. Remove `src/client/connection.rs` and all generated `zbus::proxy` traits. Rename the module from `client` to `operations` so comments, types, and documentation no longer describe a non-existent storage-service client. Other than the composition root, application modules must not import `storage_udisks`, `disks_btrfs`, or `storage_sys::rclone`.

Handle the three asynchronous flows explicitly:

1. In `subscriptions/app.rs`, subscribe through the selected `BlockStorageBackend`'s `DeviceEventSource` and map typed `DeviceEvent::{Added, Removed}` directly to the existing drive refresh messages. The subscription must not import `DiskManager` or `storage_udisks`.
2. For format, mount, unmount, LUKS, partition, and Btrfs changes, have the task that performed the mutation schedule the same refresh it currently receives indirectly through a service signal.  External device changes remain covered by the UDisks2 subscription.
3. For usage scans, call `UsageScanManager::start_usage_scan` with progress and completion callbacks that forward typed messages through the app-local message channel. A scan ID is unique while active; final progress is emitted before its one completion callback, and the manager removes that active ID immediately after completion. There is no scan subscription, cancellation, or forget API. Keep the selected-mount validation, current-user UID/GID configuration, category filtering, final byte update, and result shape. `Show all files` becomes an acknowledgement of the current user’s accessible files rather than a service Polkit check.
4. For images, poll/wait against the app’s image-operation manager, not a D-Bus proxy. Maintain the UI’s operation ID, polling interval, completion message, and cancellation behavior.

Replace rclone-specific app state (`RemoteConfig`, `RemoteConfigList`, and mount status values) with generic `NetworkDriveConfig`, list, status, and availability values keyed by `NetworkBackendId`. Render the registered backend’s configuration schema in the existing editor/wizard and route its messages through `BackendRegistry`. A malformed configuration remains editable with its `InvalidInput` error; an unavailable backend shows its disabled reason and has no operation controls. Registering a second network backend must not require a second app control flow or a backend-specific view.

Use `Result<Arc<StorageOperations>, OperationError>` for initialization. UDisks initialization is mandatory and maps to `Unavailable` or `PermissionDenied`; unavailable Btrfs tooling or rclone yields disabled/absent capabilities rather than application-startup failure. Map network-backend availability into backend-generic UI state, and load only registered backends. The app error type no longer has `ServiceNotAvailable`, D-Bus connection, or proxy-method variants. Retain useful categories for invalid input, permissions, unsupported operations, and failures converted from `StorageError`. Remove `serde_json` or `thiserror` only if a repository-wide use check proves they are no longer used by the application.

**Gate:** Review the operations, state, task, subscription, and network UI rewiring against canonical Tasks 3 and 4. Run the focused tests for refresh scheduling, progress, operation lifecycles, and backend routing; no concrete adapter leaks beyond the composition root.

### 4. Prove the service is unnecessary on a real desktop session

Before deleting packaging artifacts, run the app without `cosmic-ext-storage-service` installed or running.  Confirm that initial drive discovery completes with no full usage scan and that no process claims `org.cosmic.ext.Storage.Service`.

Manually exercise with the reusable fixture defined by the canonical plan: a clean UDisks2 VM without project service artifacts; disposable loop-backed partitioned, LUKS, and Btrfs images; a controlled busy-unmount process; checksummed image-copy inputs; and a per-user rclone test remote backed by a disposable local directory.

Exercise:

- cold startup, disk refresh, and hotplug add/remove;
- mount, unmount, busy-unmount prompt, filesystem format/check/label, and mount-option updates;
- partition create/edit/delete/resize and LUKS lock/unlock/options;
- a multi-mount usage scan, incremental progress, filtering, and permitted file deletion;
- Btrfs read and mutation operations available to the logged-in user;
- image backup, restore, loop setup, progress, completion, and cancellation;
- per-user rclone configuration, mount status, and mount-on-login.

Record any operation that cannot be performed under the new UDisks2/user-permission model.  Resolve it by narrowing the app feature or by proposing a new privilege architecture; do not defer it behind a service fallback.

**Gate:** Review the recorded desktop-session matrix and user-visible outcomes. Every retained workflow is usable by the desktop user or explicitly removed by an approved decision.

### 5. Delete the service boundary and service-only crates

Once the app has no proxy call sites, delete:

- `storage-service/`, including handlers, policies, authorization, protected-path original, and the `cosmic-ext-storage-service` binary;
- `storage-macros/`, because `authorized_interface` has no remaining user;
- `resources/systemd/`, including the systemd unit/socket, D-Bus configuration, and Polkit policy.

Also delete `storage-types/src/caller.rs` and its public module/re-export. It exists solely for the deleted macro/service caller-identity path and must not survive the in-process model. Move the rclone provider catalogue into `storage-sys/src/rclone/`, then delete `storage-types/src/rclone/`, `ConfigScope`, and every legacy rclone system-configuration/system-unit helper rather than preserving a compatibility API.

Retain `storage-contracts` as the real, tested backend boundary. Delete the legacy public UDisks helper entry points that created their own connection only after all service call sites have gone; retain `UdisksBackend` and its private connection-aware helpers.

Update the root workspace manifest and lockfile together:

- remove the two deleted members and their workspace/path dependencies, while retaining `storage-contracts`;
- add the app’s direct dependency on `storage-contracts`, with concrete adapter dependencies limited to the composition root;
- make `storage-contracts` depend only on `storage-types`, and make `storage-udisks`, `storage-btrfs`, and `storage-sys` depend on `storage-contracts` so dependency direction remains `storage-types <- storage-contracts <- adapters <- application composition root`;
- keep feature flags in the application package, moving the service’s filesystem/Btrfs/rclone availability checks with the direct operations that use them;
- remove `zbus_polkit`, `tokio-util`, the proc-macro dependencies, and any other dependencies made unreachable, after reviewing retained manifests and direct uses.

Visually review the retained source, manifests, packaging resources, and user-facing documentation to confirm the removed service, service binary, authorization attribute, project D-Bus name, and `zbus_polkit` are absent outside historical documentation.

**Gate:** Inspect the deletion diff and retained UDisks boundary. The service-only caller model, service crates, policies, and no-connection compatibility helpers are gone; `storage-contracts` and `UdisksBackend` remain with the required dependency direction.

### 6. Move the app to the root and retained libraries under `crates/`

Make the root `Cargo.toml` both the application’s `[package]` manifest and the `[workspace]` manifest.  Set `default-members = ["."]` and list the retained libraries using `crates/*` paths.  Move these application-owned paths out of `storage-app/`:

- `storage-app/src/` → `src/`
- `storage-app/build.rs` → `build.rs`
- `storage-app/i18n/` → `i18n/`
- `storage-app/i18n.toml` → `i18n.toml`
- `storage-app/resources/` → `resources/`
- merge the useful packaging recipes from `storage-app/justfile` into the root `justfile`, then delete the nested file.

Move the retained support crates without renaming their packages:

- `storage-btrfs/` → `crates/storage-btrfs/`
- `storage-sys/` → `crates/storage-sys/`
- `storage-contracts/` → `crates/storage-contracts/`
- `storage-types/` → `crates/storage-types/`
- `storage-udisks/` → `crates/storage-udisks/`

Make resource ownership match the new layout:

- move partition type TOML from root `resources/` into `crates/storage-types/resources/` and rclone provider JSON into `crates/storage-sys/resources/rclone/`, then update their `include_str!` paths;
- retain desktop metadata, app icons, and the screenshot under root `resources/` as application/repository assets;
- update the metainfo remote icon URL from `storage-app/resources/...` to `resources/...`.

Update every manifest `path`, `include_str!`/`include_bytes!`, `RustEmbed` folder, `justfile` source path, README image link, CI script, release script, and editor configuration affected by the moves.  Replace the stale VS Code launch entries with a single root-package `cosmic-ext-storage` launch configuration (or remove them if they are no longer maintained).

Merge still-correct `storage-app/README.md` material into the root README and its vendor ignores into a new root `.gitignore`, then remove the old nested manifest, README, justfile, gitignore, and empty directory.

**Gate:** Visually review the resulting tree, manifests, and resource/include paths. Run focused catalogue/resource tests and a normal workspace build so broken move paths are caught before workflow documentation changes.

### 7. Simplify developer, installation, release, and user documentation

Make the root `justfile` the only task entry point.  Keep build, release, check, run/app, package install/uninstall, and vendoring tasks as appropriate.  Remove service start/stop/status/log/introspection, policy installation, socket setup, and service-oriented default workflow steps.  `just` should build and launch the app directly.

Update `README.md` to:

- state that UDisks2 is the only required system daemon;
- remove all instructions to install policies, start a root service, or run service-oriented recipes;
- describe the backend-contract boundary without promising unshipped backends, and identify the currently shipped `UdisksBackend` adapter;
- describe per-user rclone behavior accurately;
- keep the revised root resource paths in links.

Update `.github/workflows/main.yml` and `update-version.yml` so version checks, version replacement, publish commands, and changed-file lists cover root `Cargo.toml` and all five `crates/*` manifests. Package every crate with `cargo package --allow-dirty --no-verify` before publishing, then publish with locked manifest paths in dependency order: `storage-types`, `storage-contracts`, `storage-btrfs` and `cosmic-ext-storage-storage-sys`, `storage-udisks`, then `cosmic-ext-storage`. After each dependency layer, poll `cargo info <package>@<version> --registry crates-io` every 15 seconds for at most 20 attempts; publish the next layer only when every package in the preceding layer resolves at the expected version, otherwise fail with the unresolved package names. Update all internal workspace dependency versions together and verify actual package names before changing publish commands; the existing scripts contain stale `cosmic-ext-storage-udisks` path/name assumptions. Replace the stale VS Code configuration with one LLDB root-package launch entry.

**Gate:** Review the final justfile, install recipes, README, release workflow, version workflow, and editor configuration against canonical Task 8. The reviewed workflows package and publish the intended dependency order without service-era behavior.

### 8. Final verification and acceptance criteria

After the structural move, run the normal release build and the behavior-focused unit and adapter tests added or changed by this migration. Use the temporary local `RUSTC_WRAPPER` workaround from the canonical plan where necessary. Do not use duplicated aggregate recipes or bulk source-search scripts as proof of completion.

The final state is accepted only when all of the following are true:

- Cargo reports the root application plus exactly the five retained `crates/*` workspace libraries; it reports no service binary or service-only crate.
- `cargo build --workspace --release --locked` produces only the application binary (and intentional library/utility targets), never `cosmic-ext-storage-service`.
- Launching the application does not require `sudo`, a custom D-Bus policy, a project Polkit policy, a systemd unit, or a background project daemon.
- App startup does not launch a full filesystem usage scan; detailed scans remain explicit user actions and show in-process progress.
- Device events, post-mutation refreshes, image progress, and usage progress work without any project-owned D-Bus signal.
- Backend calls outside the composition root use `storage-contracts` and `storage-types`; no other app module imports `storage_udisks`, `disks_btrfs`, or `storage_sys::rclone`. `UdisksBackend` owns one system-bus connection, and mock non-UDisks block and second network adapters prove the trait API and registry routing.
- The workspace, install recipes, README, metainfo, CI/release automation, and resource links contain no stale `storage-app`, service, or old crate-directory paths.

Record the clean-desktop and focused test results in `docs/plans/1-service-removal/validation.md`.

**Gate:** Complete a final visual review of canonical Tasks 0–9 against the implemented diff and validation record. Resolve every unmet requirement before accepting the migration.

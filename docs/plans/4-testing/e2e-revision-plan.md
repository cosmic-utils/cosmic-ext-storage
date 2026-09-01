# Revision plan: replace the placeholder UI runner with real Rust E2E coverage

**Status:** R0–R3 are implemented. R3's application semantic-accessibility
gate was resolved on 2026-08-01 with a reviewed, temporary maintained
`libcosmic`/`iced` fork pinned on the dedicated `4-ui-testing` branch. The
capability run there proves real AT-SPI widget descendants and interactive
controls. R4's v2 case executor,
application automation IDs/editable-field support, reviewed accessibility/PNG
goldens, and R5 CI migration remain pending; they must not be represented as
implemented merely because the transport gate now passes. This document
supersedes the Phase-8 and Phase-9 *implementation status* recorded by the
current tree. It does not replace the broader safety, contract, or scenario
requirements in the other documents in this directory.

## 1. Why this revision is required

The checked-in E2E layer has the shape of a test system but not its execution
semantics:

| Area | Current behaviour | Required behaviour |
| --- | --- | --- |
| `tools/ui-testing/run.py` | Parses TOML and writes a synthetic JSON object. | Runs the compiled application and drives it through its accessibility API. |
| `step` entries | Validated, then ignored. | Performed in declared order and followed by declared assertions. |
| `goldens/*.json` | Restate a case selector and a fixture file hash. | Are reviewed AT-SPI-tree and PNG baselines produced by the application. |
| `just ui-e2e` | Started Weston, but started no application client. | Runs a Rust-owned, headless Sway capability gate that starts one scenario-mode app before case execution can be enabled. |
| Most scenario files | Contain only `name` and `backend`. | Contain the complete, typed state and scripted transition needed by their case. |
| Python/`pyatspi` | Python is the runner; `pyatspi` is installed but unused. | There is no Python execution dependency; a Rust test-only package owns the runner. |

A successful legacy run established only that a container could start a private
D-Bus session and a headless Weston process, and that checked-in metadata was
self-consistent. It does **not** establish that the application started,
rendered, exposed accessible controls, or performed an operation. The normal
Weston `caught signal 15` shutdown was expected cleanup, but it had to occur only
after an application client and all cases have completed.

This revision changes that fact. No existing JSON metadata golden may be
described as a visual golden after this work. It is deleted rather than
migrated.

## 2. Definition of done

For every declared case, a green `just ui-e2e` must prove all of the following
in one isolated container run:

1. The Rust runner starts exactly one `cosmic-ext-storage` child built with
   `test-backend`, passing `--backend scenario --scenario <artifact copy>`.
2. The child identifies the selected scenario in a test-only accessible status
   node whose scenario ID and fixture SHA-256 exactly match the copied
   fixture. A production build has no such node or control endpoint.
3. The runner uses AT-SPI to find each declared target, executes every
   declared action, and checks every declared assertion. It never interprets a
   coordinate as a target selector.
4. Each checkpoint has an actual PNG, a reviewed expected PNG, a pixel diff
   (when unequal), an actual canonical AT-SPI subtree, and a reviewed expected
   canonical AT-SPI subtree.
5. The scenario trace proves the configured state transition (or configured
   typed failure) took place. It contains no passphrase, descriptor secret, or
   test-control token.
6. The runner terminates the app before Sway, records both exit states, and
   treats expected compositor termination as normal teardown rather than test
   evidence.

The E2E job is passing only if every item is true. A runner that cannot connect
to AT-SPI, cannot capture a PNG, finds zero or multiple matching controls, or
observes no selected test scenario fails; it must never quietly write a
synthetic replacement artifact.

## 3. Fixed design decisions

### 3.1 Package and dependency boundary

Create the non-published workspace package `crates/ui-e2e-runner`:

```toml
[package]
name = "ui-e2e-runner"
publish = false

[[bin]]
name = "ui-e2e-runner"
path = "src/main.rs"
```

It is a test tool, not an application dependency. The release package must not
depend on it, expose its command-line flags, or link it. It owns:

- TOML case/lock parsing and strict validation;
- child-process lifecycle and artifact layout;
- an AT-SPI client built on `atspi` with the `connection` and `proxies`
  features;
- canonical accessibility-tree snapshots, semantic actions and assertions;
- Wayland virtual-keyboard input for the one keyboard-navigation case;
- screenshot adapter invocation and deterministic pixel comparison.

Add `atspi = "=0.30.0"` directly to this package. The crate provides an
`AccessibilityConnection`, role/state values, events, and action types; its
proxy module exposes the Accessible, Action, Component, EditableText, and
related AT-SPI interfaces. Use that client API, not raw ad-hoc D-Bus method
strings. The workspace already uses `zbus`; any direct `zbus` use in this
package must use the workspace-pinned version.

The runner may additionally depend on pinned Rust crates for TOML/Serde,
SHA-256, PNG decoding, process supervision, and generated Wayland protocol
bindings. All dependencies are lockfile-pinned through `Cargo.lock`.

Delete, in the same commit that switches the recipe, these Python-specific
paths and dependencies:

- `tools/ui-testing/run.py`;
- `tools/ui-testing/tests/`;
- `python3-pyatspi` from the Containerfile and environment lock;
- the `python-unittest` target from `tests/ui/required-tests.toml`.

The remaining shell in `justfile` may establish a container/private D-Bus
session, but no test semantics may live in shell or Python.

### 3.2 Process and session ownership

`just ui-e2e` must have this ownership chain. Each child is waited for and all
PID/exit details are written to `ui-artifacts/processes.json`.

```text
docker run --network none --tmpfs /tmp …
  dbus-run-session -- ui-e2e-runner run --all …
    ├── sway --unsupported-gpu --config /workspace/tools/ui-testing/sway.conf
    ├── cosmic-ext-storage --backend scenario --scenario <case copy>
    └── locked screenshot helper (one invocation per checkpoint)
```

The runner, not the recipe, starts Sway and the application. Before either
child starts, it creates these empty mode-`0700` directories, refuses symbolic
links, and exports only these paths to its children:

```text
$ARTIFACT_ROOT/runtime
$ARTIFACT_ROOT/home
$ARTIFACT_ROOT/config
$ARTIFACT_ROOT/cache
```

It sets `XDG_RUNTIME_DIR` to `runtime`, `WAYLAND_DISPLAY=ui-test`, `HOME`,
`XDG_CONFIG_HOME`, and `XDG_CACHE_HOME` to the corresponding directories, and
uses the locale, timezone, renderer, theme, viewport, and font settings from
the checked-in environment lock. The container receives neither a host D-Bus
socket nor a host runtime directory.

The runner starts Sway with the exact `sway_command` array from the lock,
waits for `$XDG_RUNTIME_DIR/ui-test` and the one Sway IPC socket to be Unix
sockets, verifies the declared output dimensions, and then opens an
`atspi::AccessibilityConnection`. Failure to create the accessibility
connection is a hard `atspi_connect` failure. The lock must add an exact,
version-pinned `at-spi2-core` package; service activation occurs within the
private session D-Bus created by `dbus-run-session`.

The application command is constructed from explicit runner arguments, never
from a case field. It is exactly:

```text
<app-path> --backend scenario --scenario <case-artifact>/scenario.toml \
  --scenario-control-socket <runtime>/scenario-control.sock \
  --scenario-control-token-file <runtime>/scenario-control.token
```

The latter two arguments are accepted only with `test-backend`. The token is
32 random bytes written mode `0600`; its value is never logged or copied into
an artifact. The scenario control socket is mode `0600`, owns no storage
capability, and has no production implementation.

The runner has one real-time watchdog solely to detect dead processes:
15 seconds for readiness and 30 seconds for a complete case. It does not use
wall time to decide application state or simulate progress. On failure it
writes the last tree/trace/logs, sends SIGTERM to the app, waits two seconds,
sends SIGKILL only if needed, then terminates Sway. A clean app exit before
the final checkpoint is always a failure.

### 3.3 Scenario-control wire protocol

`ScenarioControl` must be reachable from the runner without constructing a
second backend. Add a test-only server owned by the selected `AppRuntime`.
It accepts exactly one authenticated request stream over the private Unix
socket. Frames are a four-byte big-endian length followed by UTF-8 JSON:

```json
{"protocol":1,"sequence":7,"token":"<redacted>","command":{"kind":"advance_clock","milliseconds":250}}
```

The complete command set is closed:

```text
advance_clock { milliseconds: u64 }
snapshot {}
replace_overlay { sha256: lowercase-hex-64, bytes_base64: string }
reload {}
shutdown {}
```

Responses echo `protocol` and `sequence`, contain either `{ "ok": … }` or a
typed `StorageError`, and are written only after the actor has serialized the
command. `advance_clock` advances the virtual clock exactly once; it never
sleeps. `replace_overlay` first validates the closed scenario schema, atomically
writes a new overlay in the case artifact directory, and leaves the previous
overlay active if validation or rename fails. `reload` applies precisely that
validated overlay and emits its one declared refresh event. `snapshot` returns
the redacted, canonical state and event sequence. No command reads an arbitrary
host path.

Every request and response produces a redacted trace record containing its
sequence, virtual timestamp, command kind, result kind, state revision, and
affected operation ID. The runner checks monotonic sequence numbers and
attaches the final trace to every case artifact.

## 4. Versioned data contracts

### 4.1 Cases become executable case manifests

Move cases to `schema_version = 2`. Version 1 manifests and all existing JSON
goldens are invalid after the migration. A case is an ordered program with no
implicit action, wait, or assertion.

```toml
schema_version = 2
id = "physical_partition_format" # lower_snake_case, unique
scenario = "tests/ui/scenarios/physical/partition-format.toml"
scenario_sha256 = "<64 lowercase hex>"

[ready]
timeout_ms = 15000
[[ready.assert]]
kind = "unique"
selector = { role = "application", name = "COSMIC Storage" }
[[ready.assert]]
kind = "unique"
selector = { role = "status", automation_id = "test.scenario" }
[[ready.assert]]
kind = "property_equals"
selector = { role = "status", automation_id = "test.scenario" }
property = "name"
value = "Test scenario: physical-partition-format sha256:<fixture hash>"

[[step]]
id = "open_create_partition"
kind = "invoke"
target = { role = "button", automation_id = "partition.create" }

[[step]]
id = "assert_dialog"
kind = "assert"
assert = { kind = "unique", selector = { role = "dialog", automation_id = "partition.create.dialog" } }

[[checkpoint]]
id = "create_partition_dialog"
after = "assert_dialog"
target = { role = "dialog", automation_id = "partition.create.dialog" }
tree_golden = "tests/ui/goldens/physical_partition_format/create_partition_dialog.a11y.json"
image_golden = "tests/ui/goldens/physical_partition_format/create_partition_dialog.png"
```

The only selector keys are `role`, `name`, `automation_id`, `states`, and
`ancestor`. `name` is an exact UTF-8 value; regular expressions, indexes,
object paths, and coordinates are forbidden. A selector must contain `role`
plus either `name` or `automation_id`; `states` and `ancestor` only narrow an
existing selector. Every lookup is required to produce exactly one live node,
unless the assertion kind explicitly says `count` with an exact integer.

`automation_id` is an application-defined accessibility attribute, not a test
runner convention. Add a small UI helper that attaches a stable ID to the
AccessKit/AT-SPI node for each control named below. IDs are ASCII lowercase
dot-separated names and never include a translated label, a device path, or a
random operation ID. Test cases may use translated visible names only when
they are deliberately testing accessibility names.

The closed step kinds are:

| Kind | Required fields | Semantics |
| --- | --- | --- |
| `invoke` | `target`, optional exact `action` | Call the named AT-SPI action, defaulting only to the sole `click`/`activate` action. Fail if ambiguous. |
| `set_text` | `target`, `value` | Require EditableText and replace text through its AT-SPI method. Value is redacted in artifacts if `secret = true`. |
| `key` | `key` | Send an exact virtual-keyboard press/release pair. Permitted keys are a fixed named-key enum plus printable Unicode scalar values. |
| `advance_clock` | `milliseconds` | Send exactly one scenario-control request. |
| `replace_overlay` | `fixture` | Send the complete checked-in overlay bytes and SHA-256 to the control server. |
| `reload` | none | Send one scenario-control `reload` request. |
| `assert` | `assert` | Evaluate one closed assertion. |

Assertions are `unique`, `absent`, `count`, `property_equals`, `state_equals`,
`text_equals`, `focused`, `trace_contains`, and `trace_excludes`. The runner
waits on an AT-SPI event relevant to the selector/action, then evaluates the
assertion from a fresh tree. The readiness/case watchdog is only a ceiling;
there is no fixed sleep or retry-count success path.

### 4.2 Accessibility-tree golden format

For each checkpoint, serialize the selected node and all exposed descendants
to canonical JSON:

```json
{
  "schema_version": 1,
  "root": {
    "automation_id": "partition.create.dialog",
    "role": "dialog",
    "name": "Create partition",
    "description": "",
    "states": ["enabled", "showing", "visible"],
    "attributes": {"automation_id": "partition.create.dialog"},
    "text": null,
    "children": []
  }
}
```

Objects use the field order shown; maps are key-sorted; state arrays are
sorted by their lower-case wire name; missing optional values use `null`.
Exclude AT-SPI bus paths, process IDs, timestamps, screen bounds, toolkit
versions, and every other volatile attribute. The serializer rejects unknown
or non-allowlisted attributes instead of silently recording them. This tree
golden tests semantic content independently of pixels.

### 4.3 Pixel golden format and comparison

Each checkpoint capture is a crop derived from the target node's AT-SPI
component bounds. Given logical bounds `(x, y, width, height)`, viewport scale
`s`, and locked margin `8` logical pixels, its pixel rectangle is:

```text
left   = floor((x - 8) * s)
top    = floor((y - 8) * s)
right  = ceil((x + width + 8) * s)
bottom = ceil((y + height + 8) * s)
```

Clamp to the single locked viewport. The runner records both logical and pixel
rectangles in the checkpoint artifact. It captures the full Sway output,
then crops with these exact rules. No case supplies a crop or a coordinate.

The image lock is changed from a fractional value to this integer contract:

```toml
image_comparison = {
  colorspace = "sRGB RGBA8",
  max_channel_delta = 2,
  max_changed_pixels = 512,
  masks = false,
  edge_rounding = "floor_start_ceil_end",
  crop_margin_logical_px = 8
}
```

Expected and actual images must have identical dimensions and decode as RGBA8
sRGB PNG. A pixel is changed when any channel's absolute delta exceeds `2`.
The comparison fails if more than `512` pixels changed. On failure, write a
lossless RGBA diff with unchanged pixels transparent and changed pixels opaque
red, plus `comparison.json` containing dimensions, count, threshold, and crop
rectangle. The normal successful artifact also contains `comparison.json`.
There are no masks, baseline auto-acceptance, or OS-dependent tolerance.

The locked screenshot helper is `grim` from the exact Debian package installed
in the image. Phase 3 must prove its capture command against the selected Sway
output before any goldens are created. The runner invokes that helper as a
child, validates the output as PNG, and never substitutes an empty image. If
the helper lacks the required headless-output capture support, the phase is
blocked: do not replace visual testing with JSON metadata. In that event, pin
and package a small Rust Wayland capture helper that speaks Sway's locked
`zwlr_screencopy_manager_v1` protocol, record its source revision and SHA-256
in the environment lock, and retain the same PNG contract.

### 4.4 Scenario schema v2

The current v1 parser deliberately rejects unknown fields, so full fixtures
must be a declared v2 migration, not an untracked extension. `schema_version =
2` is the only accepted scenario version after the migration; the loader gives
a precise migration error for v1. The normative schema descriptor and JSON
schema must be updated in the same commit.

Every v2 scenario has these top-level tables; unspecified capability is not
inferred and a reference to it is an invalid cross-reference:

```toml
schema_version = 2
id = "physical-partition-format"                 # kebab-case, unique
description = "…"                                # required, non-empty
[clock]
start_ms = 0
[backend]
id = "ui-scenario"
capabilities = ["partition", "filesystem"]     # sorted, no duplicates
[world]
revision = 0
[[world.disks]]
id = "disk0"
device = "/dev/ui-disk0"
model = "Scenario Disk"
size_bytes = 1073741824
partition_table = "gpt"                          # or "none"
[[world.partitions]]
id = "disk0p1"
disk_id = "disk0"
device = "/dev/ui-disk0p1"
start_bytes = 1048576
size_bytes = 536870912
kind = "primary"
[[world.filesystems]]
id = "fs0"
device = "/dev/ui-disk0p1"
type = "ext4"
label = "Data"
mount_points = ["/mnt/ui-data"]
```

The additional closed tables are `world.luks`, `world.btrfs`,
`world.logical`, `world.network`, `world.usage`, `world.images`, and
`world.processes`. Each record has a scenario-local string ID; all links use
those IDs rather than host paths. Device and mount fields are accepted only in
the synthetic `/dev/ui-*` and `/mnt/ui-*` namespaces. Lists have a specified
sort key and load in that order. `world.processes` models blockers; it never
starts a process.

Behaviour is a complete ordered mapping, not a loose error hook:

```toml
[[behaviour.rules]]
operation = "filesystem.unmount"
selector = { filesystem_id = "fs0" }
outcome = { tag = "error", kind = "busy", message = "Fixture user is active" }
events = []
```

Each rule has a unique `(operation, selector)` pair. Its outcome is exactly
one of `success`, `error`, `unsupported`, or `delayed`. A delayed outcome
declares nondecreasing virtual offsets, explicit progress/status values, a
terminal result, and declared events. A modelled operation with no matching
rule follows the operation inventory's explicit default; it does not succeed
accidentally. All state mutations record `before_revision`, `after_revision`,
operation ID, and the declared event sequence.

The parser validates schema version placement, all IDs/references, alignment
and non-overlap of partitions, mount uniqueness, required capabilities,
operation-selector validity, rule uniqueness, monotonic progress, and absence
of host paths before it constructs `RuntimeAdapters`.

## 5. Required fixtures and executable evidence

No required fixture may be equivalent to the blank `empty.toml` fixture. The
following exact data and case effects are the minimum acceptance set. Add
further success/error branches when the traceability matrix requires them.

| Fixture / case | Required initial world | Mandatory executable proof |
| --- | --- | --- |
| `physical/partition-format` / `physical_partition_format` | One 1 GiB GPT disk with no partitions and `partition`, `filesystem` capabilities. | Invoke `partition.create`; set start `1048576`, size `536870912`, label `Data`, type `ext4`; confirm; assert one `/dev/ui-disk0p1` ext4 filesystem, revision `1`, and one ordered refresh event. Capture dialog and resulting partition row. |
| `physical/busy-unmount` / `busy_unmount` | One mounted ext4 filesystem at `/mnt/ui-data`, one modelled blocking process, an exact busy error rule for its unmount selector. | Invoke `filesystem.unmount`; assert actionable busy dialog names the fixture process; assert mount and revision unchanged; capture error dialog. |
| `physical/luks` / `luks_unlock` | One LUKS2 container with a declared test passphrase secret, locked mapping, and an ext4 child. | Submit an incorrect passphrase and assert typed authentication error without state change; submit the redacted correct value and assert unlocked mapping/mount action; capture unlock dialog and unlocked row. |
| `logical/preflight` / `logical_preflight_confirmation` | Declared logical source, candidate disks, destructive preflight summary and confirmation key. | Open review; assert summary; mutate virtual revision through declared candidate change; prove stale key is rejected; re-preflight and confirm; assert one logical entity and one refresh event. |
| `network/mount` / `network_mount` | One network backend with a closed configuration schema and no config. | Fill every required field, create and test config, mount it, assert `mounted`; unmount and assert `unmounted`; capture the named configuration form and mounted row. |
| `workflows/image-usage` / `image_usage_progress` | One declared image asset, one copy workflow with progress at virtual `0`, `250`, `500`, `750`, `1000` ms, and one usage scan/delete fixture. | Start copy; advance clock exactly to `500`; assert `50%`; cancel; advance to `1000`; assert terminal `cancelled`, never `completed`; run scan/delete and assert in-memory deletion result. Capture progress and deletion summary. |
| `accessibility/keyboard` / `keyboard_accessibility` | A dialog-opening workflow with named controls, disabled reason, and default/cancel actions. | Use only virtual keys: Tab sequence, Shift+Tab, text input, Enter/Escape. After each focus movement assert one focused AT-SPI node and a valid visible focus indicator in its capture. |
| `reload/live` / `live_scenario_reload` | One disk label `Before`, a complete valid overlay changing it to `After`, and a separate invalid overlay fixture. | Submit invalid overlay and assert revision/label unchanged; submit valid overlay then reload; assert one refresh event, revision increments once, and visible label is `After`; capture both states. |

The image/usage fixture must use only declared data bytes and logical file IDs;
the runner's artifact directory is the only writable real filesystem area. The
LUKS passphrase field is written as a secret fixture value only in a
non-committed test secret source generated during case setup. Its plaintext is
not permitted in TOML, source, runner output, traces, goldens, JUnit, or CI
logs. The case refers to it with `value_from = "test_secret:luks0"`.

Assign and test a stable `automation_id` for every control referenced above.
The traceability matrix must list the case step ID, operation ID, expected
transition, and each checkpoint; claiming a row without a matching semantic
assertion and checkpoint is invalid.

## 6. Application and backend work required before E2E

1. Complete the existing contract migration rather than preserving a thin
   placeholder adapter. `ScenarioBackend` must implement every modelled
   physical, LUKS, Btrfs/logical, network, usage, image, and desktop interface
   as declared in the scenario inventory. Unsupported remains a typed result
   only for an operation explicitly marked unsupported in that inventory.
2. Remove the remaining global `SHARED_OPERATIONS` selection. Every UI model,
   update task, client, and subscription must receive the `AppRuntime` chosen
   at launch. A scenario process must be unable to construct a production
   UDisks/local-tools/rclone adapter.
3. Make the scenario control server a member of the selected scenario runtime;
   it is unavailable in the production factory and unavailable when the
   `test-backend` feature is absent. Test the command-line rejection before
   any production factory call.
4. Add the test-only scenario status node and test-only automation-ID helper.
   Both compile only in scenario mode. Production snapshots and package help
   must prove their absence.
5. Implement state transitions as a serial actor with a monotonic revision and
   virtual millisecond clock. Effects due at the same clock value are ordered
   by `(due_ms, operation_sequence, effect_sequence)`. Cancellation wins over
   a completion scheduled in the same tick if its actor command sequence is
   lower; this rule is unit-tested.
6. Emit a deterministic trace for every query, mutation, virtual effect,
   reload, and control command. Use typed redaction at the domain boundary,
   not a post-hoc string filter.

## 7. Runner implementation sequence

Do these phases in order; do not create PNG goldens before Phase 4 is green.

### Phase R0 — record and delete false evidence

- Add this revision plan and link it from `README.md` and
  `implementation-plan.md`.
- Mark Phase 8/9 as not completed in `phase-record.md`; record the current
  synthetic-runner evidence and its limitations.
- Add a failing Rust integration test proving `ui-e2e` must launch the app and
  produce an AT-SPI connection/capture artifact. It is expected to fail until
  Phase R4.
- Delete `__pycache__` artifacts if present; they are never committed.

### Phase R1 — complete scenario model and fixtures

- Add v2 DTOs, strict loader validation, canonical serialization, and the
  scenario-control protocol tests to `test-backend`.
- Implement the exact fixture data in section 5, including valid/invalid reload
  overlays and generated in-memory secret material.
- Implement all stated transitions, state revisions, events, and traces.
- Migrate `tests/ui/traceability.toml` so every step/checkpoint has an exact
  corresponding row. Remove `empty.toml` unless a dedicated blank-state case
  tests it.

**Gate:** every v2 fixture validates; each required transition has a success,
configured-error where applicable, trace, and state-revision test; no fixture
contains a host path or a plaintext secret.

### Phase R2 — finish runtime injection and test control

- Remove globals/direct host paths from scenario-facing UI code.
- Add scenario-only marker, automation IDs, and private control endpoint.
- Add unit/integration tests for production absence, authentication, framing,
sequence ordering, virtual time, atomic overlay replacement, reload failure,
and redaction.

**Gate:** a scenario root integration test drives the real `AppRuntime` through
the control endpoint and sees a modelled state change without a desktop server;
a no-feature release build rejects all scenario/control flags before creating
production adapters.

### Phase R3 — build the Rust runner and prove environment capability

- Create `ui-e2e-runner` with strict v2 case/lock parsers and unit tests.
- Implement process lifecycle, AT-SPI connection, selector/tree traversal,
canonical serialization, semantic invoke/text actions, trace client, and
artifact writer.
- Implement virtual-keyboard input and prove the locked Sway environment exposes the
required protocol. It must fail the environment preflight with the global name
and version if it is absent.
- Replace `python3-pyatspi` with exact `at-spi2-core` and any exact locked
capture/input helper package required by the selected design. Update the
Containerfile hash, package/font lock, and container preflight.
- Prove `grim` can capture the locked headless output and
produce one non-empty RGBA PNG. Record its executable path/version in the
environment lock.

**Gate:** a dedicated test app renders a named, accessible test window in the
same container; the runner finds it through AT-SPI, invokes its button, sends a
Tab key, captures a non-empty crop, and compares it to a committed temporary
baseline. This is a runner capability test, separate from storage UI cases.

### Phase R4 — migrate cases and create first reviewed baselines

- Implement all v2 case manifests using only the step/assertion language in
  section 4.1.
- Replace JSON selector snapshots with `.a11y.json` and `.png` baselines in the
  paths declared by the manifest. Generate them only with
  `just ui-e2e-update case=<id>` in the locked container.
- The update command rejects `CI=true`, a dirty environment lock, an
  unrecognised case, a missing app process, a failed assertion, a missing
  trace, or an output path outside `tests/ui/goldens/`.
- Review a golden update as image/tree/trace diff alongside the associated UI
  change. It does not accept only a hash change.

**Gate:** every case fails when one expected accessible name, state, operation
transition, or expected PNG pixel is deliberately changed.

### Phase R5 — CI and removal of the scaffold

- Change `just ui-e2e` to build the test-feature app and Rust runner, then run
  the runner inside the locked container. It passes no host source of truth
  other than the workspace mounted read/write only for `ui-artifacts/`.
- `just ui-e2e-update` runs the identical container command with explicit
  `--update-goldens`, never a host-native runner.
- Replace Python test target declarations with Rust runner unit/integration
  targets; update `assert_tests.py` only as a manifest checker, or migrate it
  to a Rust checker in a separate change. It must not execute E2E semantics.
- Upload `ui-artifacts/` with `if: always()` for the required CI job. Retain
  artifacts long enough for PR diagnosis according to repository policy.
- Add a CI negative check: release `--help`, release dependency graph, and
  production launch arguments contain no scenario/control/test-runner symbols.

**Gate:** CI runs the capability test and all eight required cases; pulling the
application spawn from the runner, omitting the AT-SPI client, or replacing a
PNG baseline with JSON makes the gate fail.

## 8. Artifact contract

For case `<id>`, `ui-artifacts/<id>/` must contain only this stable structure:

```text
case.toml                         # copied v2 manifest
scenario.toml                     # copied source fixture
overlay.toml                      # only if this case uses an overlay
environment.lock.toml
processes.json
application.stdout.log
application.stderr.log
sway.stderr.log
trace.json                        # canonical, redacted
step-001.before.a11y.json
step-001.after.a11y.json
checkpoints/<checkpoint>/actual.a11y.json
checkpoints/<checkpoint>/expected.a11y.json
checkpoints/<checkpoint>/actual.png
checkpoints/<checkpoint>/expected.png
checkpoints/<checkpoint>/comparison.json
checkpoints/<checkpoint>/diff.png # required only on comparison failure
result.json
```

`result.json` has `schema_version = 1`, exact case ID, fixture SHA-256,
runner/app/Sway versions, final virtual time, ordered step results, and one
of `passed`, `failed`, `infrastructure_failed`. The top-level manifest contains
the same per-case status and preserves case order from lexical case ID. No
secret/token/plaintext appears in any artifact; the redaction test scans the
whole artifact directory bytes for every generated secret value.

## 9. Exact validation inventory

Update `tests/ui/required-tests.toml` to name real targets. At minimum add:

| Target | Required tests |
| --- | --- |
| `test-backend/schema_v2` | `v1_fixtures_require_explicit_migration`; `rejects_invalid_v2_cross_references`; `canonical_v2_fixture_hash_is_stable`; `rejects_plaintext_secret_values` |
| `test-backend/control_protocol` | `control_server_authenticates_and_serializes_requests`; `same_tick_cancel_order_is_deterministic`; `invalid_overlay_never_changes_active_state`; `reload_emits_exactly_one_refresh` |
| `ui-e2e-runner/case_manifest` | `rejects_coordinate_and_index_selectors`; `requires_exactly_one_match_for_actions`; `case_requires_trace_and_png_checkpoint`; `v1_case_is_rejected` |
| `ui-e2e-runner/a11y` | `canonical_tree_excludes_volatile_fields`; `invoke_requires_unambiguous_action`; `secret_text_is_redacted`; `ready_requires_matching_scenario_marker` |
| `ui-e2e-runner/image` | `crop_rounding_is_locked`; `rgba_comparison_uses_fixed_threshold`; `dimension_mismatch_fails`; `diff_has_transparent_unchanged_pixels` |
| `ui-e2e-runner/lifecycle` | `app_is_spawned_before_ready_wait`; `early_app_exit_fails`; `atspi_failure_never_writes_synthetic_success`; `teardown_stops_app_before_sway` |
| runner capability E2E | `atspi_action_keyboard_and_capture_work_in_locked_sway` |
| storage UI E2E | the eight named cases in section 5, expanded with separate checkpoints as declared. |

The full validation sequence is:

```sh
just ui-plan-check
just ui-assert-tests phase=all
cargo test -p test-backend --locked
cargo test -p ui-e2e-runner --locked
cargo test -p cosmic-ext-storage --locked --features test-backend --test ui_runtime_contract
cargo test -p cosmic-ext-storage --locked --test scenario_feature_disabled_contract
just ui-scenario-check
just ui-e2e
just package-check
cargo test --workspace --all-features --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

Before accepting the revision, inspect a successful artifact and a deliberately
failed image/tree/action artifact. The success must include non-empty app and
Sway logs, real AT-SPI object data, and non-empty PNGs; the failure must
include a diagnostic diff and preserve the app trace. Finally run
`rg -n 'python3-pyatspi|pyatspi|tools/ui-testing/run.py' .` and require no
code, lock, CI, or Justfile result.

## 10. Review rules and non-goals

- This remains a scenario test system: it must not access UDisks, host disks,
  host mounts, rclone configuration, a host browser, or the system D-Bus.
- It complements, never replaces, `storage-testing` and the disposable
  real-backend harness.
- Do not reintroduce a test-only coordinate click, sleep-based wait, mock
  accessibility tree, semantic-only “visual” baseline, or implicit golden
  update as a shortcut around a failing case.
- A broad command that does not select the named capability/case target is not
  evidence. `ui-assert-tests` must verify names before each target executes.
- Any change to the environment lock, capture helper, fonts, renderer,
  selector schema, image threshold, or fixture schema is a reviewed E2E
  contract change and must update tests, baseline evidence, and this plan in
  the same pull request.

## 11. R3 compositor capability determination — 2026-08-01

### Historical Weston result

**Result: blocked. Do not implement the Rust E2E runner against the former
Weston configuration.**

The probes were executed in the digest-pinned image from
`tools/ui-testing/Containerfile`, with a private `dbus-run-session`, an
`XDG_RUNTIME_DIR` mode `0700`, and the v1 lock's headless Weston command. The
following observations are inputs to the next reviewed environment decision:

| Capability | Probe result | Determination |
| --- | --- | --- |
| AT-SPI session bus | `/usr/libexec/at-spi-bus-launcher --launch-immediately` starts; a client can obtain the registry desktop. | Available. The eventual Rust `atspi` client can use this private session. |
| Native Weston capture | `weston --debug` advertises `weston_screenshooter` version 1. `weston-screenshooter` produced no PNG and timed out after 3 seconds with an empty output and after 5 seconds with a `weston-flower` client present. | Unavailable. It is not an acceptable screenshot adapter. |
| Virtual keyboard | `weston-info` listed the advertised globals. It did not list `zwp_virtual_keyboard_manager_v1`. | Unavailable. A Wayland virtual-keyboard Rust client cannot drive the keyboard-only case. |
| Locked viewport | The headless output reported `1024x640`, while `environment.lock.toml` claims `1280x800`. | Invalid lock. Visual crop coordinates would be non-deterministic. |
| Application launch in image | A host-built binary first lacked `libbtrfsutil.so.1`; after temporarily installing that exact runtime package it required `GLIBC_2.39`/`GLIBC_2.43`, newer than the Debian Bookworm image's glibc. | Invalid execution image. The test app must be built inside the locked image or the runtime image must exactly match the build ABI. |

The prior `just ui-e2e` result remains **scaffold evidence only** because it
did not launch the app. It must not be used to waive any item above.

### Pinned Sway replacement result

The Weston decision was implemented with the following locked replacement:

- Sway `1.7-6`, `WLR_BACKENDS=headless`, `WLR_RENDERER=pixman`, one locked
  `1280x800` scale-1 output, and no host runtime or D-Bus socket.
- `grim 1.4.0+ds-2` for full-output capture through
  `zwlr_screencopy_manager_v1`.
- `wtype 0.4-3` for Wayland virtual-keyboard delivery through
  `zwp_virtual_keyboard_manager_v1`.
- `at-spi2-core 2.46.0-5` in a private `dbus-run-session`.
- A scenario-mode application binary compiled inside the same pinned Rust
  image, so its glibc and project-native dependencies match the runtime.

`just ui-e2e` now runs the Rust-owned `ui-e2e-runner capability` command. It
first validates the environment lock against the copied Containerfile, installed
package versions, and font hash. It then starts Sway, discovers its actual
Wayland and IPC sockets, verifies the output, starts one
`cosmic-ext-storage --backend scenario` client, enables AT-SPI's
`ScreenReaderEnabled` status for the running application, confirms the app's
AT-SPI root, captures `application.png`, and uses `wtype` to change a visible
`foot` input-probe client. It writes
`ui-artifacts/capability/capability.json` plus `atspi.json`, app/Sway logs,
output/tree JSON, and PNG artifacts.

The recorded successful run proved:

1. `HEADLESS-1` was the sole active `1280x800` scale-1 Sway output.
2. Rust connected to the private AT-SPI registry; its sole application root
   was named `cosmic-ext-storage`.
3. The real application appeared in Sway's tree and yielded a non-empty,
   signature-valid `application.png` (24,244 bytes in the recorded run).
4. The input-probe client appeared in Sway's tree; its before/after captures
   had distinct SHA-256 values after `wtype ui-e2e-keyboard-probe`.

This resolves the Weston compositor, capture, viewport, ABI, and application
AT-SPI activation blockers. It does **not** yet make the eight legacy v1
manifests real E2E tests or make the probe-client keyboard check an application
semantic assertion. The next bounded increment is the v2 case executor in
sections 3–8: expose the selected scenario marker/control nodes through AT-SPI,
drive each declared app target, and create reviewed tree/PNG goldens. PNG golden
updates remain deliberately disabled until that executor exists.

### Semantic application accessibility determination

**Result: blocked. Do not create case manifests, semantic actions, or goldens
against the current application tree.**

The capability probe now writes a complete flat AT-SPI snapshot to
`ui-artifacts/capability/atspi-tree.json`, immediately after enabling the
private session's `ScreenReaderEnabled` status and before opening the keyboard
probe client. The successful 2026-08-01 run has exactly these application
nodes:

```text
depth 0  role application  name cosmic-ext-storage
depth 1  role frame        name ""
```

There is no descendant control, status node, text field, dialog, component
bound, accessible action, or stable automation ID, despite the rendered
application containing visible controls. `capability.json` records both the
total tree node count and the count below the window node, so this observation
is reproducible rather than inferred from a screenshot.

This is not a container, Sway, D-Bus, or runner-client failure:

1. The application is compiled with libcosmic's `a11y` feature and registers
   an AccessKit application/frame with the private AT-SPI registry.
2. The pinned libcosmic revision is
   `ef162b8e16ba4493e05c169cd56c7b9f77f0fda5`. Its
   `iced/winit/src/a11y.rs` creates the initial AccessKit tree containing only
   the window node.
3. Its `iced/winit/src/lib.rs` stores `a11y_enabled` after activation but never
   reads it, never calls `Adapter::update_if_active`, and never forwards
   `UserInterface::a11y_nodes(...)` to AccessKit. The widget implementations
   do construct `A11yTree` values, but those values never reach AT-SPI.
4. Upstream libcosmic master was checked at
   `dc1cf9f00cbe2902a52166492654bb9fee8a73d1` (2026-07-29, with iced
   `7346cffd2e51e45fbe4dd31bdd42211b8ca0078e`) and has the same missing
   forwarding path.

The deterministic unblocking change belongs in libcosmic/its iced submodule:

1. On an accessibility-activation transition, request a redraw for each open
   window.
2. During every completed redraw while accessibility is active, obtain
   `interface.a11y_nodes(cursor)` and call the corresponding
   `accesskit_winit::Adapter::update_if_active`.
3. The submitted `TreeUpdate` must preserve the adapter's window root, attach
   every `A11yTree::root()` node as its children, include all root and child
   nodes, and preserve a valid focused node. It must be updated after a UI
   state mutation as well as after initial activation.
4. Add an upstream Linux AT-SPI integration test that starts a named window,
   exposes a named button and editable field, invokes the button through
   AccessKit/AT-SPI, and verifies a widget-state update. A tree containing
   only an application/frame must fail that test.
5. After that test is merged, update this repository to the exact reviewed
   libcosmic commit and lockfile, re-run `just ui-e2e`, and require at least one
   interactive descendant before enabling R4. The application can then add its
   scenario-only marker and stable automation IDs, and the runner can perform
   the v2 selector/action/golden work described above.

A local libcosmic fork would be a material new dependency-maintenance policy;
it is deliberately not silently vendored or patched by the E2E runner. The
runner must not substitute Sway tree entries, pixel coordinates, synthetic
AT-SPI objects, or keyboard focus order for the absent application semantic
tree.

### Semantic application accessibility resolution

**Result: resolved for R3; R4 is unblocked but not yet implemented.** The
preceding determination records the failure at the original pinned upstream
revision. It is superseded by the following repeatable result.

The dedicated `4-ui-testing` branch pins `stoorps/libcosmic` commit
`3d5fdb087534fb4944e64da2346f3bb673cc0215`, based on the requested upstream
base `ef162b8e16ba4493e05c169cd56c7b9f77f0fda5`. Its `iced` submodule points to
`stoorps/iced` commit `25c211d8b1c0f456fd327b65be5261311b1d7692`. The branch is
`fix/accesskit-widget-tree` in each fork. The maintained delta does exactly
four things:

1. Emits each completed `UserInterface::a11y_nodes(cursor)` tree through the
   active window's AccessKit adapter, attaching its roots below that adapter's
   existing Window node.
2. Reuses the adapter's Window node ID for the initial AccessKit tree, rather
   than allocating a second ID that makes subsequent updates invalid.
3. Wakes Winit after activation, action, and deactivation handler requests, so
   the queued control message is processed even when the otherwise-idle UI has
   no native event to wake it.
4. For explicit custom widget IDs, preserves the author ID in the AccessKit
   node so it appears as AT-SPI's stable `accessible_id`.

The fork includes unit coverage for both root-ID preservation and widget-root
attachment. `just ui-e2e` was then run against the exact lockfile inside the
digest-pinned image. Its AT-SPI snapshot contains 22 nodes: the application,
the frame, and 20 descendants below the frame, including multiple buttons,
images, paragraphs, a panel, and a scrollbar. It has exactly one accessible
`test.scenario` paragraph whose name contains the selected scenario's ID and
SHA-256. The runner now hard-fails the capability command when that marker is
missing or the selected application has no interactive AT-SPI descendant,
preventing a regression to a window-only tree from being recorded as success.

This resolution does **not** make the legacy v1 manifests valid E2E tests.
The next R4 implementation must still:

1. Assign the now-supported stable application automation IDs and names to
   every declared case control. The scenario-only `test.scenario` status node
   is already present and verified in the capability gate.
2. Complete accessibility support for editable text controls and their
   `Focus`/text actions in the maintained dependency, then use it from the
   app's named forms. Current Iced source builds no AT-SPI tree for
   `TextInput`, so the existing placeholder `set_text` cases cannot be made
   real without that work.
3. Replace the v1 case manifests with the closed v2 selector/action/assertion
   contract, use the private scenario-control endpoint for virtual time and
   reload, and create reviewed subtree/PNG baselines only after every semantic
   action and assertion passes.

Until those steps land, R4/R5 remain pending. The maintained dependency is a
deliberate temporary bridge; remove the root Cargo override after equivalent
upstream commits and integration coverage are accepted.

## 12. UI-branch handoff — 2026-08-01

The graphical implementation is intentionally isolated on the `4-ui-testing`
branch. That branch starts at `07d6f88` on `079-lvm-refactor` and owns the
following UI-only surface:

| Stays on `079-lvm-refactor` | Lives on `4-ui-testing` |
| --- | --- |
| Contract workflow types, deterministic `test-backend`, runtime adapter injection, and the shared scenario fixtures in `tests/ui/scenarios/` | `crates/ui-e2e-runner`, UI case manifests/traceability, Sway/AT-SPI container harness, UI-only Just recipes and CI jobs, and the test-only scenario marker |
| Plan 4 documents, including this baseline revision plan | The same plan, extended with branch-local implementation evidence and future R4/R5 updates |
| Upstream libcosmic `ef162b8e16ba4493e05c169cd56c7b9f77f0fda5` | Temporary `stoorps/libcosmic` pin `3d5fdb087534fb4944e64da2346f3bb673cc0215` and its iced submodule pin `25c211d8b1c0f456fd327b65be5261311b1d7692` |

The inherited, last-known-good capability evidence is deliberately narrow and
must remain reproducible before any case executor work proceeds:

```text
just ui-e2e
  application AT-SPI nodes:                 22
  descendants below the frame:              20
  interactive descendants:                   7
  stable scenario node: test.scenario
  marker: Test scenario: blank-state sha256:<fixture hash>
```

It establishes that the real app, the selected fixture, and its interactive
widget tree are present in the private Sway/AT-SPI session. It does **not**
establish that a case action, editable field, scenario-control command,
accessibility golden, visual golden, or trace assertion has executed.

The next implementation steps on this branch are ordered and fail-closed:

1. Add AccessKit support to iced `TextInput`: a stable widget node ID,
   `TextInput`/`PasswordInput` role, name/value/placeholder/bounds, and only
   the supported `Focus`, `SetValue`, and `ReplaceSelectedText` actions. An
   AT-SPI request must publish the application message generated by
   `on_input`; no shell typing or coordinate click may substitute for it.
2. Add stable IDs and accessible names in this application for each actual
   control selected by a v2 case. The ID catalog belongs next to the view
   code, and each target must be unique in a captured AT-SPI subtree.
3. Replace the v1 manifests with a closed v2 schema. The Rust runner must
   parse it, resolve exactly one ID/name/role target, perform the declared
   semantic action, wait only for the declared state/trace checkpoint, and
   reject unknown steps and duplicate targets.
4. Implement the private scenario-control client in the runner for only
   `advance_to`, `reload_overlay`, and `diagnostics`; pair every receipt with
   an expected sequence/generation/tick so virtual time is deterministic.
5. Add one real case at a time. A case is complete only after a successful
   semantic action, a trace assertion, canonical AT-SPI subtree golden, PNG
   golden, pixel comparison, and a deliberately failing action/tree/image
   check. Do not bulk-convert the eight v1 manifests.
6. Replace the capability-only CI job only when every required v2 case is
   complete. Until then, `just ui-e2e` remains an R3 transport gate and
   `ui-e2e-update` remains disabled.

Before beginning step 1, re-run the inherited capability command and inspect
`ui-artifacts/capability/capability.json` and `atspi-tree.json`. A result with
no `test.scenario` node or fewer than one interactive descendant is a bridge
regression, not a case failure, and blocks R4.

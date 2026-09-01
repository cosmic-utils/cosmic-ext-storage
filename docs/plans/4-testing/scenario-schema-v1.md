# Scenario schema v1

This document is the normative fixture contract for `schema_version = 1`.
It is deliberately independent of Rust's default `Serialize` layout: loader
DTOs own all field names, enum tags, defaults, and validation. A checked-in
fixture is valid only when it conforms to this document and `ui-scenario
validate` accepts it.

## Compatibility rule

`schema_version` is required and is the first non-comment key in a fixture or
overlay. Version 1 rejects unknown fields at every stable table. A changed
field, enum variant, default, or the meaning of an existing operation requires
a new schema version. A newly added *contract* method does not by itself change
v1: it is added to the versioned operation inventory with the mandatory
`native_only` default until a later schema version defines its fixture form.
That method must still return typed `Unsupported` in v1 and be represented in
the traceability/test inventory. A method may not gain a v1 modelled transition
without a v2 fixture migration.

This ordering rule is checked before TOML deserialization by a byte-level
pre-parser: input is UTF-8 without a BOM; it skips blank lines and lines whose
first non-space/tab byte is `#`; the first remaining line must match exactly
`schema_version = 1` followed only by spaces, tabs, `\r`, or a `#` comment.
Tables, dotted keys, multiline values, and a different numeric spelling before
that line are rejected. TOML map iteration order is never used to validate this
rule.

The loader must preserve declaration order for arrays, sort map-like state by
their documented key before exposing it, and emit all persisted state and trace
JSON using canonical key order. Fixture identity is the SHA-256 hash of the
exact immutable fixture bytes; overlay identity also records that base hash.

## Frozen Phase-0a source set

The complete machine-readable Phase-0a lock consists of:

- [`schema-v1.json`](schema-v1.json), the canonical root/state/overlay/trace
  descriptor emitted by `ui-scenario print-schema --version 1`;
- [`contract-surface-v1.toml`](contract-surface-v1.toml), the bootstrap
  inventory of every current trait method plus explicitly marked compatibility
  tombstones, with operation ID, default, request selector family, redaction
  policy, and v1 status; and
- [`tests/ui/traceability.toml`](../../../tests/ui/traceability.toml) and
  [`tests/ui/required-tests.toml`](../../../tests/ui/required-tests.toml), the
  complete executable coverage/test-target declarations.

Before Phase 0a can pass, `schema-v1.json` must contain a closed
`dto_catalog` entry for every referenced DTO. Each entry has its exact TOML
field names, field type/range, required/default status, enum tags, nested DTO
reference, lexical validator, and `additional_properties = false` rule. A
domain type name alone is not a DTO definition. The catalog is generated from
the Phase-1 public value declarations into a checked-in JSON artifact and is
reviewed as a compatibility change; the loader consumes generated DTO code from
the same declaration. `ui-plan-check` rejects a descriptor with an unresolved
domain reference, a DTO without fields, or a transition/selector that names an
unknown DTO. This is the required Phase-0a design task, not work deferred to a
state phase.

Phase 1 moves the active bootstrap inventory into the contract-surface
declaration macro without changing any ID, default, or selector. `ui-plan-check`
compares active generated entries with active bootstrap entries exactly, and
requires every `retired_after_phase_*` entry to remain a v1
`native_only_unsupported` tombstone. The generated source then becomes
authoritative and the bootstrap file remains a compatibility snapshot. There is
no code exception: an operation absent from the lock may not be introduced
opportunistically in a later state phase.

## Top-level fixture

Only these keys are legal at the root:

```toml
schema_version = 1
name = "kebab-case-fixture-id"
description = "Optional human-readable description"

[backend]
id = "ui-scenario"
capabilities = { partitioning = true, filesystem_operations = true,
  encryption_operations = true, image_operations = true,
  drive_power_management = true, logical_storage = true }

[state]
# Every state table below is optional and defaults to the documented empty form.

[behaviour]
rules = []
```

`name` matches `[a-z0-9][a-z0-9-]{0,62}`. `backend.id` follows the same rule.
Capabilities use all six keys shown above; omitted keys are `false`.

`state` has only these optional tables/arrays:

| Table or array | Empty form | Identity and ordering |
| --- | --- | --- |
| `disks` | `[]` | `id`, then `device` |
| `volumes` | `[]` | tree order by `parent_device`, `offset`, `device` |
| `partitions` | `[]` | `parent_device`, `number` |
| `filesystems` | `[]` | `device` |
| `luks` | `[]` | `device` |
| `smart` | `[]` | `device` |
| `filesystem_tools` | `[]` | `fs_type` |
| `btrfs` | `[]` | `mount_id` |
| `logical` | `{ sources = [], entities = [], candidates = [], preflights = [] }` | source order is fixed below; entities by logical ID |
| `network` | `{ backends = [] }` | backend ID, then config ID |
| `usage` | `{ mounts = [], files = [], operations = [] }` | mount ID, then operation ID |
| `images` | `{ assets = [], attachments = [], operations = [] }` | synthetic ID |
| `desktop` | `{ selections = [], launches = [] }` | declaration order |

The canonical field/default/validation tables are in `schema-v1.json`; their
human-readable summary is below. A DTO may map to a public domain value, but it
must never use `#[serde(flatten)]`, `toml::Value`, `serde_json::Value`, or a
Rust-private enum representation as an extension escape hatch. The checked-in
descriptor is a bootstrap draft until the catalog completion rule above has
passed; no implementation phase may treat its abbreviated domain references as
a fixture schema.

| DTO table | Required identity fields | Remaining fields and cross-references |
| --- | --- | --- |
| `disks` | `id`, `device` | `DiskInfoDto`; `device` is unique and `/dev/ui-*` |
| `volumes` | `device`, `parent_device`, `offset` | `VolumeInfoDto`; parent is a disk or volume; tree is acyclic |
| `partitions` | `parent_device`, `number` | `PartitionInfoDto`; parent disk and volume device must agree |
| `filesystems` | `device` | `FilesystemInfoDto`; device exists in volumes/partitions/attachments |
| `luks` | `device` | `LuksInfoDto`; device exists and its cleartext mapping, if present, is synthetic |
| `smart` | `device` | `SmartInfoDto`; device is a disk |
| `filesystem_tools` | `fs_type` | `FilesystemToolInfoDto`; command is display metadata only |
| `btrfs` | `mount_id` | `BtrfsMountDto`; mount references one declared filesystem/mount |
| `logical` | source enum and logical entity IDs | fixed Udisks/LocalTools registrations; every entity/candidate/preflight references declared IDs |
| `network` | backend ID, config ID | `NetworkBackendDto`; backend IDs/config IDs are unique within their scope |
| `usage` | mount ID, file ID, operation ID | files belong to a declared usage mount; operation schedules are finite |
| `images` | asset ID, attachment ID, operation ID | attachments reference an asset and a synthetic device; schedules are finite |
| `desktop` | selection ID / launch ID | selection references an image asset; launches are record-only |

Every `*Dto` in this table must be a named closed record in the completed
`dto_catalog`, with required/defaulted fields, lexical constraints, and
cross-references. No later phase may add a v1 field without reopening Phase 0a
and its compatibility tests.

## Safe identifiers and path-shaped values

Scenario state contains identifiers, not usable host capabilities.

- A block device is represented by a `BlockDeviceRef` plus a display path that
  matches `/dev/ui-[A-Za-z0-9._-]+`. A fixture rejects any other `/dev` path.
- Mount points match `/mnt/ui-[A-Za-z0-9._/-]+`; `..`, empty components, and
  NUL are invalid.
- Image inputs are `asset:<id>`, never host paths. A simulated attachment
  produces only a configured `/dev/ui-*` device.
- Desktop picker responses are a declared `asset:<id>` or `cancelled`; reveal
  and URL operations are recorded requests, not launched programs.
- A filesystem-tool `command` is immutable display metadata identifying a
  tool; it is never executed, parsed as an argument list, or accepted as an
  operation input. Process command names are likewise display-only.
- Secrets are supplied only in a request at runtime. Fixtures, overlays,
  traces, diagnostics, and schema errors store neither their value nor a hash.

The only allowed host filesystem access in scenario mode is the explicitly
chosen fixture, overlay, trace, and runner-artifact paths through the
`ScenarioStore` I/O facade. No operation may open, stat, create, delete, or
launch a state-derived path. This is the safety property to test; it does not
prohibit ordinary application configuration and logging outside storage flows.

## Logical sources and image devices

`state.logical.sources` is not user-ordered. Every v1 fixture produces exactly
two source registrations in this order: `udisks`, then `local_tools`. A missing
source is represented by its declared unavailable status and an empty entity
list. This preserves the application registry invariant.

Native file descriptors are not a scenario contract. Until the atomic Phase-4
migration removes `open_for_backup` and `open_for_restore` from
`BlockStorageBackend`, `test-backend` implements those legacy signatures only
as `native_only` typed `Unsupported` responses. They cannot create, retain, or
return an FD. The Phase-4 replacement typed image attachment operation consumes
an `asset:<id>` and returns a declared synthetic device. Every method of the
resulting public contract is represented below and is either modelled or marked
`native_only` with a required typed `Unsupported` response.

## Behaviour rules

Rules use a closed tagged union; separate `faults`, `events`, or arbitrary
callback tables are not legal.

```toml
[[behaviour.rules]]
operation = "filesystem.unmount"
match = { tag = "device", device = "/dev/ui-disk0p1" }
outcome = { tag = "error", kind = "busy", message = "Fixture user is active" }
latency = { ticks = 0 }
events = []
```

Every `operation` is one generated `ScenarioOperation` name. Its `match` uses
the closed `RequestSelector` union from `schema-v1.json`: `any`, `device`,
`mount`, `asset`, `network_config`, `logical_preflight`, `operation_id`, or
`request_digest`. A selector has exactly its documented fields; a request
digest is the SHA-256 of the canonical tagged request DTO emitted by
`ui-scenario describe-request`, never a serialized Rust value. Multiple
selectors are an ordered `all_of` list sorted by selector tag and then value;
duplicates and contradictory selectors are schema errors. This provides exact
matching without an untyped key/value extension map.

The selector records are exact: `any = { tag = "any" }`;
`device = { tag = "device", device = BlockDeviceRef }`;
`mount = { tag = "mount", mount = MountId }`;
`asset = { tag = "asset", asset = ImageAssetRef }`;
`network_config = { tag = "network_config", backend_id, config_id }`;
`logical_preflight = { tag = "logical_preflight", key }`;
`operation_id = { tag = "operation_id", operation_id = OperationId }`; and
`request_digest = { tag = "request_digest", sha256 = <64 lowercase hex> }`.
An `all_of` selector is exactly `{ tag = "all_of", selectors = [Selector, ...]
}` and contains two or more selectors sorted by `(tag, canonical value)`. The
generated contract declaration records the allowed selector tags for every
operation. Validation rejects a selector that is not allowed for that operation,
so overlap checking has a finite, type-aware request domain.

Success transitions use the closed `StateTransition` union in
`schema-v1.json`. Its variants are typed upsert/remove/update operations for
the state tables above, `complete_usage`, `complete_image`, and
`logical_action`. Fields not belonging to the variant are schema errors. A
transition must change state, schedule a future operation, or produce a
documented typed result override; an empty transition is rejected.
`outcome` is exactly one of:

- `error` with a `StorageErrorKind` and public message;
- `transition` with a typed, operation-specific state transition and optional
  typed result override; or
- `unsupported`, which returns `Unsupported` without state or event change.

No rule means the operation's documented modelled default. A default is always
either a defined typed transition or `Unsupported`; success is never implicit.

Rules are selected by `(operation, descending selector_count, descending
selector_specificity)`, where `request_digest` is 7, `logical_preflight` is 6,
`network_config` is 5, `operation_id` is 4, `asset` is 3, `mount` is 2,
`device` is 1, and `any` is 0. The specificity of `all_of` is the sum of its
members. The generated operation selector declaration supplies the finite list
of selector-bearing request fields, and validation rejects two rules when the
intersection of their allowed values is non-empty at equal comparator value.
Declaration index is used only to preserve trace display order, never to break
a tie. Events occur only after a successful transition and are listed in
declaration order.

## Time, events, overlays, and trace

Time is virtual. `latency.ticks` and progress schedules use nondecreasing
integer ticks; tests advance a `ScenarioClock` explicitly. The runner may wait
for a named semantic-ready condition, but it never uses elapsed wall time to
decide a scenario result.

Each mutating request is serialized by one runtime operation queue. It receives
a monotonically increasing `sequence`; trace records request, rule, response,
generation, and events with that sequence. A successful topology transition
increments generation once and publishes `(generation, sequence, event)` to
each subscriber from an ordered, non-dropping per-subscriber queue. Shutdown
closes the stream; an otherwise idle stream remains pending.

An overlay contains `schema_version`, immutable fixture hash, state snapshot,
and the last applied sequence. It cannot contain behaviour rules. The backend
writes it as temp file, file fsync, rename, and parent-directory fsync. Its own
write is acknowledged by sequence and does not trigger reload. For an
overlay-backed mutation, the actor first applies the transition to a clone and
persists that clone; only a successful atomic write publishes state, generation,
events, and a success response. A write failure returns `StorageErrorKind::Other`
and leaves all published state unchanged. An overlay loaded at startup resumes
with next sequence `last_sequence + 1`; a stale or malformed sequence is an
error. An external
replacement whose base hash and schema validate is queued after in-flight work,
replaces state at one sequence, cancels affected synthetic operations with a
typed conflict, increments generation once, and emits the sorted set of changed
synthetic devices. The diff is the symmetric difference of old/new synthetic
device paths plus devices whose typed record differs; it emits `Removed` before
`Added` for the same path and otherwise sorts lexicographically by path. Invalid
or stale overlays leave runtime state untouched.

`--watch-scenario` is a convenience trigger only. Every filesystem notification
causes at most one queued reload request for a new exact overlay byte sequence;
duplicate/self-write notifications are ignored by `(overlay_bytes,
acknowledged_sequence)`. Automated cases do not assert notification timing:
they atomically replace the overlay and invoke the scenario diagnostic page's
`Reload scenario` semantic command, which queues the same reload and returns its
trace sequence.

Trace timestamps are virtual ticks only; IDs are deterministic
`<fixture-name>:<operation>:<sequence>`. Trace redaction replaces secret values
and descriptor-equivalent fields with the literal `"<redacted>"` before they
reach an in-memory trace entry. Canonical JSON uses UTF-8, lexicographic
byte-order object keys, no insignificant whitespace, lower-case hexadecimal,
and Rust/Serde's shortest round-trippable integer form; floats are forbidden
from fixtures, overlays, and traces.

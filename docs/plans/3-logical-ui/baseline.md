# Baseline

**Observed branch:** `079-lvm-refactor` at `925d96f`.

## Current UI behaviour

`src/views/logical.rs` is a functional placeholder, not a finished detail
surface:

- it renders raw entity IDs and byte counts, a generic metadata dump, and the
  same four tabs for every entity type;
- its `Operations` tab iterates every logical operation, including operations
  that cannot apply to the selected entity;
- only delete/activate/deactivate/start/stop/check/repair have enough input to
  construct an action. All creation, device, sizing, label, and Btrfs
  subvolume/snapshot actions become prose saying that more input is required;
- progress is reduced to a debug-like operation name, while the existing
  `action_status`, `action_progress`, source status, health, and member data
  are not presented as a coherent status surface;
- a non-Btrfs entity still receives a Btrfs tab, and Btrfs receives only the
  generic metadata renderer.

The Logical sidebar is already useful as a navigation start: it has candidate
discovery, root/entity expansion, selection, and physical-device paths. It
does not need a second tree inside the detail page. It does need the detail
page to handle a selected candidate that cannot resolve to a logical root
without silently selecting an unrelated first root.

The physical-volume page also contains an older Btrfs management section
(`src/views/btrfs.rs`, `src/update/btrfs.rs`, and the Btrfs volume dialogs).
It offers subvolume and snapshot controls, but it is a second state machine
keyed by a block path and mount point. Its mutations ultimately reach native
logical operations through `BtrfsClient`; it must not remain a parallel UI.

## Data and contract reality

The typed action boundary is already broad enough for the intended UI:

- `storage_contracts::LogicalAction` covers LVM VG/LV/PV lifecycle, MD array
  lifecycle and sync, Btrfs device membership, absolute-size resize, label,
  default subvolume, subvolume, and snapshot operations.
- `LogicalState` already guards stale load/action completions and permits only
  one pending action. Its selection and refresh behaviour are worth retaining.
- the UDisks adapter re-resolves a `BlockDeviceRef` immediately before a
  native member mutation, protecting against a recycled device number.

There are two material blockers to an honest completed UI.

1. Btrfs discovery currently creates one filesystem entity per Btrfs UDisks
   object and then deduplicates them by filesystem ID. The surviving root has
   zero capacity, no member devices, and its display name is one arbitrarily
   retained block path. It does discover subvolumes, but only emits their
   parent ID as metadata. A multi-device filesystem therefore cannot be
   rendered as a filesystem.
2. Topology exposes device references only for already attached members.
   Creation and add-member forms need an authoritative, current list of
   eligible block-device candidates. Reconstructing a reference from a
   `/dev/*` path in the UI would bypass the identity guarantees and is
   prohibited.

The Btrfs proxy also exposes mutation and subvolume methods, but no currently
used property for label, member capacity, or a complete filesystem summary.
The implementation must obtain those values from documented UDisks block or
Btrfs data, or render them as unknown; it must not invent a value from the
first device.

## Execution prerequisites discovered during plan review

The original implementation order identified the right outcomes but left six
implementation contracts implicit. They are now specified in
[spec.md](spec.md#execution-contracts) and scheduled as delivery phase 0.

1. `LogicalEntity` currently represents an unknown aggregate size as `0` and
   stores Btrfs default/subvolume fields in `metadata`. It has no typed Btrfs
   filesystem, member, hierarchy, candidate-anchor, preflight, or review
   model. The existing Btrfs action variants also accept a raw subvolume ID or
   path. The phase-0 domain migration replaces those selected inputs with
   `BtrfsSubvolumeRef` and makes display availability explicit.
2. The physical sidebar and `VolumeInfo` carry only a device path. The current
   logical state stores that path and, if it cannot resolve, selects the first
   root. A new read-only anchor-capture operation is therefore required before
   the page can promise identity-based candidate resolution.
3. `ResolvedBlock` currently derives its fingerprint from `Block.IdUUID` and
   `Block.IdType`. Every member of a multi-device Btrfs filesystem shares those
   values, so that pair is not a strong per-member identity. Current Btrfs
   operation lookup also returns the first matching ObjectManager object rather
   than a comparator-selected primary proxy. Both behaviours must be replaced
   before a member ref can authorize a mutation.
4. No preflight contract exists. In particular, the adapter has no typed
   candidate eligibility result, size-constraint source, audited MD profile
   catalog, or bound review payload. The current action executor consequently
   cannot verify a confirmation's preflight key.
5. Logical action success currently starts logical and physical refreshes
   directly from an update arm. There is no common refresh state machine or
   physical-load generation that can distinguish a pre-success load from the
   required successor.
6. The full-lab catalog contains logical Btrfs/LVM/MD cases, but its executor
   deliberately reports every selected case as blocked because no fixture
   scenario is registered. This plan owns the targeted logical-suite executor
   and fixture cases; it does not claim the unrelated global full-lab profile
   is already executable.

## Verified UDisks field inventory

The adapter can use these documented fields without inventing a filesystem
aggregate:

| Need | Source and normalisation | Absence/conflict behaviour |
| --- | --- | --- |
| Btrfs grouping key | `Block.IdUUID`, trimmed then parsed as `uuid::Uuid` and serialised lowercase | Candidate-specific unavailable result; never make a name-derived root. |
| Member label | `Block.IdLabel`, trimmed; accept only one normalised value across members | `Unknown`; disagreement is a diagnostic, not first-member selection. |
| Member size | `Block.Size` for that member only | `Unknown` member size. It is never summed into filesystem capacity. |
| Member writable state | `Block.ReadOnly`: `false` is writable and `true` is read-only | `Unknown` with the property/source reason; colour is never the only state. |
| Default subvolume | Btrfs `GetDefaultSubvolumeID` on the deterministic primary | `Unknown` on unavailable/error/out-of-range result. |
| Subvolume topology | Btrfs `GetSubvolumes` on the deterministic primary | Source failure/unavailable; no partial arbitrary merge. |
| Filesystem aggregate capacity/allocation | No verified filesystem-wide field in the current proxy | `Unknown`. A mounted `statvfs` read may appear only as explicitly labelled mount-point usage. |
| Device identity | Partition UUID bound to containing-drive WWN + serial; otherwise drive WWN + serial; otherwise loop backing device + inode | Readable/inert member or unavailable picker candidate. A partition UUID without its drive binding and `Block.IdUUID`/filesystem UUID are never strong Btrfs-member identities. |

The phase-0 adapter tests must prove these mappings against snapshot fixtures,
including every `Unknown` and disagreement path.

## Existing verification

At this baseline the focused contract suite passes:

```text
cargo test --test logical_ui_contract --test logical_state_contract --test sidebar_async_contract
```

The tests are intentionally shallow: they cover generation handling and a few
capability/confirmation invariants, but do not render the page or exercise any
input-taking operation. The plan adds behaviour-focused coverage rather than
treating the current green suite as UI acceptance.

The current full-lab command is intentionally not a baseline gate for this
plan: `FullLabExecutor` blocks every selected case until a fixture scenario is
registered. The targeted logical-suite gate described in
[validation.md](validation.md#disposable-fixture-gate) replaces that impossible
claim and must be implemented before final acceptance.

## Scope boundary

In scope:

- logical page layout, state, forms, confirmations, feedback, sidebar/detail
  handoff, and localisation;
- richer Btrfs topology and operation-preflight data that the UI must consume;
- consolidation of the physical Btrfs controls onto that one workflow.

Out of scope:

- a new storage backend, service, Polkit policy, `sudo`/`pkexec`, or command
  based mutation path;
- new logical-storage operations beyond the existing typed action matrix;
- interpreting the unsupported Btrfs `max`, grow-by, or shrink-by resize
  syntax as an approximate native operation.

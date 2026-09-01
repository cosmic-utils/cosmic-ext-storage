# Logical Action Matrix

This matrix is the UI contract for `LogicalAction` variants and their typed
input refinements. A final action is dispatched only as
`ConfirmedLogicalAction { action, preflight_key }`. It keeps the action set
truthful: a control is either wired to a validated typed action or shown as
blocked with the native reason. It must not submit raw device or
selected-subvolume paths, invent an alternate operation, or construct an
identity from display text.

| Entity/context | User control and form | Typed action | Notes |
| --- | --- | --- | --- |
| Logical landing page | Create volume group: name; one-or-more preflight-selected devices | `CreateLvmVolumeGroup` | The picker requires distinct current refs. |
| LVM VG | Add physical volume: preflight-selected device | `AddLvmPhysicalVolume` | Root capability controls visibility. |
| LVM VG | Remove physical volume: member row selected by fresh ref; preserve-label review | `RemoveLvmPhysicalVolume` | Inert if member has no ref. |
| LVM VG | Create logical volume: name and non-zero byte size | `CreateLvmLogicalVolume` | Display available VG capacity beside input. |
| LVM VG | Delete volume group: review dependent LVs and affected PVs | `DeleteLvmVolumeGroup` | Preserve policies stay fixed; refreshed scope is required. |
| LVM LV | Resize: non-zero byte size | `ResizeLvmLogicalVolume` | No approximate percent/path form. |
| LVM LV | Activate / deactivate | `ActivateLvmLogicalVolume` / `DeactivateLvmLogicalVolume` | Direct confirmation when required by policy. |
| LVM LV | Delete: review | `DeleteLvmLogicalVolume` | Preserve configuration policy. |
| Logical landing page | Create RAID: name, supported level, preflight-selected members, and the displayed audited profile | `CreateMdRaidArray` | Preflight supplies the exact locked `MdRaidCreateProfile` for each enabled level, including member minimum, chunk size, and metadata version. If more than one profile is supported for a level, the user must choose the labelled profile; level alone never silently selects one. |
| MD array | Add/remove member: preflight/member-row device selection | `AddMdRaidMember` / `RemoveMdRaidMember` | Removal review preserves member signatures. |
| MD array | Start / stop; check / repair | `StartMdRaidArray`, `StopMdRaidArray`, `RequestMdRaidSync` | Show progress and health in the flattened header/status presentation. |
| MD array | Delete array: review every affected member | `DeleteMdRaidArray` | Preserve configuration policy. |
| Btrfs filesystem | Add device: preflight-selected current device | `AddBtrfsDevice` | Device list refreshes after completion. |
| Btrfs filesystem | Remove device: selected fresh member row | `RemoveBtrfsDevice` | The review names the current member ref and fixed native effect. Preflight must prove that there is no additional collateral scope; otherwise the control is blocked rather than submitting an unbound review. |
| Btrfs filesystem | Resize: absolute, non-zero byte size | `ResizeBtrfsFilesystem { AbsoluteBytes }` | `max`, grow, and shrink syntax are explicitly unsupported. |
| Btrfs filesystem | Set label: label text | `SetBtrfsLabel` | Empty label is permitted only if the native action permits it. |
| Btrfs filesystem or subvolume | Make default: populated `BtrfsSubvolumeRef` selector | `SetBtrfsDefaultSubvolume { subvolume }` | The active default is indicated in the list. The adapter rechecks ID, path, parent, epoch, and the native `u32` range from the selected ref. |
| Btrfs filesystem | New subvolume: relative subvolume name | `CreateBtrfsSubvolume` | Reject absolute/traversal names before confirmation. |
| Btrfs filesystem/subvolume | Delete subvolume: selected `BtrfsSubvolumeRef` and review | `DeleteBtrfsSubvolume { subvolume }` | No recursive deletion option is presented. Preflight blocks a selected subvolume with descendants or source-conflict topology; the adapter resolves the ref to its fresh relative path. |
| Btrfs filesystem/subvolume | New snapshot: source `BtrfsSubvolumeRef`, relative destination, read-only toggle | `CreateBtrfsSnapshot { source, destination, readonly }` | Source comes from current filesystem subvolumes; destination is a validated new-name payload, not an identity. |

## Form implementation rules

- `LogicalOperation` alone is not enough to describe a click. The page opens
  a discriminated draft/form variant so the typed action is complete before
  the confirmation dialog is displayed.
- The action button is disabled while the draft has a validation error, a
  preflight load is pending, a dialog is running, or another logical action is
  pending. The reason is visible in the form, not only inferred from a
  disabled button.
- The confirmation dialog receives the final `ConfirmedLogicalAction`, a
  human-readable summary, and the adapter-returned epoch-bound preflight key.
  It cannot be used to edit free-form values. On conflict or key invalidation
  it returns to the form with its non-identity input retained and requests
  fresh candidates/review-facts/scope.
- Byte-size forms use the preflight's explicit inclusive minimum/maximum and
  alignment constraints. A value outside those constraints is inert before
  confirmation; the adapter remains the final authority after its fresh
  snapshot revalidation.
- The `Create` capability is contextual. On a VG it means create LV; on a
  Btrfs filesystem it exposes subvolume/snapshot actions; the landing page
  owns root-level VG/RAID creation. The UI never displays an ambiguous bare
  `Create` button.

## Candidate eligibility and constraint sources

All device-taking rows request the matching `LogicalPreflight` before opening
their picker. The UI supplies a `LogicalPreflightRequestKey`; only the returned
preflight carries the UDisks epoch. A `Ready` candidate is selectable only when
it has the current strong `BlockDeviceRef`, is not already a member of the
target, and has no detected structured-data signature. A `Blocked` candidate
has display data and an exact backend reason but no device ref, so it cannot
become action input. The display path is informational only.

The contract-owned MD profile catalog supplies every labelled profile in the
MD creation selector. LVM capacity derives from `VolumeGroup.FreeSize`; Btrfs
resize has an audited open absolute-byte bound (`min = 1`, no claimed maximum,
alignment `1`) unless a later documented native source tightens it. If a
preflight cannot produce the declared source, it returns `Blocked` rather than
an unconstrained form.

## Confirmation and freshness binding

Every review is bound to the final typed action and the matching preflight key.
The executor captures one fresh completed snapshot, rejects an epoch-stale key
against it, then re-resolves every target/reference from that same snapshot
immediately before submission. The following classifications are exhaustive:

| Action family | Review payload | Revalidation rule |
| --- | --- | --- |
| Create LVM VG, add LVM PV, create MD RAID, add Btrfs device | Selected fresh device refs, fixed native data-loss effect, and (for MD) the exact audited profile | Every selected ref and the profile capability must still match. There is no separate collateral scope because the selected refs are action inputs. |
| Delete LVM VG/LV and delete MD RAID | `ConfirmedDestructiveScope`, excluding the primary entity, plus the fixed preserve/tear-down policy | Recompute the exact scope; any difference is `Conflict` and requires a new review. |
| Remove LVM PV, remove MD member, remove Btrfs device | Selected fresh member ref and fixed preserve/native effect | The member ref and capability must still match. If native discovery reports collateral effects beyond that member, block the control; do not present an unbound review. |
| Delete Btrfs subvolume | Selected `BtrfsSubvolumeRef` and an explicit non-recursive effect | The ref must resolve to the same ID, parent, and relative path and have no descendants. |
| Resize, label, default-subvolume, activate/deactivate, start/stop, check/repair | Final semantic values and any policy-required warning | Revalidate target capability and the applicable input constraints; no collateral scope is claimed. |

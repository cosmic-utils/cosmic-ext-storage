# 3 — Logical and Btrfs UI Completion

**Status:** Ready for implementation

This plan turns the current logical-storage detail page into a complete,
native-UDisks management surface for LVM, MD RAID, and Btrfs. The immediate
priority is the Btrfs filesystem page: it must describe a multi-device
filesystem accurately and provide one coherent place to manage its devices,
subvolumes, snapshots, label, size, and default subvolume.

The work is deliberately UI-led, but it includes the topology, identity,
preflight, refresh, and fixture additions the UI needs to be truthful and
safe. It does not broaden the storage feature set, add a privileged helper, or
permit a production command-based mutation fallback.

## Documents

- [baseline.md](baseline.md) records the observed state and gaps at the start
  of this work.
- [spec.md](spec.md) defines the finished experience and its safety rules.
- [action-matrix.md](action-matrix.md) maps every supported typed action to a
  visible control or an explicit unavailable state.
- [mock.html](mock.html) is a responsive, self-contained visual mock of the
  proposed Btrfs logical-filesystem page.
- [implementation-plan.md](implementation-plan.md) is the ordered delivery
  plan and file-level change map.
- [validation.md](validation.md) defines the automated and manual acceptance
  gates.

## Decisions

- The global sidebar remains the topology navigator. The detail page does not
  grow a second, competing topology pane.
- A logical root gets a shared detail shell with only applicable sections. The
  Btrfs page is one vertically ordered view, not an internal tab set.
- [mock.html](mock.html) is the approved Btrfs information-hierarchy and
  interaction reference. It is not a second design system: production uses
  the application's existing COSMIC components, theme, symbolic icons,
  typography, and spacing tokens.
- All mutation flows produce a `LogicalAction` and execute only through the
  registered UDisks `LogicalOperations` implementation. Direct `btrfs` CLI
  mutation is not a fallback.
- A device picker may only offer a freshly discovered `BlockDeviceRef`; a path
  string, a sidebar node, and a stale member entry are never mutation input.
- A preflight request is keyed only by state the UI owns; the adapter appends
  the captured UDisks epoch to its returned confirmation key. Disabled picker
  rows have no device ref and cannot accidentally become action input.
- Selected Btrfs subvolumes use a fresh `BtrfsSubvolumeRef`; their displayed
  relative paths are verified snapshot data, not mutation identity. New
  subvolume/snapshot names are validated creation payloads only.
- Btrfs discovery canonicalises filesystem UUIDs and uses the specified member
  comparator to select one primary proxy. Member ordering, primary selection,
  and malformed subvolume handling are deterministic and fixture-tested.
- The implementation begins with the concrete contracts in
  [spec.md](spec.md#execution-contracts). A UI commit may not choose a domain
  representation, identity fallback, preflight shape, or refresh-coalescing
  rule that is not defined there.
- A physical sidebar path is permitted only to capture a fresh,
  display-oriented `LogicalCandidateAnchor`. Every later candidate resolution
  is by the anchor's block identity and epoch rules; neither the anchor nor an
  action treats the path as authority.
- A Btrfs filesystem UUID and an unbound partition UUID are grouping/display
  data, never strong identities for an individual member. A member/device ref
  requires the documented per-block identity hierarchy—including the drive
  binding for a partition—or is readable but inert.
- The logical full-lab gate is a targeted `full-lab`-profile `logical` suite.
  It must execute every selected logical case in the disposable fixture; it
  does not claim that unrelated, currently unimplemented full-lab suites ran.
- The existing physical-volume Btrfs panel must delegate to the same logical
  Btrfs state and dialogs, or be removed once the logical page is reachable.
  Two independently evolving Btrfs mutation UIs are not acceptable.

## Completion definition

The work is complete when a user can open a logical root and understand its
type, health, member state, and applicable controls without raw implementation
data; every capability for that entity is either actionable through a validated
form or visibly unavailable with its exact reason; and a multi-device Btrfs
filesystem has one accurate root, device list, and subvolume/snapshot surface.
The gates in [validation.md](validation.md) are required before calling the
plan complete.

# Keyboard accessibility: observed blocker

2026-09-14, continuing directly on `4-ui-testing` after `c1f1e29`.
Status: the scoped repair was approved and implemented on the existing pinned
fork; real Tab/Shift+Tab widget-focus assertions now pass. This is not completed
keyboard-dialog E2E acceptance. The diagnosis and audit below describe the
original pin; see [the fix ledger](dependency-fix-ledger.md) for the two separate
focus patches, exact replacement pins, regressions and integration evidence.

## Harness defect fixed locally

The first keyboard probe failed before the application started. Sway reported
`Socket path won't fit into ipc_sockaddr->sun_path` and then SIGSEGV. Its runtime
directory was nested inside the artifact path, which includes the case name.
`keyboard_accessibility` made the IPC socket path longer than Linux's Unix
socket bound. This was not the iced application teardown crash and is not
quarantined.

`CapabilitySession` now owns a short `/tmp/cs-ui-*` directory using the existing
pinned `tempfile` crate (promoted from dev-only to runner runtime dependency).
The directory is private (0700) and its guard drops after session children are
terminated/reaped. Artifacts retain their descriptive paths and survive cleanup;
the control token is still removed. Host `TMPDIR` cannot lengthen runtime paths.
The Cargo.lock bytes and GUI/image/quarantine bindings are unchanged.

Two rstest regressions exercise real Unix-socket binds with a long artifact
parent, privacy, runtime/token cleanup, preserved evidence, and rejection of an
existing artifact directory. Both are in the required inventory and active
validation appendix. Runner tests pass 70/70. Normal capability passes and the
unchanged nine-step reload case passes with its existing known-shutdown warning.

## Focus publication is missing in the pinned stack

After the runtime fix, the keyboard probe started the real application and
delivered Tab through wtype. The observed AT-SPI trees contain focusable buttons
but no focused widget. The exploratory assertion naming the disk timed out.
That assertion alone does **not** prove the disk should be first in tab order;
the decisive finding is in the pinned implementation:

- libcosmic `ab3a7b9` embeds iced `25c211d8b1c0f456fd327b65be5261311b1d7692`.
- [`a11y_tree_update`](https://github.com/stoorps/iced/blob/25c211d8b1c0f456fd327b65be5261311b1d7692/winit/src/lib.rs#L85)
  sets every `TreeUpdate.focus` to `window_root` unconditionally. Its redraw
  call site supplies the node tree, but no actual focused-widget ID.
- Its existing tree-root regression explicitly expects the window ID as focus.
  A correctly focused button therefore cannot be published as focused by this
  path, regardless of how many Tab keys the test sends.

Do not weaken `Assertion::Focused`, infer focus from selection/highlight, use
an accessible click in place of keyboard activation, or add sleeps/retries to
hide this. The required keyboard case remains schema-v1 inventory rather than
promoting an incomplete exploratory program to a completed UI flow.

## Existing-fix check

Searched open and historical PRs in `pop-os/libcosmic`, `pop-os/iced`, and the
two `stoorps` forks for focus/accessibility/a11y work. No compatible widget-focus
publication fix was identified in those results. Relevant candidates inspected:

- [libcosmic #1215](https://github.com/pop-os/libcosmic/pull/1215) registers
  buttons with focus operations; that code already exists in our pin. It does
  not publish the focused ID to AccessKit.
- [libcosmic #1404](https://github.com/pop-os/libcosmic/pull/1404) requests a
  segmented-button redraw after Tab; it does not fix this publication path.
- [libcosmic #1425](https://github.com/pop-os/libcosmic/pull/1425) adds visual
  menu focus marks, not AT-SPI focus state.
- [iced #390](https://github.com/pop-os/iced/pull/390) concerns Wayland
  surface/popup focus events, not widget accessibility focus.
- [iced #45](https://github.com/pop-os/iced/pull/45) is closed historical,
  broad pre-current-API work, not a targeted current fix.

At the time of this audit, no upstream PR was opened and no dependency
branch/pin was changed. The later approved fork-only changes are recorded below.

## Approved bounded repair

The user approved repairing **focus publication only** in the existing
pinned fork: query the UI's actual focused widget during tree generation,
publish its existing accessibility ID, preserve root fallback when nothing is
focused, and add regression tests for focused/unfocused and removed-widget
states. Revalidate real Tab/Shift+Tab/activation/cancellation through AT-SPI.
Do not update to master, redesign Wayland lifetime/teardown, expand quarantine,
or open a libcosmic PR. Any required pin/hash rebind must remain explicit and
reviewed. This unblocks a necessary assertion mechanism, not all remaining
coverage, visual-review or UI-flow requirements.

Implementation found a second necessary focus-publication omission: AccessKit
also needs the real window Focused/Unfocused transitions, which the pinned
runtime was not forwarding. Repair 6 publishes the widget ID; repair 7 forwards
those existing window events without changing focus policy. The runner retains
an owned virtual keyboard for key-driven cases and reaps it on cleanup.

The exact [successful diagnostic program](keyboard-focus-probe.toml) is retained
outside the mandatory case inventory. It traverses the five header controls,
asserts focus on Volume, Usage and Keyboard Scenario Disk, then uses Shift+Tab
and asserts Usage focus again. It is not a substitute for dialog activation,
disabled-reason, cancellation or submission coverage. The full
`keyboard_accessibility` inventory remains planned. Do not count this diagnostic
as one of the seven completed remaining UI cases.

## Evidence

Ignored local evidence directory: `target/testing-v2-completion/ui/`.

- `keyboard-01.log`: original Sway path overflow; artifacts
  `ui-artifacts/executed/keyboard_accessibility-12-1789415721716888588`.
- `keyboard-02.log`: fixed runtime, failed focus assertion; artifacts
  `ui-artifacts/executed/keyboard_accessibility-12-1789415889045786263`.
- `keyboard-focus-probe.toml`: the exact exploratory program, preserved for
  investigation rather than registered as a completed mandatory case.
- `runner-tests.log`, `capability-short-runtime.log`, `clippy.log` and
  `reload-short-runtime.log`: local verification. Reload artifact:
  `ui-artifacts/executed/live_scenario_reload-13-1789416116943067763`.

No fresh combined coverage percentage is claimed after this source change;
the previous 39.81%/39.18% report is now a dated baseline, not current acceptance.

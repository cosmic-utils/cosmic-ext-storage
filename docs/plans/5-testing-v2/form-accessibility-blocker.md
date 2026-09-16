# Form accessibility: missing widget support

2026-09-14, continued from `4-ui-testing` commit `0a52ec5`.
Status (2026-09-15): the user approved the custom text-input/dropdown work.
The text-input, dropdown, iced overlay publication and exclusive accessibility
focus repairs are approved, committed and pushed to the pinned forks. The app
working tree integrates them; real normal/instrumented validation is in progress.
Testing V2 is not complete.
The live form now reaches editing and option selection, but disabled-state
validation is blocked by [AccessKit's Linux state translation](accesskit-disabled-state-blocker.md).
No coverage threshold, required UI flow,
secret-handling requirement or visual-approval obligation is waived.

## What now works

The standard iced Tooltip previously hid its entire wrapped accessibility tree.
The separately regression-tested repair is [fix 8 in the ledger](dependency-fix-ledger.md).
On libcosmic `7a4912de43b6720a86f9c2e663758f8870261472`, the real Create Partition
control is exposed and keyboard navigation/activation opens its wizard.

The [35-step diagnostic](keyboard-dialog-probe.toml) verifies exact focus on
Volume, Usage, the disk, Create Partition, Next and Cancel; activates and
cancels using only keys; observes the closed dialog, unchanged generation and
operation sequence (both zero), and the original full free-space extent.
Normal run `keyboard_accessibility-12-1789418812567602309` passes and shuts down
cleanly. Instrumented run `keyboard_accessibility-12-1789419156750270318` also
passes all 35 steps with clean shutdown and verified app/runner profiles.
These are useful partial assertions, not disabled-reason/submission
coverage or acceptance of the full required keyboard case.

## Evidence of the remaining gap

The open-dialog screenshot from
`ui-artifacts/executed/keyboard_accessibility-12-1789418620334977673/final.png`
shows a Volume Name text field and an ext4 filesystem selector. Its matching
`atspi-tree.json` contains neither field nor selector, although it contains
the wizard's labels and Cancel/Next buttons. This is not a selector-capitalization
or readiness problem:

- [Pinned COSMIC TextInput](https://github.com/stoorps/libcosmic/blob/7a4912de43b6720a86f9c2e663758f8870261472/src/widget/text_input/input.rs#L582) implements Widget but no
  `a11y_nodes` method or accessibility-event handling. This custom widget is
  not the standard iced text input; forwarding a child tree is insufficient.
- [Pinned COSMIC Dropdown](https://github.com/stoorps/libcosmic/blob/7a4912de43b6720a86f9c2e663758f8870261472/src/widget/dropdown/widget.rs#L374) has only a commented `a11y_nodes` TODO.
  It publishes neither the selection control nor its options through that path.
- The focused-ID repair correctly falls back to the window for an unexposed
  node. Changing that fallback would publish a dangling ID, not expose a field.
- The existing `keyboard_dialog_flow_has_named_controls` host test only checks
  that a TOML file exists. It remains an explicit gap, not evidence that these
  widgets can be found, focused, edited or selected.

Upstream check: searched current and historical pop-os/libcosmic PRs for
`accessibility text input`, `text_input a11y`, `a11y dropdown`, and
`dropdown accessibility`, plus open accessibility PRs. No compatible repair
was identified; the old examples PR #12 is unrelated. Read the current upstream
versions of both files: the text-input methods remain absent and dropdown
still contains the commented TODO. No master/release revision was adopted,
and no upstream PR was opened.

## Why this changes the scope

This is new accessibility support for interactive form widgets, not another
small forwarding or routing backport. It affects physical-format fields,
LUKS secret input and network provider/forms as well as keyboard coverage.
Blind typing, coordinates, backend mutations through the test control channel,
or reporting focus from highlights would not meet the existing acceptance
contract. Replacing production COSMIC widgets solely for tests would create
a different tested UI and is not an acceptable shortcut.

Widget scope approved by the user on 2026-09-15:

1. Implement text-input nodes, stable identity/label, focus and editable-text
   actions against the existing pinned custom widget. Verify Unicode editing,
   read-only/disabled behavior, targeted actions and actual application updates.
2. Treat secure inputs as a separate security obligation: protected state,
   masked presentation and no secret-bearing labels, tree dumps or diagnostic
   artifacts. Keep artifact-redaction regressions and inspect real AT-SPI output.
3. Implement dropdown control/option semantics and targeted selection/focus
   behavior, including disabled items and popup ownership. Do not fold Wayland
   lifetime or general popup redesign into this work.
4. Keep each repair independently regression-tested and documented in the
   ledger. Prove the real normal/instrumented field interaction before
   converting the required cases or counting profiles as completed UI coverage.

Retain the same exact-pinned-fork approach, no upstream PR, no master upgrade,
no expanded shutdown quarantine and no coverage waiver. Approval for this
widget-support scope does not authorize those other changes. Preserve the
successful diagnostic separately and keep the required full
`keyboard_accessibility` case planned until actual acceptance passes.

## Implementation findings, 2026-09-15

The first text-input patch, now committed as `5a1938b3` in the pinned libcosmic
checkout, is based on `7a4912de43b6720a86f9c2e663758f8870261472`. It adds a named
TextInput/PasswordInput node, author identity, a stable text-run child, masked
protected content, disabled/read-only semantics and targeted value replacement.
Secure-input semantics remain protected when the visual reveal toggle is used.
It is integrated in the working app pin, not an end-to-end completion claim.

Three lower-layer constraints must be handled explicitly:

1. **Inline overlays have no accessibility publication path in pinned iced.**
   `iced/core/src/overlay.rs` exposes no accessibility-node method, and
   `iced/runtime/src/user_interface.rs::a11y_nodes` collects only the root widget
   tree. COSMIC dropdown options live in such an overlay (or a separate popup
   when explicitly configured). Adding nodes solely to the option widget will
   not publish an inline menu. Do not duplicate invisible options on the closed
   control or change popup ownership simply to make tests pass. A separately
   scoped iced overlay-forwarding repair was approved in the follow-up and is
   implemented locally as iced `7192a2dca` with six passing runtime regressions.
   See [fix 10](dependency-fix-ledger.md#10-publish-open-iced-overlay-trees-local-not-promoted).
   It is now integrated with libcosmic menu option nodes (fix 12). Exclusive
   accessibility-focus routing was subsequently approved separately (fix 11).
2. **Accessibility focus is not routed through iced's exclusive focus operation.**
   `iced/winit/src/lib.rs`, `Event::Accessibility`, still has a TODO for
   `Action::Focus` before forwarding the event. Local widget focus alone cannot
   guarantee that a previously focused button is unfocused. Before promoting
   the text-input focus action, route valid, enabled focus targets through the
   existing widget focus operation and test cross-widget focus exclusivity,
   stale/unknown targets and preservation of read-only state. This is another
   small iced integration requirement, not Wayland lifetime work.
3. **The pinned AccessKit Unix adapter does not implement EditableText.**
   In AccessKit `f0599ee`, `platforms/atspi-common/src/node.rs::interfaces`
   advertises Accessible, Action, Component, Selection, Text and numeric Value;
   there is no EditableText implementation. The runner's existing
   `EditableTextProxy::set_text_contents` therefore cannot edit these fields,
   even after libcosmic supports AccessKit `SetValue`. Prefer verified target
   focus plus normal keyboard replacement in the existing private compositor,
   with secret-free command arguments/logs and actual application assertions.
   This runner refinement is implemented with 82 passing runner tests; live
   verification is in progress. Do not claim a libcosmic
   value-action unit test proves Linux AT-SPI text editing, and do not upgrade
   or fork AccessKit without a separate decision.

Remaining validation includes full widget-tree/child-button traversal, exclusive
focus, dropdown options and selection, normal/instrumented real form runs,
AT-SPI and screenshot secret-artifact checks, and the exact-pin/quarantine
evidence review. Existing passing diagnostic artifacts remain on the old pin.

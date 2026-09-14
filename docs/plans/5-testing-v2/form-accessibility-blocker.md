# Form accessibility: missing widget support

2026-09-14, continued from `4-ui-testing` commit `0a52ec5`.
Status: tooltip forwarding repaired; full form accessibility requires a scope
decision. Testing V2 is not complete. No coverage threshold, required UI flow,
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

Recommended separate, explicitly approved scope before resuming full UI flows:

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
widget-support scope would not authorize those other changes. Until then,
preserve the successful diagnostic separately and keep the required full
`keyboard_accessibility` case planned.

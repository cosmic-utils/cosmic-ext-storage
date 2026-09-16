# Dependency fixes retained for future upstreaming

This is a patch inventory, not authorization to submit upstream PRs. The user
has prohibited a libcosmic PR. Keep each repair reviewable separately and recheck
upstream for equivalent fixes before proposing any submission or replacing pins.
No upstream-master upgrade or Wayland teardown/lifetime repair is included.

When upstreaming or retiring a repair: recheck the target revision and open
fixes; submit each behavioral change with its own regression and dependency
ordering; record the upstream PR/commit/release only after it actually exists.
Then compare the replacement's behavior and exact dependency graph, rerun the
normal/instrumented UI gates, and remove only patches genuinely superseded.
An upstream merge alone is not permission to move this app to unstable master.

## Pinned patch stack

The current focus work starts from libcosmic `ab3a7b9c58a632eb64f4478d1c95bf8764e1c9aa`
and its iced submodule `25c211d8b1c0f456fd327b65be5261311b1d7692`.
The icon submodule stays `5252095787cc96e2aed64604158f94e450703455`.
The first four rows below were already in that base; the button repair was
previously validated during Testing V2. They are documented here retrospectively,
not represented as newly implemented or freshly upstream-reviewed today.

| Repair | iced commit | libcosmic commit |
| --- | --- | --- |
| Publish widget trees | [a4a13a53d](https://github.com/stoorps/iced/commit/a4a13a53d) | [285b580f](https://github.com/stoorps/libcosmic/commit/285b580f) |
| Preserve the adapter root ID | [721fd9b15](https://github.com/stoorps/iced/commit/721fd9b15) | [6f496308](https://github.com/stoorps/libcosmic/commit/6f496308) |
| Wake the event loop for accessibility events | [5758902cf](https://github.com/stoorps/iced/commit/5758902cf) | [a1fa1058](https://github.com/stoorps/libcosmic/commit/a1fa1058) |
| Expose author-provided widget IDs | [25c211d8b](https://github.com/stoorps/iced/commit/25c211d8b) | [3d5fdb08](https://github.com/stoorps/libcosmic/commit/3d5fdb08) |
| Route button actions to their target | Not changed | [ab3a7b9c](https://github.com/stoorps/libcosmic/commit/ab3a7b9c) |
| Publish widget focus | [d446d617b](https://github.com/stoorps/iced/commit/d446d617b3c0ad8b3ffa8b8826df85ed14178830) | [9afc8819](https://github.com/stoorps/libcosmic/commit/9afc881977fea36a87a88e33bb7ebfe091f93f06) |
| Forward window focus to AccessKit | [a2d4097d3](https://github.com/stoorps/iced/commit/a2d4097d394a8be295b31fe4d731039fa605707e) | [fe1630e7](https://github.com/stoorps/libcosmic/commit/fe1630e7c3a2af47a812f50007ed5f95b3d1691b) |
| Preserve tooltip content accessibility | [b852a3354](https://github.com/stoorps/iced/commit/b852a3354aca786ca0e6d899fb36dc2d4fa62aaf) | [7a4912de](https://github.com/stoorps/libcosmic/commit/7a4912de43b6720a86f9c2e663758f8870261472) |

The libcosmic commits corresponding to iced fixes are submodule integrations;
the actual iced patches belong in iced review, not duplicated into libcosmic.
The initial bridge commit also changes the iced submodule URL to the fork.
Repairs 6–8 are published only on `codex/pinned-a11y-focus` in each `stoorps` fork.
They are separate iced commits on the previous exact submodule pin, with
corresponding separate libcosmic submodule bumps. No fork master or upstream branch changed;
no PR was opened. The app remains on `4-ui-testing`.
The focus integration changed exactly eighteen git source URLs, with no added/removed
packages or package-version changes. Its Cargo.lock SHA-256 was:
`a04ad48f45e2005a91fceb320b51e93929ffef7713f1904b822c3abb255150c8`.

## 1. Publish the widget accessibility tree

- **Symptom:** the adapter's initial window tree alone does not expose the
  application's controls to AT-SPI.
- **Scope:** `iced/winit/src/lib.rs`; make the adapters available to the render
  loop and send widget nodes attached to their window root on accessibility
  redraws. Accessibility-gated; no application storage/backend change.
- **Regression/evidence:** `a11y_update_attaches_widget_roots_to_the_window`;
  the app's capability check requires actual non-window/interactable nodes.
- **Upstream notes:** preserve adapter cleanup and multi-window ownership.
  This patch originally used window-root focus, which is corrected separately
  by repair 6; do not upstream that limitation as the intended focus behavior.

## 2. Retain the initial adapter root identity

- **Symptom:** generating a new root ID during activation disagrees with the
  adapter/window ID later used for updates.
- **Scope:** `iced/winit/src/a11y.rs` plus activation construction in `lib.rs`;
  pass and retain the assigned window ID instead of generating a second one.
- **Regression:** `initial_tree_uses_the_adapter_window_node_id` checks both
  the initial tree root and initial fallback focus ID.
- **Upstream notes:** depends on the bridge integration; initial window fallback
  remains correct before a widget tree/focused widget is available.

## 3. Wake winit for queued accessibility work

- **Symptom:** queued activation/action/deactivation work does not itself wake
  a sleeping event loop, so delivery may wait for unrelated input.
- **Scope:** `iced/winit/src/a11y.rs` and handler construction; retain the
  existing control-channel messages and wake the event loop after enqueueing.
- **Evidence:** real capability activation and AT-SPI action runs; the initial
  root regression is retained. No dedicated event-loop wake unit test is claimed.
- **Upstream notes:** review activation, action and deactivation together. This
  does not change worker/display lifetime, add retries, or swallow action errors.

## 4. Publish stable author IDs

- **Symptom:** selectors cannot retrieve a widget's custom ID through AT-SPI.
- **Scope:** `iced/accessibility/src/id.rs`, `iced/core/src/widget/text.rs`,
  `iced/widget/src/button.rs`; expose only author-provided custom identifiers,
  not synthetic unique/set IDs. No selector semantics or action routing change.
- **Evidence:** the capability test verifies `test.scenario` together with its
  exact scenario marker; runner selector tests distinguish IDs from labels.
- **Upstream notes:** keep stable author strings separate from AccessKit's
  numeric node identities. Review coverage of additional widget types separately;
  this commit does not claim universal author-ID publication.

## 5. Target only the requested COSMIC button

- **Symptom:** clicking one AT-SPI button activated unrelated buttons too.
- **Scope:** `libcosmic/src/widget/button/widget.rs`; require matching target ID
  and Click action before accessing state or publishing an application message.
  Disabled, unrelated and unsupported actions remain inert.
- **Regressions:** `accessibility_click_only_activates_the_target_button`,
  `accessibility_click_does_not_touch_an_unrelated_button_state`, and
  `disabled_buttons_and_unsupported_accessibility_actions_are_inert`. The three
  tests failed before the repair and passed after it in the prior execution.
- **Upstream notes:** independent COSMIC widget fix, not a Wayland repair.
  It deliberately does not implement the accessibility Focus action.
  See [the original diagnosis and upstream search](ui-action-routing-blocker.md).

## 6. Publish the actual focused widget (approved 2026-09-14)

- **Symptom:** widget-level AT-SPI focus assertions cannot succeed because
  every redraw reports the window as `TreeUpdate.focus`.
- **Scope:** only `iced/winit/src/lib.rs`. Reuse the existing read-only
  `operation::focusable::find_focused` through `operation::black_box` against
  the actual UI, convert its ID, and publish it only if the current accessibility
  tree contains the node. No focus mutation or second focus-tracking state.
- **Fallbacks:** no focused widget, a composite/set ID, or an absent/removed
  accessibility node falls back to the window; never publish a dangling focus ID.
- **Regressions:** `a11y_update_publishes_the_focused_widget`,
  `a11y_update_does_not_publish_a_removed_widget_as_focused`, and
  `a11y_focus_query_preserves_the_widgets_focus_state_and_id`. The latter uses
  real focus operations with fake states that panic if focus is mutated.
  Existing initial-root and tree-parenting tests are retained.
- **Red/green proof:** keeping the old window-only publication makes the two
  focused-widget tests fail (expected widget IDs, actual window ID 42), while
  the fallback tests pass. Restoring the repair passes all five iced_winit tests
  and all twenty libcosmic library tests under the same feature selection.
- **Upstream notes:** depends on the bridge API in this pinned fork; port the
  behavior, not blindly the patch context. Keep keyboard delivery, widget
  activation, dialog focus management, and Wayland teardown as separate concerns.
  See [the diagnosis and existing-fix audit](keyboard-focus-blocker.md).

## 7. Forward real window-focus transitions to AccessKit

- **Symptom:** repair 6 alone passes focused-ID unit tests, but the real AT-SPI
  tree still reports no focused control. The pinned runtime never calls
  AccessKit's `Adapter::process_event`; its Unix adapter only publishes widget
  focus when it also knows the containing window is focused.
- **Scope:** only `iced/winit/src/lib.rs`; forward existing core window
  Focused/Unfocused events to the matching adapter before the UI consumes them.
  Using converted core events covers both winit and the SCTK backend. Preserve
  gain/loss order and ignore unrelated events; do not invent a permanent
  focused state or change keyboard routing, compositor focus, or display lifetime.
- **Regression:** `a11y_window_focus_preserves_gain_loss_order_and_ignores_other_events`.
  All six iced_winit and twenty libcosmic tests pass after this second patch.
  The first patch's real probe failure is retained as `focus/keyboard-01.log`;
  its unit success was not misrepresented as full keyboard acceptance.
- **Upstream notes:** this is distinct from focused widget-ID publication.
  It forwards focus notifications only; existing omissions for other adapter
  window-event types (such as geometry changes) are not claimed fixed here.
  No Wayland worker/lifetime or popup focus policy is modified.

Dependency verification command (run from the libcosmic checkout so features
are enabled on the workspace package, not an out-of-workspace dependency):

```sh
cargo +1.95.0 test --locked -p libcosmic -p iced_winit --lib \
  --features libcosmic/a11y,libcosmic/tokio,libcosmic/winit,libcosmic/wayland,libcosmic/wgpu
```

The dependency checkout emits existing warnings; these results do not claim
warning-free upstream code. Red/green logs are under the app's ignored
`target/testing-v2-completion/focus/`. App integration and real keyboard evidence
must be recorded separately from these dependency unit-test results.

## 8. Preserve tooltip content accessibility (continued execution)

- **Symptom:** the real tree has no Create partition button, even though the
  application supplies its name and `partition.create` ID. The standard iced
  Tooltip wraps it but does not implement `a11y_nodes`, so the default empty
  tree hides all wrapped controls from assistive technology.
- **Existing-fix audit (2026-09-14):** searched open/historical pop-os/iced and
  pop-os/libcosmic PRs for tooltip accessibility/a11y. No matching fix appeared.
  Inspected current upstream `widget/src/tooltip.rs` and its recent path
  history: `operate` exists but `a11y_nodes` is still absent. The old
  `949c4edc` operation-forwarding change does not publish accessibility nodes.
  No upstream revision was adopted.
- **Scope:** only `iced/widget/src/tooltip.rs`; forward the content's existing
  layout (including offsets), own child state and cursor to `a11y_nodes`.
  Return its tree unchanged. No new IDs, hidden-hint nodes, input behavior,
  focus policy, tooltip timing/geometry, overlay policy or Wayland change.
- **Regression:** `tooltip_preserves_content_accessibility_without_exposing_hidden_hint`
  constructs a real iced button, state tree and layout with offsets. Checks
  full root/child equality, button role/name/author ID/action and omission of
  the hidden hint. Before the repair it fails with an empty tree; afterward
  it passes. All 2 iced_widget, 6 iced_winit and 20 libcosmic tests pass.
- **Evidence:** `target/testing-v2-completion/tooltip/{red,green}.log`; test
  command is the dependency command above with `-p iced_widget` additionally.
  Normal capability also passes (`tooltip/capability.log`). Real dialog-flow
  and final-pin quarantine/instrumented verification are tracked separately;
  this unit result alone is not full UI acceptance.
- **Upstream notes:** standard iced Tooltip is used by libcosmic's public
  helper; the separate Wayland tooltip already has its own forwarding and
  is not changed. This patch does not implement accessible tooltip-overlay
  descriptions or promise that every wrapper/widget has complete accessibility.

## 9. COSMIC custom text-input accessibility

2026-09-15: user-approved work is committed as libcosmic `5a1938b3` in
`src/widget/text_input/input.rs`, on the existing pinned fork branch.
The app's working pin includes it via `0708e2cdc64a1395effa6cf896ca3da51ba5be4b`;
normal/instrumented UI validation is in progress. Scope: named field nodes,
stable text-run identity, plain/Unicode values, always-protected password tree
content (including visual reveal), disabled/read-only semantics and targeted
value updates through the application's existing callback. The patch does not
modify validation rules or storage behavior.

Local validation: `cargo +1.95.0 test --locked -p libcosmic --lib --features
a11y,tokio,winit,wayland,wgpu` passes all 29 tests (nine new text-input
regressions). `cargo +1.95.0 check --locked -p libcosmic --no-default-features
--features tokio,winit,wayland,wgpu` also passes. Both run from the libcosmic
checkout and emit existing upstream warnings; this is not a warning-free
Clippy claim. Logs are retained in the app's ignored
`target/testing-v2-completion/form-a11y/` directory. Regressions cover real
Widget-trait node publication, stable text-run identity, Unicode/control-char
handling, secure and revealed-password masking, malformed/unsupported/wrong
targets, disabled/read-only fields, managed values, repeated focus callbacks
and empty password replacement. No real compositor run has used this draft.

The final patch also forwards input child-button nodes and operations, outside
the protected text subtree, and offers `accessible_name` without adding another
visible label. Two additional regressions verify child-button exposure and the
real focus operation's read-only preservation/exactly-once focus callback.
Focus support depends on the separately approved iced fix 11 below; do not
extract just the widget-local Focus handler when upstreaming.
Dropdown options additionally need the overlay path in fix 10. The pinned AccessKit adapter has no EditableText
interface; direct AccessKit value-action unit tests are not evidence that the
Linux runner can edit a field. See [the implementation findings and remaining
verification](form-accessibility-blocker.md#implementation-findings-2026-09-15).
These are separately scoped integration requirements, not Wayland destruction
repairs. No upstream PR or dependency upgrade is authorized by this entry.

## 10. Publish open iced overlay trees (local, not promoted)

- **Approval:** the user explicitly approved this additional overlay fix on
  2026-09-15. Accessibility-focus routing was subsequently approved separately
  and is documented as fix 11.
- **Commit:** local iced `7192a2dca5e4e41194b36ef4daf7fa734774ae53`, based on the
  existing pinned-fork `b852a3354aca786ca0e6d899fb36dc2d4fa62aaf`. Not pushed;
  initially no new libcosmic or app pin was committed for this repair.
  Follow-up: pushed with fix 11 on the existing fork branch, included by
  libcosmic `12bd0e28` and the app's working `0708e2cd` pin. The heading records
  its original local-only validation stage; real UI promotion is still pending.
- **Cause:** Overlay had no node-publication API, and UserInterface collected
  only the base widget tree. A menu's option nodes could not reach AccessKit
  through an inline overlay even if the widget implemented them correctly.
- **Scope:** add a feature-gated, empty-by-default `Overlay::a11y_nodes` hook;
  forward it through message Map and Group; collect open descendants in
  Nested; join base and current overlay trees in UserInterface. Preserve node
  IDs, descendant relationships, layout/virtual offsets and cursor input.
  Read current overlay state and calculate current layout instead of relying
  on a stale open/close/render cache. The existing event-routing machinery,
  message mapping and popup ownership are unchanged.
- **API integration:** `UserInterface::a11y_nodes` now takes `&mut self` and a
  renderer reference so it can construct current overlays/layouts. The sole
  runtime caller in iced/winit passes the window's existing renderer. This is
  an API consideration for other custom iced integrations when upstreaming;
  it is not a dependency version upgrade.
- **Red/green verification:** four initial runtime regressions failed on the
  old publication path (one base node instead of five, missing option/bounds).
  All six final regressions pass: grouped/mapped/nested publication without
  duplicate IDs; close/reopen identity; fresh bounds after layout changes;
  delivery to a published nested target with one mapped message and rejection
  of an unknown target; cursor/physical/virtual offset forwarding; and removal
  despite a stale render cache, including relayout. These use the real
  UserInterface, overlay wrappers and event loop update path with a null
  renderer and test widgets, not a live compositor or COSMIC Dropdown.
- **Broader local checks:** six iced-runtime, two iced-widget, six iced-winit
  and 29 libcosmic library tests pass (43 total). The no-a11y libcosmic build
  also passes. Existing dependency warnings remain; no clean-Clippy claim.
  Evidence is under ignored `target/testing-v2-completion/overlay-a11y/`
  (`red.log`, `green.log`, `without-a11y.log`).
- **Reproduction:** from the libcosmic checkout, run `cargo +1.95.0 test
  --locked -p libcosmic -p iced_runtime -p iced_widget -p iced_winit --lib
  --features libcosmic/a11y,libcosmic/tokio,libcosmic/winit,libcosmic/wayland,libcosmic/wgpu`.
  Select features on the workspace's libcosmic package; Cargo rejects explicit
  iced-runtime feature selection from this parent workspace. Adding iced-core
  to this test command also fails because its dev-dependencies belong to the
  nested workspace; runtime regressions exercise the changed core wrappers.
- **Not included:** libcosmic dropdown option/control semantics, text-input
  child-button forwarding, exclusive accessibility focus, AccessKit
  EditableText, tooltip-overlay descriptions, Wayland teardown, app pin or
  quarantine changes. No upstream PR. Keep these obligations explicit before
  promoting the fork or claiming full form/keyboard coverage.

## 11. Route accessibility focus exclusively

- **Approval and commits:** explicitly approved after fix 10; iced `d38647d7a`
  and libcosmic submodule integration `12bd0e28`, pushed only to the existing
  stoorps fork branches. No upstream PR or dependency upgrade.
- **Cause:** the runtime forwarded Focus to a widget without first applying
  exclusive focus. A text field could focus itself while a button stayed focused.
- **Scope:** validate a data-free Focus request on the root accessibility tree,
  require one present, enabled, visible node advertising Focus, and preflight
  exactly one real focusable widget with that numeric ID. Only then apply the
  existing exclusive focus operation in that window, request redraw and forward
  the event for widget callbacks. Invalid requests are not forwarded. Other
  action types, windows, popup ownership and Wayland lifetime are unchanged.
- **Tests:** three runtime regressions cover moving focus both directions,
  repeated focus, disabled/unadvertised/non-operable nodes and malformed,
  stale or non-Focus requests without clearing the previous focus. Actual
  COSMIC text-input operation tests separately verify read-only preservation
  and one application callback after runtime focus. Nine runtime tests total
  pass (six overlay plus three focus), alongside six iced-winit tests.

## 12. COSMIC dropdown controls and option selection

- **Commit:** libcosmic `0708e2cdc64a1395effa6cf896ca3da51ba5be4b`, on the pinned
  fork branch. App working pin updated; end-to-end validation in progress.
- **Scope:** publish a named ComboBox with author identity, value, expanded
  state and disabled semantics for an empty list; include it in real focus
  operations. Keyboard activation/selection uses the existing open/close flags
  and callbacks. Publish ListBox/options through both inline-overlay and popup
  wrappers, with selected state and layout bounds. Targeted accessible clicks
  select through the existing application callback and close path.
- **Identity/lifecycle:** per-menu IDs survive unchanged option lists; replacing
  or reordering the labels replaces option IDs. Requests to unknown, replaced
  or already-closed options do not select. The code does not replace popup
  creation/destruction or repair Wayland lifetime. The existing string-list API
  has no per-item-disabled configuration; all its nonempty entries are selectable.
  Empty-control disabled behavior is tested, not an invented disabled-item API.
- **Tests:** seven new libcosmic regressions cover actual control/option nodes,
  names/IDs/value/bounds/selection, empty controls, duplicate labels with distinct
  IDs, changed-list invalidation, targeted selection and popup-close callbacks,
  stale/unsupported requests, and focused keyboard boundaries/open/close/Tab.
  All 38 libcosmic library tests pass, plus nine runtime, two iced-widget and six
  iced-winit tests (55 total). The no-a11y build also passes, with upstream warnings.
- **App-only integration:** create-wizard fields receive stable IDs, numeric
  fields receive accessible names without duplicated visible labels, and the
  filesystem dropdown receives a stable name/ID. The runner replaces the absent
  Unix EditableText path with Component focus, exclusive-focus observation,
  normal keyboard input over stdin and another focus check. It rejects disabled,
  non-editable or mismatched secret/ordinary targets and control characters.
  Protected contents never enter argv/helper logs. Plain-text assertions use
  the real AT-SPI Text interface; such assertions on password fields are rejected.
  Eleven new rstest runner cases cover the input checks/redacted diagnostics;
  all 82 runner tests and strict runner Clippy pass. Live form evidence is still
  required; these host results are not a completed UI-case or coverage claim.

## 13. Match named COSMIC buttons by numeric accessibility identity

- **Commit:** libcosmic `2ca5a4174bb76a742ef5d6d09d056bd9cc9b25c6` on the
  existing pinned fork branch. No iced or Wayland lifetime change.
- **Live cause:** form probe `form_accessibility_probe-12-1789496246689414972`
  could select the disk, but accessible activation of Create Partition did
  nothing. The button had a custom author ID; AccessKit reconstructs a numeric
  widget ID. Comparing the two enum representations directly rejects the same
  underlying node. Earlier button regressions covered only generated IDs.
- **Scope:** publish the custom author ID and compare numeric identities of the
  button, event target and request target. Reject requests carrying unexpected
  data; keep the existing callback and disabled-button checks.
- **Red/green evidence:** the regression now uses a named button and the real
  adapter's numeric reconstruction. It fails before the repair (one failure,
  two passes) and all 38 libcosmic library tests pass afterward. Logs:
  `/tmp/cosmic-named-button-red.log` and `/tmp/cosmic-named-button-green.log`.
  The app pin is updated. Live rerun
  `form_accessibility_probe-12-1789496758145541032` now activates Create
  Partition and exposes `create.name`; its next focus observation failed on
  a retired AT-SPI object. This proves button delivery, not the complete form.

## Focus integration evidence and exact quarantine rebind

Evidence root: ignored `target/testing-v2-completion/focus/`. These are local
container/host results, not a claim of hosted CI or completion of Testing V2.

- `keyboard-01.log`: repair 6 alone still lacks real focused-widget state;
  this exposed the separate adapter window-focus omission in repair 7.
- `keyboard-02.log`: both repairs publish an actually focused header button.
  The probe's assumption that the disk was the first Tab target was wrong;
  the exact named assertion correctly fails. Do not remove that failure or
  mislabel it as a dependency regression.
- `keyboard-03.log`: the corrected 13-step probe passes all steps and shuts
  down cleanly. Artifact: `ui-artifacts/executed/keyboard_accessibility-12-1789417402926050845`.
  It observes Volume → Usage → Keyboard Scenario Disk, then Shift+Tab → Usage.
  No accessible click or highlighted/selected-state substitute is used.
- [The exact probe](keyboard-focus-probe.toml), SHA-256
  `3af96af97652d0d439d00150dcc02ff14737ef6c122b8ec63d37b30e48c3ecc3`,
  is archived in these docs, outside the required-case directory. To reproduce,
  temporarily use its contents for `tests/ui/cases/keyboard_accessibility.toml`,
  run `just ui-e2e-case tests/ui/cases/keyboard_accessibility.toml`, and restore
  the planned case afterward. Keep its original case ID/hash for evidence
  comparison. This diagnostic does not complete the required dialog flow.
- `host-final.log`: on the final `fe1630e7` pin, workspace/all-feature Nextest
  passes 290 tests with 36 explicitly ignored native/probe entries.
  `python-final.log`: 32 discovered, 30 pass and two container-only checks
  explicitly skip on the host. `capability-rebound.log` additionally passes
  the normal capability run and the UI container shell-contract check.

Before rebinding, `reload-new-pin-unbound.log` deliberately retains the failure
on the new pin with the old lock hash. All nine functional steps pass, then
the exact known post-close SIGSEGV is observed in artifact
`ui-artifacts/executed/live_scenario_reload-12-1789417336212490150`:
`wl_proxy_destroy` in libwayland-client → wayland backend `ConnectionState`
drop → `WaylandSource<...SctkState>` drop. Its debugger JSON SHA-256 is
`134509fa88aad15184b6d7a53f7614b1a5175c008c6ce2837e834d9bb4d75e99`.
The old quarantine correctly does not accept the changed lockfile.

The only policy-value change is Cargo.lock SHA-256, from
`11070a571c8806d59f456d1edbeb65d428b28bbd256e6c28a3d3e7f4256ced0d`
to `a04ad48f45e2005a91fceb320b51e93929ffef7713f1904b822c3abb255150c8`.
Case hash `a2233684796ffdff2a02e97e0f57bb48f857a1c01b0dceeef0e33d537407bd90`,
environment hash `66c8e39ffdc0ab59f8c7741e8ea3f67b3b8e3d5adc58fc963eb1d28394484017`,
ordered stack matcher, owner and 2026-10-13 expiry are unchanged. The exception
still covers only `live_scenario_reload`, never this keyboard diagnostic.
Missing evidence, earlier crashes, other stacks and timeouts still fail.

Fresh post-rebind execution:

| Run | Result | Artifact directory under `ui-artifacts/executed/` |
| --- | --- | --- |
| Normal reload | All 9 steps pass; known shutdown warning | `live_scenario_reload-12-1789417802979380058` |
| Instrumented reload | All 9 steps pass; known shutdown warning | `live_scenario_reload-12-1789417832762975314` |
| Instrumented focus diagnostic | All 13 steps pass; clean shutdown | `keyboard_accessibility-12-1789417873715403128` |

Logs: `reload-rebound.log`, `reload-instrumented.log`,
`keyboard-instrumented.log`. The existing `collect_ui` validation accepts both
instrumented runs: two nonempty profiles (app and runner), two matching ELF
hashes, exact case/scenario/input provenance and acknowledged pre-close flush.
The accepted copies are in `focus/verified-reload` and `focus/verified-keyboard`;
`reload-provenance.log` and `keyboard-provenance.log` record validation.
Each pair also merges successfully with the pinned `llvm-profdata` and renders
workspace-scoped mappings with the matching `llvm-cov`, with no mapping warnings
(`profile-mapping-verification.log` and each copy's `mapping-check.txt`). These
are focused-run checks, not the combined project coverage/threshold gate.

Final strict workspace/all-target/all-feature Clippy, formatting and diff checks
pass. The mandatory validator accepts all 90 named tests and still reports
the eight-flow inventory as planned, not executed (`required-final.log`). The
archived diagnostic's temporary replacement of the keyboard inventory was
restored; the normal UI image was restored as the default local tag. No new
mandatory-case success, screenshot approval or final-head CI is claimed.

For instrumented reproduction, build the same UI Containerfile with
`--build-arg UI_COVERAGE=1` and the normal required `VERGEN_GIT_SHA` /
`VERGEN_GIT_COMMIT_DATE` arguments, tagged `cosmic-storage-ui-e2e:local`, then
run `UI_COVERAGE=1 bash tools/ui-testing/run-case.sh <case-path>`.
Do not use `just ui-e2e-case` for this step: it rebuilds the normal image.
Functional assertions, pre-close coverage checkpoint, profile/ELF transport
and shutdown-policy acceptance must each be verified. Do not claim a new
aggregate percentage from these focused runs.

## Tooltip integration and exact revalidation

Fix 8 updates the app from libcosmic `fe1630e7` to
`7a4912de43b6720a86f9c2e663758f8870261472` (iced `b852a3354`). Cargo.lock again
changes only eighteen git source URLs; no package/version change or unrelated
submodule update. Its SHA-256 is now
`3666c9738e009f594a10b5fe3c71575eeddffd534c64db90124869cf688b7476`.

Before rebind, reload ran all nine steps, then failed the old lock binding as
required: `live_scenario_reload-12-1789418816905834692`. Its post-close SIGSEGV
contains the unchanged ordered libwayland proxy → backend ConnectionState →
iced WaylandSource teardown stack. Debugger SHA-256 is still
`134509fa88aad15184b6d7a53f7614b1a5175c008c6ce2837e834d9bb4d75e99`.
Only the quarantine's Cargo.lock hash moves from `a04ad48f...` to `3666c973...`;
case, environment, signature, owner and October 13 expiry are unchanged.
There is no new-case allowance. This is not a teardown repair.

Evidence root: `target/testing-v2-completion/tooltip/` (ignored).

| Run | Result | Artifact directory under `ui-artifacts/executed/` |
| --- | --- | --- |
| Normal keyboard dialog diagnostic | 35 steps pass; clean shutdown | `keyboard_accessibility-12-1789418812567602309` |
| Normal rebound reload | 9 steps pass; clean shutdown | `live_scenario_reload-12-1789419004630507314` |
| Instrumented reload | 9 steps pass; existing shutdown warning | `live_scenario_reload-12-1789419125107298393` |
| Instrumented keyboard dialog diagnostic | 35 steps pass; clean shutdown | `keyboard_accessibility-12-1789419156750270318` |

The [exact dialog probe](keyboard-dialog-probe.toml) has SHA-256
`f4cb4466ad9af6c8a88f65da7f2959271dc540eb112158474a66b23780e600c9`.
It is archived outside the required-case directory, whose planned keyboard
inventory is restored afterward. It checks opening/cancellation and unchanged
storage state, not the full required disabled-reason/submission flow. See the
[new form-widget blocker](form-accessibility-blocker.md). Use the same temporary
case replacement/reproduction procedure above, not a second runner.

The normal capability and UI shell-contract check pass. Existing `collect_ui`
accepts two profiles and two matching binaries for each instrumented run,
including exact program/input provenance and acknowledged pre-close flush.
Pinned LLVM merges both profile pairs and reports first-party mappings without
warnings (`profile-verification.log`, `verified-{reload,keyboard}/mapping-check.txt`).
The default local image tag is restored to the normal image. No complete
coverage percentage, final CI, mandatory keyboard-case success or visual
approval is claimed by this diagnostic batch.

## App-only fixes and exclusions

- Encryption-options visual reveal previously replaced `secure_input` with
  an ordinary text input. Keep `secure_input` and vary its existing `hidden`
  parameter instead, so the accessibility role and masked content remain
  protected. Two rstest cases exercise the actual production field builder
  and published widget tree for hidden/revealed states: revealed fails before
  the fix; both pass after (`/tmp/cosmic-secret-reveal-{red,green}.log`). This
  app-only fix does not claim live secret-artifact or complete LUKS coverage.
- Test-only scenario composition now accepts bounded secret pairs on explicit
  `--scenario-secrets-stdin`, through the same runtime constructor with or
  without control-server support. Values are not arguments, environment,
  fixture data or control commands. Duplicate/unsafe IDs, malformed/control/
  oversized values fail with a generic message. Eight parser regressions pass;
  the runner's ephemeral-value transport and live LUKS case are still pending.

- Continued form/reload validation exposed an observation race in the runner:
  accessible properties wrap `UnknownObject` as `zbus::Error::FDO`, while
  the existing retired-node handler recognized only raw method errors. Four
  rstest cases cover direct/wrapped errors and unrelated failures (two red
  before, all green after; 86 runner tests total). Match typed errors, never
  error text. Only tree observations restart after an accessibility signal
  within the original deadline; actions are not retried. Evidence:
  `/tmp/cosmic-retired-node-{red,green}.log`. This is not a dependency fix.

- App commit `9924e1b`: short owned `/tmp/cs-ui-*` runtime directories fix Sway
  IPC path overflow for long case/artifact names. This belongs to our harness,
  not an iced/libcosmic upstream patch. Two rstest tests bind real Unix sockets
  and check privacy/cleanup/evidence preservation.
- Keyboard cases now retain one owned `wtype -s 180000` virtual keyboard so
  per-key wtype clients do not constitute the entire keyboard-device lifetime.
  This models an attached desktop keyboard; it is not a readiness sleep or an
  action retry. Startup waits for Sway's actual keyboard-device inventory,
  each step checks child liveness, and the case watchdog/Drop bound and clean
  up the child. Non-keyboard cases (including reload) do not start it.
  `keyboard_keeper_is_reaped_and_early_exit_is_rejected` verifies failure and
  kill/reap behavior. This belongs to our runner, not an upstream iced patch.
- The iced Wayland destruction race is **not fixed**. Its expiring,
  exact-signature shutdown quarantine remains a test policy, not an upstream
  patch or clean-shutdown claim. No stashed lifecycle experiment is included.
- Test-only LLVM checkpoints, Bash wrapper extraction and rstest adoption are
  app/testing infrastructure work, not dependencies to mix into upstream PRs.
- The ten create-dialog navigation validation cases and required-name support
  for standard Cargo library tests are app/test infrastructure only. They
  change no storage algorithm or dependency API; keep them out of an upstream
  tooltip patch. See the continued execution record for exact discovery and
  stale-selection checks.
- Required-test validation now rejects ignored tests using a separate real
  `--ignored --list` discovery. Ordinary libtest output does not annotate
  ignored entries; the previous single listing could accept them. Missing,
  duplicate and stale names still fail, and failed ignored discovery fails
  closed. Python regression coverage plus a real ignored fixture-helper check
  verifies this app-only checker fix; it is unrelated to libcosmic upstreaming.

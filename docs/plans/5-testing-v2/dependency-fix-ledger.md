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

The libcosmic commits corresponding to iced fixes are submodule integrations;
the actual iced patches belong in iced review, not duplicated into libcosmic.
The initial bridge commit also changes the iced submodule URL to the fork.
Repairs 6–7 are published only on `codex/pinned-a11y-focus` in each `stoorps` fork.
They are two separate iced commits on the previous exact submodule pin, with
corresponding separate libcosmic submodule bumps. No fork master or upstream branch changed;
no PR was opened. The app remains on `4-ui-testing`.
The app lockfile changes exactly eighteen git source URLs, with no added/removed
packages or package-version changes. New Cargo.lock SHA-256:
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

## Integration evidence and exact quarantine rebind

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

## App-only fixes and exclusions

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

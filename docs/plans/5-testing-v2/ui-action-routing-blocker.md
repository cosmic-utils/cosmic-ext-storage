# Interactive execution blocker: COSMIC action routing

Status: button routing repaired on 2026-09-13; all nine real UI steps pass.
The intermittent pinned iced shutdown crash is now an explicit, narrow
functional-test quarantine, approved by the user. Testing V2 remains incomplete.

## Reproduction

```sh
just ui-e2e-case tests/ui/cases/live_scenario_reload.toml
```

The pinned image starts the actual app with the reload scenario and discovers
exactly one AT-SPI button named `Before`. The runner invokes its sole `click`
action once. This also opens Settings and Format Disk and changes the selected
tab, removing the expected sidebar button. The subsequent semantic assertion
correctly fails. All storage in this reproduction is simulated; no real disk
action was submitted.

Local evidence is under
`ui-artifacts/executed/live_scenario_reload-12-1789327560948161419/`:
`select_before-before.a11y.json`, `select_before.png`,
`before_remains_visible-before.a11y.json`, `final.png`, `control.json`,
and `execution.json`. The final rerun used image digest
`sha256:d4c3ea767425daf710960c7169e4c997d17a1fd7908a83f9e329e2767bdeff16`,
also recorded in `/tmp/v2-ui-reload-stable.log`. These local files are
diagnostic evidence, not uploaded final acceptance artifacts or approved goldens.

## Cause and next step

The pinned dependency is `stoorps/libcosmic` revision
`3d5fdb087534fb4944e64da2346f3bb673cc0215`. Its
[button update handler](https://github.com/stoorps/libcosmic/blob/3d5fdb087534fb4944e64da2346f3bb673cc0215/src/widget/button/widget.rs#L789)
accepts `_id` but handles `Event::A11y(event_id, ...)` without comparing IDs.
It publishes the button's action regardless of the requested target. Upstream
`pop-os/libcosmic` master was checked and still had that behaviour, so simply
updating the dependency does not resolve it.

## Pinned-base repair and upstream search

The user authorized a fork repair but explicitly prohibited a libcosmic PR
and an unvalidated upgrade to upstream master. GitHub has no published
libcosmic release; its newest tag, `v0.12` (2024-10-16), predates our pin.
Therefore **keep the project's pinned commit as the repair base**, including
its exact iced and icon submodules. Absence of a release is not permission
to upgrade to latest master.

On 2026-09-13 the upstream search covered all 23 open PRs' changed-file lists
and all-state accessibility/AccessKit searches. The relevant open changes,
[#1425](https://github.com/pop-os/libcosmic/pull/1425) and
[#1411](https://github.com/pop-os/libcosmic/pull/1411), do not repair the target-ID
guard. Older merged [#664](https://github.com/pop-os/libcosmic/pull/664) and
[#1087](https://github.com/pop-os/libcosmic/pull/1087) do not supply it either.
No suitable existing fix was found. Recheck upstream when replacing this pin.

The app now pins fork commit
[`ab3a7b9c58a632eb64f4478d1c95bf8764e1c9aa`](https://github.com/stoorps/libcosmic/commit/ab3a7b9c58a632eb64f4478d1c95bf8764e1c9aa),
on `codex/pinned-a11y-button-routing`, exactly one commit after the previous
`3d5fdb08` pin. Only the button implementation and its regression tests change;
no submodule or upstream-base upgrade is included. All three new tests fail
before the fix and pass after it; all 20 library tests pass with
`cargo +1.95.0 test -p libcosmic --lib --features a11y,tokio,winit,wayland,wgpu`.
Tests check that a click activates only its target among two controls and that
unrelated, disabled, and unsupported actions neither access state nor consume
the event. Unsupported accessibility Focus stays inert, not newly implemented.

The latest-master investigation remains unpushed local work. No fork master,
iced branch, dependency cache, or vendored source was changed remotely, and no
libcosmic PR was opened.

## App verification and newly exposed shutdown blocker

With that exact fork pin, all 204 selected workspace Nextest tests pass
(32 explicitly ignored storage targets remain separate), strict all-targets/
all-features Clippy passes, formatting passes, and all 15 Python testing
infrastructure checks pass. Cargo.lock changes only the fork source revision;
no unrelated package or submodule versions change.

The actual UI rerun completed all nine steps, including `select_before`,
`before_remains_visible`, `after_is_visible`, and generation exactly one.
Evidence is in
`ui-artifacts/executed/live_scenario_reload-12-1789330715228357505/`, with image
`sha256:067f7b52f13bdaed2ee973f8523996ca276524de9b3f4c3cafd6110eea5cae04`.
That pre-quarantine run nevertheless exits failed: normal Wayland close causes
SIGSEGV. Do not blanket-ignore exit status or assume a crashed process flushed
its LLVM counters. The subsequent user-approved treatment is recorded below.

The matching core (host PID 1289160, 2026-09-13 21:18:41 BST) and image
libraries resolve the crashing stack through `wl_proxy_destroy`,
`wayland_backend::sys::client_impl::ConnectionState::drop`, and the SCTK
worker's calloop Wayland-source cleanup. The main thread is already in libc
exit. Diagnostic backtrace: `/tmp/cosmic-ui-pinned-core-resolved.log`.

The pinned iced revision remains `25c211d8b1c0f456fd327b65be5261311b1d7692`.
Its SCTK thread uses a foreign display and drops its join handle; its
`OwnedDisplayHandle` is not retained by that thread. Additionally,
`WaylandSpecific` declares the display owner before foreign connections,
so normal struct drop order can release the owner first. These are lifetime
hazards to address with coordinated worker shutdown and explicit drop order.
A local attempt to move the owner into the worker was rejected by the
compiler because `OwnedDisplayHandle` is not Send/Sync. That experiment is
stashed, not shipped; do not add an unsafe Send implementation as a shortcut.
A direct repair would require thread-lifecycle work in the companion iced
fork. The user instead chose to contain this failure in the testing layer;
the iced pin and source remain unchanged.

## Approved durable-test treatment (2026-09-13)

The runner now saves functional results, control responses, the final tree
and screenshot before requesting a normal close. The pinned container starts
the owned app under GDB and captures a structured crash stack. It needs no
host core collector, host display, extra ptrace capability, or action retry.
Only the diagnosed ordered Wayland teardown stack after functional completion
can match `tools/ui-testing/shutdown-quarantine.toml`: exact case, environment
and Cargo.lock hashes, named owner, and expiry on 2026-10-13 UTC. Missing
diagnostics, unknown stacks/signals, timeouts and earlier crashes still fail.
An owned process group and container teardown clean up failure paths.

Two real local runs exercised both outcomes:

- `live_scenario_reload-12-1789332625613667692`: all nine steps passed; GDB
  captured the actual `wl_proxy_destroy` / `ConnectionState` / SCTK calloop
  destruction stack. Result: `semantic_passed_with_known_shutdown_failure`,
  explicit warning and `shutdown_status: known_failure`.
- `live_scenario_reload-12-1789332807186622317`: all nine steps passed and
  the app exited zero normally. Result: `semantic_passed`, with
  `shutdown_status: clean`. The race is intermittent; this is not a repair.

Both directories are under `ui-artifacts/executed/`. The latter final-code
image is `sha256:77f9d736aac1a90345ab6055fb0c7821e76ef0ed575c8452c3dd3c1cf630ca2b`.
Functional and shutdown evidence are separate files; combined execution
evidence uses schema 2. CI executes this regression after its capability
check and uploads all evidence. This is not eight-case or visual acceptance.

Coverage remains independent. A Rust 1.95 instrumented probe under this same
container/debugger wrote a 456-byte profile on normal exit (LLVM successfully
decoded its three functions), but left a zero-byte profile on SIGSEGV.
Local probe evidence: `/tmp/ui-profile-probe.gtJUdI/`. This demonstrates the
failure mode, not profile completeness for the actual app. No manual flush or
coverage waiver was introduced. Missing/empty UI profiles remain a hard gate.

Do not substitute coordinate clicks, simulated reducer calls, a passing
inventory, or automatic golden acceptance for the required AT-SPI test.
Even after this blocker is repaired, seven further executed cases, pixel
comparisons, combined near-100% coverage, negative CI runs, and final review
remain required by the implementation plan.

# Interactive execution blocker: COSMIC action routing

Status: reproduced on 2026-09-13. Testing V2 is not complete.

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

The intended repair is a small, tested change to the already-used libcosmic
fork: ignore actions for other widget IDs, route supported actions only to the
matching widget, and verify focus/click behaviour with at least two controls.
Then pin that reviewed commit here and rerun the real UI case. Publishing a
change to that separate repository requires explicit user direction. No fork
branch, PR, dependency-cache modification, or vendored workaround has been made.

Do not substitute coordinate clicks, simulated reducer calls, a passing
inventory, or automatic golden acceptance for the required AT-SPI test.
Even after this blocker is repaired, seven further executed cases, pixel
comparisons, combined near-100% coverage, negative CI runs, and final review
remain required by the implementation plan.

# UI testing commands

`just ui-e2e` is the environment capability check, not the eight-case suite.
It proves the image-built application exposes an accessibility tree and that
the private Sway session can capture pixels and receive virtual keyboard input.

`just ui-e2e-case tests/ui/cases/live_scenario_reload.toml` runs a version-2
program against the actual application through AT-SPI. Fresh artifacts under
`ui-artifacts/executed` include actions completed, control responses, trees,
screenshots, and a non-zero failure. The workspace is mounted read-only; only
the artifact directory is writable. No host bus, home, network, or devices are
shared. A successful semantic result would be `semantic_passed`, not full visual
acceptance; baseline approval and comparison remain unimplemented.

Current status: the reload case reproduces a dependency action-routing bug;
see [the blocker record](../../docs/plans/5-testing-v2/ui-action-routing-blocker.md).
The other seven manifests are still inventory-only version 1 and are rejected
by the execution command. Required-test metadata remains explicitly planned.

Do not update goldens or declare coverage from the capability/inventory checks.

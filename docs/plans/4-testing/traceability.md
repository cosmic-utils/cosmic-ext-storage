# Scenario-test traceability matrix

This is the readable contract for the generated-and-reviewed planning matrix.
The frozen machine source is
[`tests/ui/traceability.toml`](../../../tests/ui/traceability.toml); it imports
the single method inventory from
[`contract-surface-v1.toml`](contract-surface-v1.toml), expands one row per
method, and adds the UI-flow rows below. Before a code phase begins, expansion
must be complete. The matrix prevents an adapter method, dialog branch, or
fixture from becoming an untestable implied requirement.

## Row contract

Each row contains exactly these columns:

| UI entry point / message | Contract method | `ScenarioOperation` | State DTO + default | Configured error(s) | Transition + events | Fixture | In-process test | E2E case |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |

Rows are required for every method of `BlockStorageBackend`, `BtrfsBackend`,
`LogicalTopologySource`, `LogicalOperations`, `NetworkDriveBackend`,
`FilesystemToolDiscovery`, `UsageOperations`, `ImageWorkflowOperations`, and
`DesktopServices`. A method that has no UI caller still has a row with
`UI entry point / message = none`; it must declare either a modelled transition
or a precise `Unsupported` response.

Every dialog has separate rows for opening, validation failure, cancel,
successful submit, configured backend error, pending/progress, completion, and
refresh. Every state-derived call must name the runtime field it uses, so the
runtime-injection audit can be mechanically checked.

## Completion rules

- The contract-surface generator emits the authoritative method inventory.
  `just ui-scenario-check` fails if the matrix omits, duplicates, or names a
  method outside that inventory.
- Every checked-in fixture is named by at least one matrix row. A fixture not
  used by an automated row must be explicitly marked `manual` with its owner
  and purpose.
- Each E2E case names the rows it covers. One case may cover multiple rows, but
  an assertion and checkpoint must be recorded for each branch claimed.
- The matrix is updated in the same commit as a contract, UI, fixture, or test
  change. It is not an after-the-fact report.

## Initial required coverage

The frozen Phase-0a matrix includes the following named fixtures and cases:

| Fixture | Required case(s) |
| --- | --- |
| `physical/partition-format.toml` | `physical_partition_format` |
| `physical/busy-unmount.toml` | `busy_unmount` |
| `physical/luks.toml` | `luks_unlock` |
| `logical/preflight.toml` | `logical_preflight_confirmation` |
| `network/mount.toml` | `network_mount` |
| `workflows/image-usage.toml` | `image_usage_progress` |
| `accessibility/keyboard.toml` | `keyboard_accessibility` |
| `reload/live.toml` | `live_scenario_reload` |

`just ui-plan-check` verifies that every operation in the contract inventory
has exactly one generated row, every declared UI operation occurs in a flow,
every fixture has an automated or explicitly owned-manual row, and every named
test resolves to `required-tests.toml`. The readable rendering is committed as
`ui-scenario traceability --format markdown`; it is reviewed alongside source
changes but is not a second authority.

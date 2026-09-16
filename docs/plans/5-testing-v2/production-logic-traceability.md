# Production business-logic audit

2026-09-16, checkpoint `95b4f66`. This is a caller-based migration ledger, not
an assertion that the migration is complete. No rendered tests are required
to exercise the state and operation boundaries below.

## Findings

- All five `AppModel::reduce_*_workflow` entry points and the `workflows` model
  field are gated by `test-backend`. Their only dispatch callers are the
  workflow harness. The corresponding normal UI handlers must become the
  tested path; merely removing their feature gates would not connect them.
- Production logical handlers already use `app.runtime.operations()` and real
  `LogicalState` confirmation/generation methods. Prefer that implementation
  over the simpler parallel logical reducer.
- Many volume/network/image handlers still call `*Client::new()` inside their
  async tasks. These resolve the selected global context. The clients already
  expose `with_operations`; pass the app's selected context explicitly rather
  than installing a fixture globally or serialising tests around a singleton.
- `UiDrive::new`, `load_drive_candidates` and `load_all_drives` also resolve
  that global context. Add explicit-context variants through the same model
  construction/refresh path; preserve compatibility callers until migrated.
- Pinned iced already exposes typed `runtime::task::into_stream`. Narrow unit
  tests can execute an actual handler's output task and inspect typed app
  messages without rendering or parsing task Debug output. Reject unsupported
  desktop/window actions and bound waits; do not simulate a second UI runtime.
  Use small shared production executors where that is clearer.

## Migration matrix

| Family | Actual production route/state/effect | Existing evidence and remaining action |
| --- | --- | --- |
| Create/format | `Message::VolumesMessage` → `VolumesControl::update` → `create::create_message`; real create/format dialogs; Partitions/Filesystems clients → `UpdateNav` or error dialog | Ten real navigation-validator cases exist. Add handler routing, form state, duplicate submission, invalid input, selected-adapter operation and resulting model assertions. Remove parallel `PhysicalIntent::FormatPartition` path after migration. |
| Encryption | Volume `UnlockMessage` → `encryption::unlock_message`; `UnlockEncryptedDialog`; Luks client → refresh or retry dialog | Current harness unlock test bypasses the real dialog and handler. Inject operations, verify wrong/right secret and retry state, secret projections and stale/missing target behavior through production. Migrate the parallel physical Unlock path. |
| Busy unmount | Volume mount/unmount handlers and root unmount-completion/busy-dialog handlers | Harness tests only a block adapter error and its own phase. Exercise actual busy-dialog/retry/completion behavior and state refresh. Migrate the parallel physical Unmount path. |
| Logical | `LogicalViewRequested`/candidate capture; `LogicalActionPrompted`; `LogicalPreflightLoaded`; `LogicalActionConfirmed`/`Execute`; `LogicalActionFinished`; `LogicalState` | Existing logical-state tests exercise the real source and generation methods; the parallel harness does not prove top-level message routing. Add real routing/preflight/cancel/identity/duplicate/stale tests and retire `workflows/logical.rs` once callers migrate. |
| Network | `handle_network_message`; actual wizard/editor/remote state; create/save/test/mount/unmount/status messages and completions | Separate reducer only models a subset. Inject RcloneClient operations, test actual validation, normalised options, retries and stale/changed selection handling. Preserve provider-specific behavior rather than adopting the reducer's simplified state. |
| Image/usage | `update/image/{dialogs,ops}` and root Usage handlers; image dialog, operation ID, `UsageTabState`; client/workflow operations | Parallel harness uses virtual progress and state not displayed by production. Characterise real progress/terminal/cancellation paths, explicit-context effects and stale IDs; keep unsupported adapter paths explicit. |
| Reload/selection | Subscription/device-event and root load/update navigation paths, `SidebarState`, `VolumesControl`, model refresh | Existing scenario atomicity tests are useful backend tests, not proof of production navigation. Test model refresh/selection/generation behavior and migrate the parallel reload facade without inventing a new UI reload path. |

## Test-quality corrections

- `verify_workflow_facade_contract` checks hard-coded route strings. Replace
  its claimed migration proof with actual message/effect/state assertions, then
  delete it and update the exact required-test inventory.
- Several `tests/ui_runtime_contract.rs` tests only parse command-line arguments
  despite names claiming runtime wiring. Preserve useful parser assertions under
  accurate names; add selected-runtime behavior tests for their real obligations.
- `tests/logical_ui_contract.rs::logical_dialogs_preserve_source_defaults`
  asserts a literal `"max"`; test the actual dialog/form defaults instead.
- Existing scenario adapter tests, native lab tests and genuine model tests
  remain useful. Do not delete them merely because their filename contains UI.

## Per-family completion evidence

Each migrated row must record exact tests, normal-build callers, typed effect
execution, application-state assertions, a local red/green production-path
mutation check, removed duplicate code and fresh coverage impact. Until those
exist, the row remains open. Do not count a new helper's tests as closing an
untested parent message route.

### Create/format slice — 2026-09-16

The real root `Message::VolumesMessage` route now passes the selected operations
context to the existing create/format handler. Drive refresh uses the same
context. Fifteen production-handler cases execute typed iced task outputs,
then deliver their actual completion messages to the normal update handler.
Tests reject window/runtime effects and global operation lookup; waits and
message delivery are bounded. No renderer or alternate application reducer runs.

The slice caught and fixed three concrete gaps:

1. Submission bypassed size/tool checks present in wizard navigation. Zero,
   oversized and unavailable-tool cases were red against production. Submission
   now revalidates, returns to the relevant step, and produces no backend effect.
   Filesystem choice is resolved from the validated selection, not a stale field.
2. Scenario `list_volumes` always returned an empty list. It now projects current
   partitions/filesystems and LUKS mapper relationships; refresh assertions test
   the actual resulting models, not a fabricated snapshot.
3. Filesystem formatting read a second, empty tool cache for selected adapters,
   while discovery used their real provider. Formatting now consults that same
   provider; the obsolete `StorageOperations.filesystem_tools` cache was removed.

Removed the parallel physical create intent/effect/validator and its integration
test. Required-test and flow traceability now point to the 13 production library
tests. Other workflow families remain in the old integration target until their
own migrations; the validator checks both targets and exact names.

Local routing fault check: temporarily replaced the production create dispatch
with `Task::none`; the success test failed at the real dialog's running assertion
(`/tmp/production-create-mutation.log`). Restored the route immediately. The
three original validation failures are in `/tmp/production-create-red.log`;
13 passing handler tests are in `/tmp/production-create-green.log`.

Cancellation before submission, duplicate submit, backend failure, missing
target, refresh state, and preserving a replacement idle dialog are covered.
Create/format now attach a fresh operation ID to the dialog and completion.
The normal root handler rejects success/error results from cancelled or replaced
dialogs and duplicate completions. Two additional cases execute old/new tasks
in controlled order and verify that an old result cannot replace a new running
dialog or regress its models. This does not claim backend cancellation or rollback:
a dispatched storage operation can finish; ordinary device refresh remains separate.
Fifteen cases pass in `/tmp/production-create-ordering.log`. Final slice checks:
74 app library tests and 13 remaining workflow integration tests pass; the full
test-backend suite, exact required-name execution, strict affected Clippy,
formatting and scenario-disabled/no-default-features build pass. Python contracts:
39 pass, two explicitly container-only skips. The added backend projection test
also verifies create/format/mount/unmount state, and LUKS projection is checked
across unlock/lock. Logs use `/tmp/production-{create-final,backend,named-tests,
create-clippy,no-scenario,python}.log`. Final workspace coverage/CI remain pending.
Other families remain open.

The frozen dependency pins are unchanged.

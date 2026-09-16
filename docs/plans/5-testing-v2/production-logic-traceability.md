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

### Logical slice — 2026-09-16

Eight tests now drive the real logical message handlers, confirmation dialog,
selected operation adapter and completion/refresh routes. They cover exact-once
confirmation, changed identity, cancelled preflight/confirmation, stale candidate
capture after navigation, stale topology resolution, failed execution without a
success refresh, and actual form validation/cancellation.

Fixed production gaps exposed by those tests:

- Candidate capture now carries a navigation generation and cannot restore a
  selection after the user left or selected a different candidate.
- A stale topology completion previously overwrote candidate resolution before
  checking its generation. The generation check now precedes all result mutation.
- Post-success incremental drive discovery/build now receives the app runtime
  explicitly, like the logical operation itself.

The global-context detector is stricter for handler tests: it fails immediately
even when a production handler would swallow the returned lookup error. That
change exposed the incremental-refresh lookup (`/tmp/production-logical-strict-red.log`).
Legacy harness self-tests still deliberately check the non-panicking rejection.
The guard is thread-local and non-Send; tests use Tokio's current-thread executor.

Removed `src/workflows/logical.rs`, its private harness dispatch/effects/state,
the two parallel logical tests, and the hard-coded five-name facade verifier.
These tracked files/history remain recoverable in Git. No production logical
state was replaced by the test reducer. Exact required names/flow mappings now
refer to the library-handler cases.

A deliberate local fault disabling the real confirmation executor caused the
new success test to fail on missing production pending state; it was restored
before validation (`/tmp/production-logical-mutation.log`). The normal run passed
82 library tests, 10 remaining integration workflows and 8 logical-state tests
(`/tmp/production-logical-final.log`). Remaining families and full coverage/CI
are still outstanding.

### Encryption slice — 2026-09-16

Four required production-handler cases cover wrong secret → error/retry → correct
secret → refreshed mapper child → lock, duplicate submit, cancel/missing target,
and a late failed unlock after cancellation. All use an owned scenario runtime
with the fixture secret supplied in memory, and prohibit global client lookup.
The old parallel physical Unlock intent/effect/test has been removed.

The tests exposed a Debug disclosure in `UnlockMessage` and
`UnlockEncryptedDialog`, now explicitly redacted, and a stale error completion
that reopened a cancelled secret dialog. The operation-ID completion boundary
is now shared by create/format/unlock and rejects that completion before state
changes. The selected operations context is passed through unlock, lock and
their refreshes; no global adapter installation is used.

Red/green evidence: `/tmp/production-encryption-{red,green}.log`. Deliberately
disabling the production Unlock dispatch failed the real retry-state assertion
(`/tmp/production-encryption-mutation.log`); restored immediately. These tests
prove message/dialog debug redaction, not a new serialized UI secret transport
or all possible backend log strings. Existing backend out-of-band secret tests
remain; rendered transport and its end-to-end claims remain deferred.

Final slice validation: 86 library tests, 9 remaining workflow integration tests,
exact required-name execution, strict app Clippy, formatting and scenario-disabled
build pass (`/tmp/production-encryption-{final,names,clippy,normal}.log`).

### Mount/unmount slice — 2026-09-16

Five production-route cases cover segment/child/sidebar busy errors, retry,
cancel, a successful mount/unmount round trip with real model refresh, missing
targets, and unsupported process termination remaining an explicit failure.
All operation/refresh clients carry the selected runtime; no host process is
terminated by these tests. Busy-dialog cancellation dismisses the dialog; it
does not promise cancellation of an already dispatched storage operation.

The tests reproduced an actionable-error bug: a typed `Busy` backend error was
ignored unless its English message contained "busy" or "in use". The client now
respects `StorageErrorKind::Busy` (retaining the old text fallback for compatibility).
Segment, child, sidebar, retry and kill/retry paths share one production unmount
executor. This also fixes sidebar handling of a failed `UnmountResult` as success.
Non-busy failures now surface an error dialog rather than disappearing into logs.
Protected-path checks and backend ownership/authorization remain unchanged.

Removed the remaining parallel `src/workflows/physical.rs`, harness dispatch,
state and busy test, plus the duplicate production retry/unmount implementations.
All deletions are tracked/recoverable in Git. Five handler tests pass; temporarily
disabling the real segment-unmount route made its case fail, then the route was
restored (`/tmp/production-mount-{red,green,mutation}.log`). Final slice validation
passed 91 library tests, eight remaining integration workflows, strict Clippy
and scenario-disabled builds; full coverage and final-head CI remain outstanding.

### Network slice — 2026-09-16

Eight real-handler cases now cover wizard/editor schema validation, duplicate
submission, conflict/retry, persisted option updates, selection after create,
mount/status/unmount, and controlled stale create/save/status completions.
Every Rclone client in the handler receives the selected runtime explicitly.
Removed the parallel network reducer, executor, harness methods/state and test.

The red tests reproduced duplicate saves and late create results replacing a
new form. Submitted forms now carry operation IDs. A successful create publishes
the returned configuration before selecting it: chaining LoadRemotes and Select
does not wait for the asynchronous load spawned by the first message. Per-mount
request IDs also prevent old status results overwriting newer mount results;
status failures stay visible as errors instead of becoming false "unmounted"
successes. These are app fixes, not dependency patches.

Evidence: `/tmp/production-network-red.log` (three real failures),
`/tmp/production-network-suite.log` (99 library and seven remaining integration
tests), and `/tmp/production-network-clippy.log` (strict Clippy). Disabling the
production WizardCreate route made the real state assertion fail in
`/tmp/production-network-mutation.log`; the intentional fault was restored.
Network delete/rename transactionality and overlapping list reloads remain
follow-up coverage gaps; these eight tests do not imply complete network coverage.

### Image/usage slice — 2026-09-16

Thirteen real-handler cases replace the parallel image/usage reducer and its
executor. Coverage includes validation/failure/retry, exactly-once startup,
progress, cancellation before and after startup, terminal cleanup, discarded
startup cleanup, stale progress/completion, usage wizard cancellation, scan and
delete state/results, stale scan/delete responses, and unsupported adapters.
The existing image/usage scenario now includes a disk and mounted filesystem so
the normal navigation and usage wizard can operate on declared fixture data.

Caller audit found that ImageClient bypassed the portable image adapter entirely.
The client now selects its image implementation from the composition root:
native runs keep the existing descriptor/file copy manager; supplied runtime
adapters use image-workflow operations and require synthetic asset references.
No fallback on adapter errors and no dependency changes. The old unused
UnavailableWorkflowAdapter placeholder was deleted. This is shared application
orchestration with distinct storage adapters, not another test-only reducer.

The production image subscription uses the app's operations context and the
same tested status-to-message function. Startup request IDs and completion
operation IDs prevent replacement dialogs from consuming old work. An early
cancel is retained until startup returns. Cancel errors are surfaced instead
of discarded. Restore now respects a failed/busy UnmountResult before copying.
Terminal operations are explicitly forgotten. The native lab test additionally
checks the native manager's terminal status through ImageClient.

Usage handlers now use the selected context, UUID scan identities (not colliding
millisecond timestamps), and delete request identities. Running portable scans
are awaited rather than misreported as completed-without-result. The waiting
test uses Tokio's paused clock/test utilities, not a real sleep or custom clock.
The older create-empty-image/attach dialogs and file picker still have separate
native I/O paths; they are not covered by these copy/usage claims. Native scan
progress streaming is also not established by a final-result test.

Initial image tests failed on forbidden global operations access
(`/tmp/production-image-red.log`); the thirteen replacement cases pass in
`/tmp/production-image-usage-final.log`. Full fresh coverage and native execution
are still required before declaring final validation complete.

Deliberately disabling the production image Start and usage WizardStartScan
routes failed nine real state assertions (`/tmp/production-image-usage-mutation.log`).
Both intentional faults were restored immediately; no fault was committed.

Exact required-name execution, strict app Clippy, formatting and the
scenario-disabled build pass (`/tmp/production-image-{names,clippy,normal}.log`).

### Reload/selection and harness retirement — 2026-09-16

Eight real-handler cases cover overlapping lists/builds/finish events, duplicate
or invalid device identities, atomic publication, failed-build preservation,
selection by stable identity, missing selections, and background refresh leaving
a running dialog intact. A real scenario-overlay change is observed through the
same selected-runtime event stream used by the production subscription, then
fed through the real load/navigation handlers. Invalid overlays preserve the
previous model/generation. Two independent apps remain isolated while the real
create and reload handlers execute under the strict no-global-lookup guard.

The initial three tests reproduced all three bugs: stale lists were accepted,
loading cleared the visible models, and background refresh closed an unrelated
running dialog (`/tmp/production-reload-red.log`). Load UUIDs and a staging set
now require every distinct build to complete successfully before publication.
Failures retain the last complete snapshot and expose a refresh error. Normal
complete updates invalidate old incremental results. Child selection is restored
only when its identity still exists. Device subscriptions now carry the selected
operations context rather than looking it up globally.

Removed the final parallel reload reducer, the whole WorkflowHarness and its
application state/public test facade, unused secrets/effect wrappers, old
integration target and fixture, and dead global load wrappers. The supported
workflow command and exact-name validator now execute the real library-handler
tests. Shared rstest scenario/scratch fixtures and scenario/backend tests remain.
Deleted files are recoverable in Git. The headless AppModel constructor is now
test-only and called `for_handler_test`.

Corrected parser tests that claimed runtime wiring, removed duplicate parser and
function-reference assertions, and replaced the literal logical-form assertion
with a real action/form round trip. Historical specs are not used as evidence of
these new tests. Eight reload tests and all 120 library tests passed; disabling
the actual LoadDrivesIncremental dispatch failed four cases
(`/tmp/production-reload-{final,suite,mutation}.log`), then the fault was restored.

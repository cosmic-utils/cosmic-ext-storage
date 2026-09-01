# Storage testing harness

`storage-testing` is a non-published workspace tool for the disposable
loop-fixture integration lab. It is not linked into the desktop application.

Use `just harness-nondestructive` for the required safe profile. It creates a
fresh marker-bearing artifact directory and writes `run-report.json` plus a
fixture ledger atomically. A selected case is always `Passed`, `Failed`, or
`Blocked`; the runner has no skip outcome.

The destructive profile is deliberately fail-closed. Run
`STORAGE_TESTING_ENABLE_DESTRUCTIVE=1 just harness` only from the documented
disposable VM. Fixture allocation and cleanup use the closed,
ledger-validated `FixtureCommandExecutor`; logical mutations under test remain
typed UDisks operations rather than shell commands.

# Testing V2 execution record

## Current status (2026-09-13)

Implementation is **not acceptance-complete**. The transport seam and all
native real-adapter families pass hosted CI. The old harness is retired;
merged host/lab/UI coverage at the specified thresholds, eight executed UI
cases, forced-failure PR evidence, and the final required coverage check
remain gated work.

The first executed interactive case is now blocked by a reproduced dependency
bug, not a missing display capability: the pinned COSMIC button handler ignores
the AT-SPI event's target ID and activates unrelated buttons. See the
[reproduction and required dependency decision](ui-action-routing-blocker.md).
No interactive or coverage acceptance is claimed.

## Harness retirement and hosted evidence

All seven jobs passed on commit `a11dfb32b4f4452a94316b31b4fa81b4c9d3a67e`
in [hosted run 34775793563](https://github.com/cosmic-utils/cosmic-ext-storage/actions/runs/34775793563),
including all 16 selected Storage lab cases. Following that green run, the
separate repository-administration change added `Storage lab` (GitHub App
integration 15368) to main ruleset 6374955, preserving its existing checks,
conditions, review requirements, and bypass actors. Coverage is not yet green
or required; no merge was performed.

The Phase-4 migration removes the old 17-file tool, workspace membership,
four recipes, and CI step together. Historical instructions and observations
are preserved verbatim in [the retirement archive](legacy-harness-history.md),
not relabelled as current execution. The 15 Python collector/lab-contract tests
pass and replace the obsolete safe-harness step in ordinary CI. Deleted source
remains recoverable from Git history.

The retirement commit `b6b31b4612f631d02e6d529261aba764465386e8` also passed
[all seven hosted jobs](https://github.com/cosmic-utils/cosmic-ext-storage/actions/runs/34776809016).
Its local native suite passed 16 selected cases in 73.691 seconds with image
`sha256:9a57a8cf63624423852f59d6d4bae0ccde3e53f36587e4d8865318e22f839eeb`.

## Coverage boundary and executed-UI preparation

- Moved 35 inline unit-test modules from 34 implementation files into each
  package's `tests/unit` tree using Rust syntax spans, preserving their logical
  modules/private access and every existing test name. The inventory comparison
  removed no tests. This excludes test source by path without hiding production.
- Moved the outer container lifecycle/evidence helpers from the integration
  test target into `storage-lab-tests/src/outer_bridge.rs`. The helpers themselves
  must remain in the 100% test-support denominator; only test bodies are excluded.
- The resulting workspace run passes 204 selected Nextest tests; 32 explicitly
  ignored targets still belong to the isolated lab. Strict all-targets/all-features
  Clippy passes. Fifteen Python coverage/lab-contract tests pass.
- Added a version-2 semantic case executor with real AT-SPI invoke/text actions,
  exact unique selectors, event-driven assertions, authenticated scenario-control
  calls, per-step trees/screenshots, a watchdog, and normal application shutdown
  for eventual profile flushing. It cannot approve visual baselines. Only the
  reload manifest has been migrated; the other seven cases remain planned.
- The UI run exposed a debug-image packaging bug: dependency localizations lived
  only in the build cache. `ui-test` now enables `rust-embed/debug-embed`, embedding
  those resources into the actual image-built binary without a runtime cache mount.
- Drain a signal-only accessibility stream concurrently with tree RPCs. An
  undrained all-message stream fills zbus's bounded queue and deadlocks tree
  queries. Re-rendering may retire nodes during observation: re-query only after
  an object event and only for the exact UnknownObject error, with a deadline.
  Never retry a UI action to hide a failure.
- Scenario event subscriptions are now live, independently cursor-based streams.
  Valid reload publishes one typed Refresh and advances the live generation once;
  an overlay revision cannot cause a second increment or roll back the live clock.
  Real backend and application-workflow regression tests cover these invariants.
- New UI execution artifacts use fresh directories; an existing user-supplied
  artifact path is no longer recursively deleted by the capability runner.

## Executed local evidence

`STORAGE_LAB=1 just test-lab` passed **15 tests, zero skips**, with Nextest run
`3d145fa1-0818-43cf-bb45-020699dcf86f` in 56.451 seconds, against image
`sha256:7fb1c537dbec971d7b07d5185b13578110ac062987698693f6941a6cecfdf345`.
The suite independently checked no lab-backed loops remained after every case.
It covers the native family targets in `crates/storage-lab-tests/tests` plus
the baked root application's `tests/storage_lab.rs`; exact selections and
per-case evidence are retained in `target/storage-lab-artifacts`.

The expanded run subsequently passed **16 selected bridge tests** in 61.726
seconds, including real Btrfs member add/remove and deliberate inner-failure
propagation. One non-ignored bridge unit test was excluded by this lab-only
selection and is exercised by the default workspace suite; no required inner
case was skipped. This also verifies the kernel-metadata payload fix and exact
member rescans. Hosted execution remains a separate gate.

The workspace host tests and strict Clippy checks also passed during this
iteration. Subsequent changes need their own final run and hosted evidence.

## Lessons that must survive migration

- Test the actual all-features workspace configuration. zbus's peer regression
  must use Tokio Unix streams when feature unification enables its Tokio
  transport; the standalone configuration alone did not catch that mismatch.
- Never hide Cargo diagnostics behind JSON redirection. On build failure,
  print rendered compiler messages before returning the failing status.
- The source image excludes `.git`. Pass verified commit metadata explicitly
  when compiling the root app, which needs it for its settings view.
- Pin exact snapshot package versions (Bookworm's selected `jq` is `1.6-2.1`).
  The Btrfs shared library is a runtime dependency, not just a builder library.
- Partition API results are device paths. Resolve them to D-Bus object paths
  on the explicitly supplied transport before invoking mutation methods.
  Discovery must read actual partition name/type/flags, not UI placeholders.
- Keep D-Bus method diagnostics: the upstream convenience error can discard
  the daemon's message. A raw call on the typed proxy preserves that evidence.
- UDisks volume groups have no `LogicalVolumes`/`PhysicalVolumes` properties.
  Enumerate child LV/PV interfaces and their `VolumeGroup` back-references.
  Group deletion/removal also takes an explicit positional wipe boolean.
- Metadata-0.90 MD arrays do not preserve the requested human-readable name.
  Identify them by owned members; never guess a global MD device number.
- Container udev visibility does not ensure device nodes exist. Materialize
  only ledger-owned descendants, with actual sysfs major/minor numbers.
  MD membership can be temporarily incomplete during assembly/stop; defer
  alias creation without relaxing exact-member ownership checks.
- A node-worker failure must not short-circuit all cleanup. Join every worker,
  recover its created-node list, clean independently reverified resources,
  and still report the original error. Missing MD nodes must not prevent
  rollback of a kernel array whose members are all owned by this fixture.
- An adapter reply may precede its discovery update. Poll the specific
  postcondition with a deadline; do not rerun a destructive operation.
- UDisks format can auto-open a LUKS mapper before returning. Rollback must
  discover owned dependent mappings even when setup fails before registration.
- Hosted run `34773799230` exposed a transient busy mapper during that unwind:
  `dmsetup remove` returned `Device or resource busy`, leaving a loop and
  correctly failing subsequent leak checks. Cleanup now retries only this
  error for at most five seconds, revalidating the mapper UUID and all owned
  ancestors before each attempt. It never uses force or deferred removal.
  The regression holds an actual mapper descriptor open for 300 ms during
  panic cleanup and requires the retry ledger entry. The complete local suite
  passed all 16 selected cases in 60.900 seconds (Nextest
  `759dd30e-e8a0-4409-8ce7-60ea7af61118`); hosted revalidation is still required.
- Run `34774662856` passed both hosted LUKS cases with the bounded retry.
  It then exposed a separate MD alias race: udev created the expected symlink
  between the worker's existence check and creation. Compare `read_link`
  targets (including dangling links), tolerate only the exact expected target,
  and never overwrite an existing conflicting alias or regular file. A native
  host unit test injects both same-target and conflicting-target races.
  Follow-up execution also demonstrated udev's relative `../md127` spelling:
  compare resolved targets, not the raw symlink text. A separate regression
  covers relative/dangling aliases and rejects a different device. MD sysfs
  entries can disappear during successful stop; a missing entry is not a
  node-worker failure and must never trigger an unverified mutation.
- An LVM object-manager snapshot may outlive a volume group deleted during
  discovery. On a property failure, query a fresh snapshot on the selected
  connection and skip only confirmed-absent groups. Preserve failures for
  still-present objects and failures of the confirming discovery call.
- Exercise encryption options, not just unlock/lock. The real daemon rejected
  a missing `passphrase-contents` argument even when no key was being stored;
  send an empty byte string for that case. Decode startup unlocking from the
  `noauto` option rather than always returning false. The exact daemon contract
  is in [UDisks 2.9.4's crypttab implementation](https://github.com/storaged-project/udisks/blob/udisks-2.9.4/src/udiskslinuxblock.c#L1617).
- Transfer `File` ownership into `OwnedFd`; `from_raw_fd(file.as_raw_fd())`
  returned an already-closed descriptor and risked closing a reused descriptor.
- Cancellation must stop copying between chunks, not merely change the final
  status after a full copy. Verify exact partial length and no post-cancel
  writes. Block-device metadata reports zero length; seek on the supplied
  descriptor to measure backup progress and restore its shared offset.
- Btrfs mounting/subvolume tests do not prove the UDisks member operations.
  libblockdev 2.28 consults on-disk libkmod metadata even for a built-in driver.
  Copy only the actual running kernel's bounded indexes/Btrfs module through
  Testcontainers, record hashes, and reject kernel-release mismatches. Do not
  replace the module check with an always-success helper or mount host paths.
- The full suite now compiles selected root-application cases. Its cold CI
  build needs a larger job budget than Phase 0's low-level-only probe; the
  native test timeout remains five minutes and retries remain disabled.

## Coverage honesty

The instrumented host/bridge/inner pipeline passed all 16 selected lab cases
in 176.616 seconds (Nextest `71690e39-62ca-4a68-a5b6-4f9bfc55a17b`). Its
coverage acceptance still fails: interactive UI profiles are absent, coverage
is far below the required thresholds, and later source edits invalidate that
run for final acceptance. The collector preserves matching ELFs/raw profiles,
exports independent build groups before unioning coverage, and hashes source,
profile, executable, and report evidence. Nextest stores JUnit relative to the
workspace's `target/nextest`, independently of Cargo's instrumented build path.

Running the required whole-workspace Nextest gate also exposed a pre-existing
cross-process temporary-directory collision in usage-scanner tests. Replacing
their process-local counter/custom cleanup with the existing `tempfile` crate
made all **205 selected workspace tests pass** with normal parallelism; 32
explicitly ignored storage targets remain the separate container suite, not
claims of host execution. Keep this multiprocess gate: ordinary `cargo test`
had not exposed the collision.

The previously reported raw counts included inline unit-test source in
production `.rs` files. That source-layout problem is now corrected, but those
old counts are stale and must not be described as final production-only coverage.
The current source also adds previously excluded bridge implementation to the
denominator. A fresh combined run and non-Rust test-support coverage remain
outstanding.

The intermediate **host-only** workspace report was 18.95% lines and 21.69%
functions; the root-only report was 10.63% lines and 11.93% functions. These
precede subsequent edits and omit real inner-lab/UI execution; they are not
current acceptance coverage. The exception manifest starts empty. A checker
is being built to reject missing lab/UI profiles, uncovered changed code,
threshold regressions, and expired/broad exceptions. No threshold was lowered.

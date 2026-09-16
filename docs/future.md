# Future work

## Reusable Rust UI-testing toolkit for the libcosmic community

- [ ] Investigate and extract a reusable testing library/runner from our UI
  testing infrastructure, after completing and stabilizing the planned UI cases.

### Investigate `rstest` for our own usage first

Investigation and local adoption are now recorded in the
[rstest execution record](plans/5-testing-v2/rstest-execution-record.md).
Community extraction remains future work; the criteria below informed adoption.

- Evaluate leveraging `rstest` for fixtures and parameterized cases in our
  existing Rust tests before designing equivalent abstractions ourselves.
- Prototype it on a small representative set of tests and compare readability,
  duplication, async support, and setup/teardown behavior with our current tests.
- Verify that adoption preserves isolated Testcontainers environments, owned
  resource cleanup, failure propagation, and explicit execution evidence. It
  must not introduce hidden retries or silently skip required cases.
- Record where it helps, where our existing approach is preferable, and which
  abstractions could carry over to a community toolkit. Adoption is a decision
  to validate, not a requirement to rewrite the whole suite.

### Community extraction

- Extract the existing Rust UI runner's reusable session lifecycle,
  accessibility selectors/actions/assertions, screenshots, and diagnostic
  artifacts into a library with an optional CLI.
- Replace hard-coded application identity, launch arguments, and scenario
  control with an app-supplied adapter for fixtures, virtual clocks, and state
  diagnostics.
- Provide a companion pinned container environment for consistent local and
  GitHub CI execution; a crate alone does not supply the compositor and system
  dependencies.
- Keep storage-specific fixtures, repository coverage thresholds, and our
  specific shutdown quarantine in this application. Consider the small GDB
  Python helper as an optional diagnostic component; converting all coverage
  orchestration from Python to Rust is not a prerequisite.
- Validate the extracted API against a second libcosmic application before
  publishing. Review packaging, documentation, maintenance responsibilities,
  and license compatibility as part of publication readiness.

This is future work, not a replacement for the remaining
[Testing V2 implementation and acceptance gates](plans/5-testing-v2/implementation-plan.md).

# AccessKit disabled-state translation blocker

2026-09-15, continuing on `4-ui-testing`. This is a newly identified AccessKit
boundary, not another iced or libcosmic forwarding fix. No AccessKit source,
pin or fork has been changed. The user subsequently chose to stop dependency
patching; the backport proposal is paused, not awaiting execution. Follow the
[non-rendered testing plan](non-rendered-testing-plan.md). The diagnosis below
is retained for future upstream compatibility review.

## Evidence

App working pin: libcosmic `2ca5a4174bb76a742ef5d6d09d056bd9cc9b25c6`,
iced `d38647d7a`; AccessKit remains `f0599ee`.

The [form diagnostic](form-accessibility-probe.toml) reaches 15 completed
steps in `ui-artifacts/executed/form_accessibility_probe-12-1789497112055077790`.
It activates the real named Create Partition button, locates the name field,
focuses it exclusively, enters and reads back `Data é🦀`, opens the inline
filesystem list, selects its ext4 option, observes closure, reopens and verifies
selected state, closes with Escape, advances to sizing and enters zero.
The scenario advertises only ext4: this verifies option delivery and retained
selection, not changing to another filesystem. An earlier Btrfs expectation
was invalid for this fixture and its failed evidence is retained.

The zero-size screenshot shows `0.00` and a disabled-looking Next button, but
the actual AT-SPI tree reports Next as enabled, sensitive and focusable. The
required disabled-state assertion fails. The application view disables Next
when size is zero; COSMIC publishes `Node::set_disabled()` when its callback
is absent. The loss occurs at the platform translation layer:

`platforms/atspi-common/src/node.rs`, `NodeWrapper::state`, at pinned `f0599ee`:
the Enabled/Sensitive insertion is the `else` arm of the read-only branch.
Buttons do not support the read-only state, so they take that arm even when
their AccessKit disabled flag is true. Read-only enabled text fields have the
inverse problem. This matches the existing upstream report exactly.

## Existing upstream repair

- [AccessKit PR #787](https://github.com/AccessKit/accesskit/pull/787) reported
  the exact disabled-button/read-only-field mapping defect; it was closed in
  favor of the maintainer's replacement, not rejected as an invalid diagnosis.
- [Maintainer PR #788](https://github.com/AccessKit/accesskit/pull/788) merged
  on 2026-09-01. Commit `ec349dcee8bdc8b3fd57c84a5487fdca4fe8b009`, merge
  `6ee0558b6315b3ef1594db24ce45a030ecac7cb5`.

Use #788, not the superseded #787 diff. The maintainer's actual patch gates the
entire read-only/enabled branch on `!is_disabled()` and otherwise preserves
the prior treatment of enabled read-only fields. Do not inadvertently include
#787's broader read-only behavior change in a claimed exact backport.

Recommendation: with explicit approval, backport that precise state-mapping
repair to our existing AccessKit revision and add regressions for enabled and
disabled buttons and enabled read-only/disabled text inputs. Adapt paths/API
only as necessary for the older pin. Preserve package versions and unrelated
submodules; do not adopt master or open an upstream PR. Rerun this diagnostic
normally and instrumented before claiming the blocker resolved.

The archived diagnostic retains the executed file's SHA-256
`c6fc2a26169a2463a40d7008f68bc05eefb6d381b077d123021adedd7b90babd`.
To reproduce, copy it to `tests/ui/cases/form_accessibility_probe.toml` and use
the existing `run-case.sh`; remove the temporary copy afterward. It remains
outside required-case discovery and is not keyboard-only acceptance evidence.

Do not weaken the test to accept an enabled disabled button, infer semantics
from screenshots, replay actions or silently expand the shutdown quarantine.
Other UI flows, coverage targets and human visual approval remain outstanding.

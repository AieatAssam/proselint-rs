# Migration progress

## Current state

- Immutable baseline: proselint `0.16.0`, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; the pinned Python 3.12.3 environment passed its integrity checks and all 351 upstream tests plus 15 subtests.
- Inventory: 124 registered rule entries over 106 unique paths (110 enabled by default, 14 disabled); see `rust/inventory.json`.
- Harness: independently verified; 14/14 harness tests, 23/23 comparator discriminators, and 2/2 harness-only live oracle checks pass.
- Rust currently records 13 implemented capabilities and 76 compatibility cases. The strict live gate passed 76/76 at source fingerprint `693d9474949706bc5b74d7bea48dcc0bf454181b742469f42b3ce636c7b7f1c0`; report: `rust/artifacts/strict-gate-professions-final-20261105.json`. Independent verification is in progress for this fingerprint.
- Six live cases for `rule.misc.false_plurals.misc` are specified against the unchanged pinned oracle, but that capability is not implemented or included in the accepted strict gate. Evidence: `rust/artifacts/false-plurals-misc-live-20261105T054532Z/false-plurals-misc-live-report.json`.
- Rust tests currently include 76 integration tests. The last stable strict run passed formatting, Clippy with warnings denied, locked tests and locked build. Locked dependencies are clap 4.6.7, serde 1.0.229 and serde_json 1.0.154; Rust toolchain is 1.99.1.
- The accepted capabilities cover selected-rule configurations and bounded CLI behavior. They do not establish default configuration or full Proselint parity. Most registered checks remain unimplemented; combined-rule behavior and broader Unicode case folding remain unverified.
- Custom agent roles were assigned as requested, but session model and reasoning-effort metadata are not exposed for independent routing audit.

## Ownership and next gate

The coordinator owns shared status and progress records. The oracle engineer owns pinned case specification; builders own assigned Rust implementation and tests; parity verifiers are read-only. Next, add failing Rust tests for the six `misc.false_plurals.misc` cases, implement against the pinned source, run cumulative live comparison, independently verify, update evidence, and push the milestone. Continue capability by capability; do not describe this partial port as full parity.

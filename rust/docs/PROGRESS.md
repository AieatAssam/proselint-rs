# Migration progress

## Current state

- Immutable baseline: proselint `0.16.0`, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; pinned Python 3.12.3 integrity checks and all 351 upstream tests plus 15 subtests passed.
- Inventory: 124 registered rule entries over 106 unique paths (110 enabled by default, 14 disabled); see `rust/inventory.json`.
- Harness: independently verified; 14/14 harness tests, 23/23 comparator discriminators, and 2/2 harness-only live oracle checks pass.
- Rust records 17 implemented capabilities and 101 integration tests. Recent typography rules `ellipsis`, `copyright`, and `sentence_spacing` each passed direct oracle cases and independent verification. Evidence is in `rust/artifacts/live-*-rust-20261105.json` and `rust/artifacts/independent-*`.
- The strict live gate passes all 101 compatibility cases across 17 accepted capabilities; report: `rust/artifacts/strict-gate-sentence-spacing-final-20261105.json` (source fingerprint `3149d8d195675dc3c8a67a3e4204f0b606ff2fe4d751fba4d2b0833dd93451b9`).
- Rust dependencies are current for toolchain 1.99.1: locked clap 4.6.7, serde 1.0.229 and serde_json 1.0.154. Crates.io check and `cargo update --dry-run` found zero compatible packages to update.
- Accepted capabilities cover selected-rule configurations and bounded CLI behavior; they do not establish default configuration or full Proselint parity. Most registered checks remain unimplemented; combined-rule behavior and broader Unicode case folding remain unverified.
- Custom agent roles were assigned as requested, but session model and reasoning-effort metadata are not exposed for independent routing audit.

## Ownership and next gate

The coordinator owns shared status and progress records. The oracle engineer owns pinned case specification; builders own assigned Rust implementation and tests; parity verifiers are read-only. Select the next capability from the pinned inventory and repeat live specification, behavioral RED, GREEN, cumulative live comparison, independent verification, recording, and incremental push. Do not describe this partial port as full parity.

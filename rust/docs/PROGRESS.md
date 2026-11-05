# Migration progress

## Current state

- Immutable baseline: proselint `0.16.0`, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; pinned Python 3.12.3 integrity checks and all 351 upstream tests plus 15 subtests passed.
- Inventory: 124 registered rule entries over 106 unique paths (110 enabled by default, 14 disabled); see `rust/inventory.json`.
- Harness: independently verified; 14/14 harness tests, 23/23 comparator discriminators, and 2/2 harness-only live oracle checks pass.
- Rust records 14 implemented capabilities and 82 integration tests. `rule.misc.false_plurals.misc` passed six new direct oracle cases and independent verification; its evidence is in `rust/artifacts/live-false-plurals-misc-rust-20261105.json` and `rust/artifacts/independent-false-plurals-20261105/`.
- The strict gate passes all 82 compatibility cases across 14 accepted capabilities; report: `rust/artifacts/strict-gate-false-plurals-misc-final-20261105.json` (source fingerprint `3462133416a2b3ae0d514d57fce45ee392f9cd529931c34a382e112c3bc4232b`).
- Rust dependencies are locked at clap 4.6.7, serde 1.0.229 and serde_json 1.0.154. A crates.io check and `cargo update --dry-run` found zero Rust 1.99-compatible packages to update. Rust toolchain: 1.99.1.
- The accepted capabilities cover selected-rule configurations and bounded CLI behavior. They do not establish default configuration or full Proselint parity. Most registered checks remain unimplemented; combined-rule behavior and broader Unicode case folding remain unverified.
- Custom agent roles were assigned as requested, but session model and reasoning-effort metadata are not exposed for independent routing audit.

## Ownership and next gate

The coordinator owns shared status and progress records. The oracle engineer owns pinned case specification; builders own assigned Rust implementation and tests; parity verifiers are read-only. Select the next capability from the pinned inventory and repeat live specification, behavioral RED, GREEN, cumulative live comparison, independent verification, recording, and incremental push. Do not describe this partial port as full parity.

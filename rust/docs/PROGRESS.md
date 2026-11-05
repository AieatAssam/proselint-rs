# Migration progress

## Current state

- `inventoried`: CLI, config/output, matching engine, tests and corpus provenance reviewed by three read-only scouts. The live pinned registry yielded 124 registered entries, 106 unique paths, 110 default-enabled and 14 default-disabled entries; see `rust/inventory.json`.
- `cli.json_no_findings`: independently verified and recorded, with 3 live CLI cases passing against the pinned Python oracle at frozen source fingerprint `0c246effde70e29fa7193efe9b6258d881c0fe20c3af043bf8bc4a533ad3922e`. Evidence: `rust/artifacts/live-cli-json-no-findings-20261105.json` and `rust/artifacts/independent-cli-json-no-findings/verification.json`.
- Oracle: immutable source snapshot and isolated Python 3.12.3 environment are provisioned; 98 loaded modules resolve inside the venv, dependency/package/executable hashes verify, and upstream pytest passed (351 tests plus 15 subtests; exit 0).
- Harness: `rust/harness/**` and `rust/oracle.lock.json` are independently verified on fingerprint `5817279dd9575a62a982da6d943fcbd26e8f31ff317b3626d71175afcdc6b38d`. Unit tests: 14/14; comparator discriminators: 23/23; live oracle self-check: 2/2 PASS with harness-only scope. Before the first capability was recorded, the strict gate and benchmark correctly returned BLOCKED with no accepted capabilities.
- Next specified capability: `rule.annotations.misc` (7 live oracle cases). All 10 capability cases passed real-oracle self-comparison at source fingerprint `bcbfdda37eaccf44c4210930e25f78f45c1c0ad823bae307f9f6da87b3d01269`; direct reference CLI probe passed all 12 manifest cases. The selected config was verified to enable only `annotations.misc`.
- Rust crate dependencies in `Cargo.lock`: clap 4.6.7, serde 1.0.229, serde_json 1.0.154; Rust toolchain 1.99.1.
- The accepted JSON CLI slice was tested only with an explicit configuration disabling all checks. General configuration and lint behavior are excluded from its parity claim.
- Custom-agent roles were used, but child model and effort metadata remain unavailable for independent routing audit.

## Environment

Start revision: `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`. The oracle uses Python 3.12.3 and uv 0.13.7. Rust 1.99.1, rustfmt and Clippy are installed under `rust/.cache`. Initial PATH lacked `uv`, `pytest`, and Cargo; no system tools were changed.

## Ownership

- Coordinator: shared docs/status/inventory and later Rust crate integration.
- Oracle engineer: `rust/harness/**`, `rust/oracle.lock.json`, oracle integrity and harness evidence.
- Read-only scouts: completed.

## Next gate

Add focused RED tests for `annotations.misc`, including Unicode word-boundary behavior and quote suppression. Implement the rule using pinned upstream semantics, then pass focused tests, cumulative live parity and independent verification before recording it.

# Migration progress

## Current state

- `inventoried`: CLI, config/output, matching engine, tests and corpus provenance reviewed by three read-only scouts. The live pinned registry yielded 124 registered entries, 106 unique paths, 110 default-enabled and 14 default-disabled entries; see `rust/inventory.json`.
- `cli.json_no_findings`: independently verified and recorded, with 3 live CLI cases passing against the pinned Python oracle at frozen source fingerprint `0c246effde70e29fa7193efe9b6258d881c0fe20c3af043bf8bc4a533ad3922e`. Evidence: `rust/artifacts/live-cli-json-no-findings-20261105.json` and `rust/artifacts/independent-cli-json-no-findings/verification.json`.
- `rule.annotations.misc`: independently verified and recorded, with 7 selected-config live cases passing at frozen fingerprint `f1644d864072cfca0a938357196a30f6e274779b2ec4255f53809bbfa9ce42dc`; cumulative verification passed all 10 accepted cases. Evidence: `rust/artifacts/live-annotations-misc-20261105.json` and `rust/artifacts/independent-annotations-misc/verification.json`.
- Oracle: immutable source snapshot and isolated Python 3.12.3 environment are provisioned; 98 loaded modules resolve inside the venv, dependency/package/executable hashes verify, and upstream pytest passed (351 tests plus 15 subtests; exit 0).
- Harness: `rust/harness/**` and `rust/oracle.lock.json` are independently verified on fingerprint `5817279dd9575a62a982da6d943fcbd26e8f31ff317b3626d71175afcdc6b38d`. Unit tests: 14/14; comparator discriminators: 23/23; live oracle self-check: 2/2 PASS with harness-only scope. Before the first capability was recorded, the strict gate and benchmark correctly returned BLOCKED with no accepted capabilities.
- Next work: specify multiline and file-input coordinate/newline cases against the live oracle. Existing annotations tests cover Unicode single-line offsets, boundaries, quote suppression, ordering, and selection of only `annotations.misc`; default configuration remains outside the claim.
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

Specify CLI coordinate and newline handling from live oracle behavior. Then add focused failing Rust tests, implement the smallest verified semantics, and pass cumulative live parity and independent verification before recording the capability.

# Migration progress

## Current state

- `inventoried`: CLI, config/output, matching engine, tests and corpus provenance reviewed by three read-only scouts. The live pinned registry yielded 124 registered entries, 106 unique paths, 110 default-enabled and 14 default-disabled entries; see `rust/inventory.json`.
- `specified`: no capability accepted or specified with live oracle outputs yet.
- Oracle: immutable source snapshot and isolated Python 3.12.3 environment are provisioned; import provenance passed and upstream pytest passed (exit 0). Final integrity and portability review is still in progress.
- Harness: assigned to the custom `oracle_engineer`; `compare.py --self-test` currently reports PASS for its mutation tests, but unit tests, live oracle self-comparison, report/gate validation, and independent review remain outstanding. This provisional result is not an accepted gate.
- Production Rust: not started, per protocol.
- Independent verifier: not yet assigned; will receive a frozen fingerprint after the first capability reaches green.

## Environment

Start revision: `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`. The oracle uses Python 3.12.3 and uv 0.13.7. Rust 1.99.1, rustfmt and Clippy are installed under `rust/.cache`. Initial PATH lacked `uv`, `pytest`, and Cargo; no system tools were changed.

## Ownership

- Coordinator: shared docs/status/inventory and later Rust crate integration.
- Oracle engineer: `rust/harness/**`, `rust/oracle.lock.json`, oracle integrity and harness evidence.
- Read-only scouts: completed.

## Next gate

Finish harness self-tests and actual oracle-versus-itself CLI comparison; obtain independent verifier evidence. Then specify and RED-test the first CLI/diagnostic capability before assigning the Rust builder.

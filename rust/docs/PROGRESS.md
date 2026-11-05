# Migration progress

## Current state

- `inventoried`: CLI, config/output, matching engine, tests and corpus provenance reviewed by three read-only scouts. The live pinned registry yielded 124 registered entries, 106 unique paths, 110 default-enabled and 14 default-disabled entries; see `rust/inventory.json`.
- `specified`: no capability accepted or specified with live oracle outputs yet.
- Oracle: immutable source snapshot and isolated Python 3.12.3 environment are provisioned; 98 loaded modules resolve inside the venv, dependency/package/executable hashes verify, and upstream pytest passed (351 tests plus 15 subtests; exit 0).
- Harness: `rust/harness/**` and `rust/oracle.lock.json` are independently verified on fingerprint `5817279dd9575a62a982da6d943fcbd26e8f31ff317b3626d71175afcdc6b38d`. Unit tests: 14/14; comparator discriminators: 23/23; live oracle self-check: 2/2 PASS with harness-only scope. The strict production gate and benchmark both correctly return BLOCKED while the accepted capability list is empty.
- Specified capabilities: `cli.json_no_findings` (3 live cases) and `rule.annotations.misc` (7 live cases). All 10 new cases passed real-oracle self-comparison at source fingerprint `bcbfdda37eaccf44c4210930e25f78f45c1c0ad823bae307f9f6da87b3d01269`; direct reference CLI probe passed all 12 manifest cases. The selected config was verified to enable only `annotations.misc`. This is specification evidence, not a Rust parity result.
- Production Rust: not started, per protocol.
- Independent verifier: not yet assigned; will receive a frozen fingerprint after the first capability reaches green.

## Environment

Start revision: `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`. The oracle uses Python 3.12.3 and uv 0.13.7. Rust 1.99.1, rustfmt and Clippy are installed under `rust/.cache`. Initial PATH lacked `uv`, `pytest`, and Cargo; no system tools were changed.

## Ownership

- Coordinator: shared docs/status/inventory and later Rust crate integration.
- Oracle engineer: `rust/harness/**`, `rust/oracle.lock.json`, oracle integrity and harness evidence.
- Read-only scouts: completed.

## Next gate

Build a minimal compiling Rust scaffold, add integration tests for `cli.json_no_findings`, and capture a behavioral RED. Then implement the native CLI/library path, compare against the live baseline, independently verify, and proceed to the failing `annotations.misc` tests. No production Rust capability is accepted yet.

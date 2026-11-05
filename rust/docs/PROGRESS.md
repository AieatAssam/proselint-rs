# Migration progress

## Current state

- `inventoried`: CLI, config/output, matching engine, tests and corpus provenance reviewed by three read-only scouts. The live pinned registry yielded 124 registered entries, 106 unique paths, 110 default-enabled and 14 default-disabled entries; see `rust/inventory.json`.
- `cli.json_no_findings`: independently verified and recorded, with 3 live CLI cases passing against the pinned Python oracle at frozen source fingerprint `0c246effde70e29fa7193efe9b6258d881c0fe20c3af043bf8bc4a533ad3922e`. Evidence: `rust/artifacts/live-cli-json-no-findings-20261105.json` and `rust/artifacts/independent-cli-json-no-findings/verification.json`.
- `rule.annotations.misc`: independently verified and recorded, with 7 selected-config live cases passing at frozen fingerprint `f1644d864072cfca0a938357196a30f6e274779b2ec4255f53809bbfa9ce42dc`; cumulative verification passed all 10 then-accepted cases. Evidence: `rust/artifacts/live-annotations-misc-20261105.json` and `rust/artifacts/independent-annotations-misc/verification.json`.
- `cli.coordinates`: independently verified and recorded at frozen fingerprint `b78fb0812d306959aa43f02471951084dfefe8788f0c4ff3a7b689ea786eaff6`; 6 new cases and 16 cumulative cases passed. Stdin CRLF offsets remain code-point based; file reads normalize newlines as Python does. Evidence: `rust/artifacts/live-cli-coordinates-20261105.json` and `rust/artifacts/independent-cli-coordinates/verification.json`.
- `rule.cliches.hell`: independently verified and recorded at frozen fingerprint `74b6959fe3247f8cab46eedb3a7775e0defc067b6cc860d297a1eb95158ac664`; 6 new and 22 cumulative cases passed. This is an ASCII case-insensitive unanchored phrase rule under selected config. Evidence: `rust/artifacts/live-cliches-hell-20261105.json` and `rust/artifacts/independent-cliches-hell/verification.json`.
- `rule.misc.apologizing`: independently verified and recorded at frozen fingerprint `4b9bc41f33731897c46943d3a481b95d7ddc9264191332537a45c28a70f72694`; 6 new and 28 cumulative cases passed. Evidence: `rust/artifacts/live-apologizing-20261105.json` and `rust/artifacts/independent-apologizing/verification.json`.
- `rule.misc.whence`: independently verified and recorded at frozen fingerprint `651affdc96c5d423e672846ceaeeab71425c8ef078d97ece8403451bc29b5c33`; 6 new and 34 cumulative cases passed. Evidence: `rust/artifacts/live-whence-20261105.json` and `rust/artifacts/independent-whence/verification.json`. An initial strict gate attempt was BLOCKED because status entries used strings; after correcting the schema, the live strict gate passed all 34 then-accepted cases.
- `rule.misc.false_plurals.kudos`: independently verified and recorded at frozen fingerprint `51397dadaaeb71d21b5fa7862a6a9393412905156ef49067c874c31521a77f5a`; 6 new and 40 cumulative cases passed. Evidence: `rust/artifacts/live-kudos-20261105.json` and `rust/artifacts/independent-kudos/verification.json`.
- Oracle: immutable source snapshot and isolated Python 3.12.3 environment are provisioned; 98 loaded modules resolve inside the venv, dependency/package/executable hashes verify, and upstream pytest passed (351 tests plus 15 subtests; exit 0).
- Harness: `rust/harness/**` and `rust/oracle.lock.json` are independently verified on fingerprint `5817279dd9575a62a982da6d943fcbd26e8f31ff317b3626d71175afcdc6b38d`. Unit tests: 14/14; comparator discriminators: 23/23; live oracle self-check: 2/2 PASS with harness-only scope. Before the first capability was recorded, the strict gate and benchmark correctly returned BLOCKED with no accepted capabilities.
- `psychology.mental_telepathy`: independently verified and recorded at frozen fingerprint `e8b977cf37707a3f1fba2abe2aac64cb4daf02dcd470526507823033bbbb2e30`; 6 new and 46 cumulative compatibility cases passed. Evidence: `rust/artifacts/live-mental-telepathy-20261105.json` and `rust/artifacts/independent-mental-telepathy/verification.json`. Two oracle-smoke manifest entries are harness-only.
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

The corrected strict live gate passes all 40 cases across seven recorded capabilities; report: `rust/artifacts/strict-gate-kudos-20261105.json`. The strict live gate passes all 46 cases across eight recorded capabilities; report: `rust/artifacts/strict-gate-mental-telepathy-20261105.json`. Add behavioral RED tests for `rule.misc.illogic.coin`, implement its pinned pattern and verify cumulatively. Default configuration, combined-rule behavior, and most rules remain unverified.

# Migration progress

## Current state

- The immutable reference is Proselint `0.16.0`, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; its upstream suite passed 351 tests and 15 subtests.
- The rule inventory contains 124 registered entries across 106 unique paths (110 enabled by default and 14 disabled by default).
- The Rust port has 49 accepted capabilities and 423 accepted compatibility cases. Current-directory and ancestor config discovery independently passed all eight cases; the refreshed cumulative gate passed all 423 accepted cases. See [`../artifacts/strict-gate-ancestor-discovery-20261106.json`](../artifacts/strict-gate-ancestor-discovery-20261106.json), [`../artifacts/independent-cwd-discovery-verification-20261106.json`](../artifacts/independent-cwd-discovery-verification-20261106.json), and [`../artifacts/independent-ancestor-discovery-20261106/ancestor-discovery-live.json`](../artifacts/independent-ancestor-discovery-20261106/ancestor-discovery-live.json).
- The current manifest contains 429 cases: 423 accepted-scope cases, four per-file configuration cases not yet accepted as a capability, and two harness smoke cases outside production comparison.
- Accepted coverage includes eleven bounded CLI capabilities and selected rules in annotations, cliches, misc, psychology, and typography. The inventory remains the source of truth for exact coverage.
- This is a partial port. Most registered checks, broad default configuration behavior, and full CLI compatibility have not been accepted. Passing selected-rule configurations does not establish full parity.
- See [`BENCHMARKS.md`](BENCHMARKS.md) for measured performance on one accepted rule. Those results apply only to the specified workloads and measurement modes.

- Configuration coverage includes 22 malformed-JSON cases, five invalid-UTF-8 cases, 12 recursion-depth cases, five ignored-value cases, and 12 consumed-special-value cases. The exact 100,000-unclosed-array regression now matches Python’s structured RecursionError without a native stack abort. Directory config-path errors now match in JSON, Full, and compact modes for four live cases; the Unix focused test also checks symlink canonicalization.

## Next work

Full output now defaults when `--output-format` is omitted for the verified stdin cases. Current-directory and nearest-first ancestor config discovery now match, including explicit-path precedence and malformed-winner behavior. Next, specify XDG/HOME config discovery; no-config-found bundled default-rule execution remains separate. Continue with the pinned rule inventory. For each, record upstream-derived cases, meaningful pre-implementation failures, passing implementation tests, live comparisons, independent verification on a frozen source fingerprint, and evidence before proceeding.

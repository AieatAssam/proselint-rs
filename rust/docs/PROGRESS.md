# Migration progress

## Current state

- The immutable reference is Proselint `0.16.0`, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; its upstream suite passed 351 tests and 15 subtests.
- The rule inventory contains 124 registered entries across 106 unique paths (110 enabled by default and 14 disabled by default).
- The Rust port has 44 accepted capabilities and 395 accepted compatibility cases. The latest cumulative live gate passed all 395 cases with zero mismatches; see [`../artifacts/strict-gate-ignored-values-20261106.json`](../artifacts/strict-gate-ignored-values-20261106.json). The ignored-value capability passed an independent five-case comparison at a frozen source fingerprint in [`../artifacts/ignored-values-independent-live-20261106.json`](../artifacts/ignored-values-independent-live-20261106.json).
- The current manifest contains 402 cases: 395 accepted-scope cases, one directory-config case awaiting implementation, four per-file configuration cases not yet accepted as a capability, and two harness smoke cases outside production comparison.
- Accepted coverage includes six bounded CLI capabilities and selected rules in annotations, cliches, misc, psychology, and typography. The inventory remains the source of truth for exact coverage.
- This is a partial port. Most registered checks, broad default configuration behavior, and full CLI compatibility have not been accepted. Passing selected-rule configurations does not establish full parity.
- See [`BENCHMARKS.md`](BENCHMARKS.md) for measured performance on one accepted rule. Those results apply only to the specified workloads and measurement modes.

- Configuration coverage includes 22 malformed-JSON cases, five invalid-UTF-8 cases, 11 recursion-depth cases, and five cases for Python-valid values in ignored top-level fields. Consumed non-finite values and lone-surrogate semantics and directory config-path errors remain open.

## Next work

Next, implement the consumed-setting errors already observed for `NaN`, `1e999`, and lone-surrogate values, then fix directory config-path errors as a separate capability. Continue with the pinned rule inventory. For each, record upstream-derived cases, meaningful pre-implementation failures, passing implementation tests, live comparisons, independent verification on a frozen source fingerprint, and evidence before proceeding.

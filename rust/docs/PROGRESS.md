# Migration progress

## Current state

- The immutable reference is Proselint `0.16.0`, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; its upstream suite passed 351 tests and 15 subtests.
- The rule inventory contains 124 registered entries across 106 unique paths (110 enabled by default and 14 disabled by default).
- The Rust port has 20 accepted capabilities and 122 compatibility cases. The latest cumulative live comparison passed all 122 cases; see [`../artifacts/strict-gate-benchmark-docs-final-20261105.json`](../artifacts/strict-gate-benchmark-docs-final-20261105.json).
- Accepted coverage includes two bounded CLI contracts and selected rules in annotations, cliches, misc, psychology, and typography. The inventory remains the source of truth for exact coverage.
- This is a partial port. Most registered checks, broad default configuration behavior, and full CLI compatibility have not been accepted. Passing selected-rule configurations does not establish full parity.
- See [`BENCHMARKS.md`](BENCHMARKS.md) for measured performance on one accepted rule. Those results apply only to the specified workloads and measurement modes.

## Next work

Continue selecting capabilities from the pinned inventory. For each, record upstream-derived cases, meaningful pre-implementation failures, passing implementation tests, live comparisons, independent verification on a frozen source fingerprint, and evidence before proceeding. Continue updating this record as gates pass.

# Migration progress

## Current state

- The immutable reference is Proselint `0.16.0`, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; its upstream suite passed 351 tests and 15 subtests.
- The rule inventory contains 124 registered entries across 106 unique paths (110 enabled by default and 14 disabled by default).
- The Rust port has 43 accepted capabilities and 390 accepted compatibility cases. The latest cumulative live gate passed all 390 cases with zero mismatches; see [`../artifacts/strict-gate-recursion-depth-final-20261106.json`](../artifacts/strict-gate-recursion-depth-final-20261106.json). The depth capability also passed an independent 11-case live comparison at a frozen source fingerprint in [`../artifacts/recursion_depth_final_verifier.json`](../artifacts/recursion_depth_final_verifier.json).
- The current manifest contains 396 cases: 390 accepted-scope cases, four passing per-file configuration cases that are not yet accepted as a capability, and two harness smoke cases outside production comparison. The per-file comparison is recorded at [`../artifacts/independent-review-fixes/per-file-targeted-348.json`](../artifacts/independent-review-fixes/per-file-targeted-348.json).
- Accepted coverage includes five bounded CLI capabilities and selected rules in annotations, cliches, misc, psychology, and typography. The inventory remains the source of truth for exact coverage.
- This is a partial port. Most registered checks, broad default configuration behavior, and full CLI compatibility have not been accepted. Passing selected-rule configurations does not establish full parity.
- See [`BENCHMARKS.md`](BENCHMARKS.md) for measured performance on one accepted rule. Those results apply only to the specified workloads and measurement modes.

- Configuration coverage includes 22 malformed-JSON cases, five invalid-UTF-8 cases, and 11 recursion-depth cases. The accepted depth slice fixes the 66,000-level process abort and matches tested Python behavior through depth 9,996 and at the 9,997 recursion boundary. Python-valid numeric and lone-surrogate values, directory config-path errors, and any additional extreme-depth mismatches remain open.

## Next work

Next, probe `NaN` and `1e999` in settings the config actually consumes before selecting conversion semantics; then probe lone-surrogate values and directory config-path errors as separate capabilities. Continue with the pinned rule inventory. For each, record upstream-derived cases, meaningful pre-implementation failures, passing implementation tests, live comparisons, independent verification on a frozen source fingerprint, and evidence before proceeding.

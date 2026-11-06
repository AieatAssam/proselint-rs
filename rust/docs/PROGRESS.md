# Migration progress

## Current state

- The immutable reference is Proselint `0.16.0`, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; its upstream suite passed 351 tests and 15 subtests.
- The rule inventory contains 124 registered entries across 106 unique paths (110 enabled by default and 14 disabled by default).
- The Rust port has 54 accepted capabilities and 462 accepted compatibility cases. `misc.capitalization.months` passed its 444-case cumulative gate; `psychology.p_equals_zero` passed focused tests 5/5 and independent live comparison 8/8; `spelling.ance_ence` passed focused tests 9/9 and independent live comparison 10/10. See [`../artifacts/strict-gate-ance-ence-20261106.json`](../artifacts/strict-gate-ance-ence-20261106.json), [`../artifacts/independent-ance-ence-20261106/ance-ence-live.json`](../artifacts/independent-ance-ence-20261106/ance-ence-live.json), and [`../artifacts/ance_ence_oracle_self_20261106.json`](../artifacts/ance_ence_oracle_self_20261106.json).
- The current manifest contains 468 cases: 462 implementation-verified cases, four per-file configuration cases not yet accepted as a capability, and two harness smoke cases outside production comparison.
- Accepted coverage includes twelve bounded CLI capabilities and selected rules in annotations, cliches, misc, psychology, and typography. The inventory remains the source of truth for exact coverage.
- This is a partial port. Most registered checks, broad default configuration behavior, and full CLI compatibility have not been accepted. Passing selected-rule configurations does not establish full parity.
- See [`BENCHMARKS.md`](BENCHMARKS.md) for measured performance on one accepted rule. Those results apply only to the specified workloads and measurement modes.

- Configuration coverage includes 22 malformed-JSON cases, five invalid-UTF-8 cases, 12 recursion-depth cases, five ignored-value cases, and 12 consumed-special-value cases. The exact 100,000-unclosed-array regression now matches Python’s structured RecursionError without a native stack abort. Directory config-path errors now match in JSON, Full, and compact modes for four live cases; the Unix focused test also checks symlink canonicalization.

## Next work

Full output now defaults when `--output-format` is omitted for the verified stdin cases. Cwd/ancestor and XDG/HOME config discovery now match for their recorded cases. No-config-found bundled default-rule execution remains separate. The latest regex rule verifies its specific unescaped-dot behavior; this does not establish general regex-engine equivalence. Next specify `misc.composition`, preserving its two same-path registrations, duplicates, and max_errors ordering. After one small accepted batch, broaden the benchmark to a parity-proved combined profile.

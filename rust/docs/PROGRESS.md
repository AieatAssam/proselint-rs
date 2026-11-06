# Migration progress

## Current state

- The immutable reference is Proselint `0.16.0`, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; its upstream suite passed 351 tests and 15 subtests.
- The rule inventory contains 124 registered entries across 106 unique paths (110 enabled by default and 14 disabled by default).
- The Rust port has 51 accepted capabilities and 438 accepted compatibility cases. Cwd/ancestor and XDG/HOME discovery independently passed all 16 cases; `misc.phrasal_adjectives.ly` independently passed all seven cases. The cumulative live gate passed all 438 accepted cases. See [`../artifacts/strict-gate-phrasal-adjectives-accepted-20261106.json`](../artifacts/strict-gate-phrasal-adjectives-accepted-20261106.json), [`../artifacts/independent-phrasal-adjectives-20261106/phrasal-adjectives-live.json`](../artifacts/independent-phrasal-adjectives-20261106/phrasal-adjectives-live.json), and [`../artifacts/phrasal-adjectives-live-green-20261106.json`](../artifacts/phrasal-adjectives-live-green-20261106.json).
- The current manifest contains 444 cases: 438 accepted-scope cases, four per-file configuration cases not yet accepted as a capability, and two harness smoke cases outside production comparison.
- Accepted coverage includes twelve bounded CLI capabilities and selected rules in annotations, cliches, misc, psychology, and typography. The inventory remains the source of truth for exact coverage.
- This is a partial port. Most registered checks, broad default configuration behavior, and full CLI compatibility have not been accepted. Passing selected-rule configurations does not establish full parity.
- See [`BENCHMARKS.md`](BENCHMARKS.md) for measured performance on one accepted rule. Those results apply only to the specified workloads and measurement modes.

- Configuration coverage includes 22 malformed-JSON cases, five invalid-UTF-8 cases, 12 recursion-depth cases, five ignored-value cases, and 12 consumed-special-value cases. The exact 100,000-unclosed-array regression now matches Python’s structured RecursionError without a native stack abort. Directory config-path errors now match in JSON, Full, and compact modes for four live cases; the Unix focused test also checks symlink canonicalization.

## Next work

Full output now defaults when `--output-format` is omitted for the verified stdin cases. Cwd/ancestor and XDG/HOME config discovery now match for their recorded cases. No-config-found bundled default-rule execution remains separate. The latest rule verifies the exact ASCII whitespace adaptation needed for one Fast `ExistenceSimple` pattern; this does not establish general regex-engine equivalence. Next, select an unaccepted rule from the pinned inventory, specify it against the oracle, and add meaningful failing tests before implementation.

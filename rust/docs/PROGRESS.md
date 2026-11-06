# Migration progress

## Current state

- The immutable reference is Proselint `0.16.0`, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; its upstream suite passed 351 tests and 15 subtests.
- The rule inventory contains 124 registered entries across 106 unique paths (110 enabled by default and 14 disabled by default).
- The accepted set and latest official strict gate cover 65 capabilities and pass 591/591 implementation cases at fingerprint `a797fada007b078ea9de845f18948959d3121cbf2d96c64942a23ed26af29570`. `redundancy.misc.nordquist` is now frozen as an 11-case RED while its implementation is pending. Recent independent live milestones include `oxymorons` (13), `skunked_terms` (11), `spelling.in_un` (10), `mondegreens` (10), `malapropisms` (10), `spelling.ve_of` (11), `spelling.ely_ly` (9), `spelling.ally_ly` (10), `misc.composition` (38), `spelling.ance_ence` (10), and `psychology.p_equals_zero` (8). Evidence is indexed in `status.json` and `status-history.jsonl`.
- The current manifest contains 608 cases: 591 implementation cases pass in the latest official gate, eleven `nordquist` cases are frozen as RED, four per-file configuration cases remain unaccepted, and two harness smoke cases are outside production comparison.
- Accepted coverage includes thirteen bounded CLI capabilities and selected rules in annotations, cliches, misc, psychology, terms, and typography. The inventory remains the source of truth for exact coverage.
- This is a partial port. Most registered checks, broad default configuration behavior, and full CLI compatibility have not been accepted. Passing selected-rule configurations does not establish full parity.
- See [`BENCHMARKS.md`](BENCHMARKS.md) for separate isolated-rule and combined selected-rule measurements. Each result applies only to its stated checks, inputs, and measurement mode.

- Configuration coverage includes 22 malformed-JSON cases, five invalid-UTF-8 cases, 12 recursion-depth cases, five ignored-value cases, and 12 consumed-special-value cases. The exact 100,000-unclosed-array regression now matches Python’s structured RecursionError without a native stack abort. Directory config-path errors now match in JSON, Full, and compact modes for four live cases; the Unix focused test also checks symlink canonicalization.

## Next work

Full output defaults when `--output-format` is omitted for the verified stdin cases. Cwd/ancestor and XDG/HOME config discovery match for their recorded cases. No-config-found bundled default-rule execution remains separate. The latest regex rule verifies its specific unescaped-dot behavior; this does not establish general regex-engine equivalence. `cli.combined_checks` passes three live cases covering registry-order truncation, final position sorting, and duplicate retention. The benchmark has isolated and combined profiles, but both candidate fingerprints predate the current Rust implementation; rerun performance measurements on the final parity-frozen source before making current-candidate claims. Next implement Nordquist from the 11-case live RED, preserving literal-period regex matching, the Python `None` lookup for escaped dictionary keys, and pattern batching before `max_errors` sorting.

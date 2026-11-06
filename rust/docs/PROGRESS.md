# Migration progress

## Current state

- The immutable reference is Proselint `0.16.0`, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; its upstream suite passed 351 tests and 15 subtests.
- The rule inventory contains 124 registered entries across 106 unique paths (110 enabled by default and 14 disabled by default).
- The accepted set and latest official strict gate cover 66 capabilities and pass 602/602 implementation cases at fingerprint `2b10b9c84770b2c408bbbc88fd8a10bbd33c633f2daa3337bcb9ac040fa5874e`. Recent independent live milestones include `oxymorons` (13), `skunked_terms` (11), `spelling.in_un` (10), `mondegreens` (10), `malapropisms` (10), `spelling.ve_of` (11), `spelling.ely_ly` (9), `spelling.ally_ly` (10), `misc.composition` (38), `spelling.ance_ence` (10), and `psychology.p_equals_zero` (8). Evidence is indexed in `status.json` and `status-history.jsonl`.
- The current manifest contains 608 cases: 602 implementation cases pass in the latest official gate, including the eleven focused and independent `nordquist` cases, four per-file configuration cases remain unaccepted, and two harness smoke cases are outside production comparison.
- Accepted coverage includes thirteen bounded CLI capabilities and selected rules in annotations, cliches, misc, psychology, terms, and typography. The inventory remains the source of truth for exact coverage.
- This is a partial port. Most registered checks, broad default configuration behavior, and full CLI compatibility have not been accepted. Passing selected-rule configurations does not establish full parity.
- See [`BENCHMARKS.md`](BENCHMARKS.md) for separate isolated-rule and combined selected-rule measurements. Each result applies only to its stated checks, inputs, and measurement mode.

- Configuration coverage includes 22 malformed-JSON cases, five invalid-UTF-8 cases, 12 recursion-depth cases, five ignored-value cases, and 12 consumed-special-value cases. The exact 100,000-unclosed-array regression now matches Python’s structured RecursionError without a native stack abort. Directory config-path errors now match in JSON, Full, and compact modes for four live cases; the Unix focused test also checks symlink canonicalization.

## Next work

Full output defaults when `--output-format` is omitted for the verified stdin cases. Cwd/ancestor and XDG/HOME config discovery match for their recorded cases. No-config-found bundled default-rule execution remains separate. The latest regex rule verifies its specific unescaped-dot behavior; this does not establish general regex-engine equivalence. `cli.combined_checks` passes three live cases covering registry-order truncation, final position sorting, and duplicate retention. The benchmark has isolated and combined profiles, but both candidate fingerprints predate the current Rust implementation; rerun performance measurements on the final parity-frozen source before making current-candidate claims. Nordquist independent verification and the official cumulative 602-case comparison are complete; continue with the next unimplemented rule.

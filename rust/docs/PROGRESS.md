# Migration progress

## Current state

- The immutable reference is Proselint `0.16.0`, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; its upstream suite passed 351 tests and 15 subtests.
- The rule inventory contains 124 registered entries across 106 unique paths (110 enabled by default and 14 disabled by default).
- The latest cumulative gate passes 557/557 implementation cases across 62 capabilities; `spelling.in_un` passed focused and independent 10/10 live comparisons. Earlier milestone results include `misc.capitalization.months` (444-case cumulative gate), `psychology.p_equals_zero` (5 focused tests, 8 independent live cases), `spelling.ance_ence` (9 focused tests, 10 independent live cases), `misc.composition` (23 focused tests, 38 independent live cases), `cli.combined_checks` (3 live cases), `terms.eponymous_adjectives` (4 live cases), `spelling.ally_ly` (10 live cases), `spelling.ely_ly` (9 live cases), `spelling.ve_of` (11 live cases), and `malapropisms` (10 focused and independent live cases). Evidence for the latest completed gate and latest focused rule is recorded below and in the status history.
- The current manifest contains 563 cases: 557 pass in the latest cumulative gate, four per-file configuration cases remain unaccepted, and two harness smoke cases are outside production comparison.
- Accepted coverage includes thirteen bounded CLI capabilities and selected rules in annotations, cliches, misc, psychology, terms, and typography. The inventory remains the source of truth for exact coverage.
- This is a partial port. Most registered checks, broad default configuration behavior, and full CLI compatibility have not been accepted. Passing selected-rule configurations does not establish full parity.
- See [`BENCHMARKS.md`](BENCHMARKS.md) for separate isolated-rule and combined selected-rule measurements. Each result applies only to its stated checks, inputs, and measurement mode.

- Configuration coverage includes 22 malformed-JSON cases, five invalid-UTF-8 cases, 12 recursion-depth cases, five ignored-value cases, and 12 consumed-special-value cases. The exact 100,000-unclosed-array regression now matches Python’s structured RecursionError without a native stack abort. Directory config-path errors now match in JSON, Full, and compact modes for four live cases; the Unix focused test also checks symlink canonicalization.

## Next work

Full output now defaults when `--output-format` is omitted for the verified stdin cases. Cwd/ancestor and XDG/HOME config discovery now match for their recorded cases. No-config-found bundled default-rule execution remains separate. The latest regex rule verifies its specific unescaped-dot behavior; this does not establish general regex-engine equivalence. `cli.combined_checks` passes three live cases covering registry-order truncation, final position sorting, and duplicate retention; the cumulative gate passes 557/557 after `spelling.in_un`. The benchmark has isolated and combined profiles, but both candidate fingerprints predate the current Rust implementation; rerun performance measurements on the final parity-frozen source before making current-candidate claims. Next implement `mondegreens` from the ten-case live RED; its eight preferred-form mappings are pinned, while focused green, independent verification, and the cumulative gate remain pending.

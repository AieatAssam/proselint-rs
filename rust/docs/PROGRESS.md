# Migration progress

## Current state

- The immutable reference is Proselint `0.16.0`, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; its upstream suite passed 351 tests and 15 subtests.
- The rule inventory contains 124 registered entries across 106 unique paths (110 enabled by default and 14 disabled by default).
- The latest cumulative gate passes 578/578 implementation cases across 64 capabilities; `skunked_terms` passed eleven focused and independent live cases. Recent independent live milestones: `spelling.in_un` (10 cases), `mondegreens` (10 cases), and earlier `malapropisms` (10 cases), `spelling.ve_of` (11 cases), `spelling.ely_ly` (9 cases), `spelling.ally_ly` (10 cases), `misc.composition` (38 cases), `spelling.ance_ence` (10 cases), and `psychology.p_equals_zero` (8 cases). Next implement `oxymorons`; upstream treats `advancing backwards?` as a regex with an optional final `s`. Evidence is indexed in `status.json` and `status-history.jsonl`.
- The current manifest contains 584 cases: 578 pass in the latest cumulative gate, four per-file configuration cases remain unaccepted, and two harness smoke cases are outside production comparison.
- Accepted coverage includes thirteen bounded CLI capabilities and selected rules in annotations, cliches, misc, psychology, terms, and typography. The inventory remains the source of truth for exact coverage.
- This is a partial port. Most registered checks, broad default configuration behavior, and full CLI compatibility have not been accepted. Passing selected-rule configurations does not establish full parity.
- See [`BENCHMARKS.md`](BENCHMARKS.md) for separate isolated-rule and combined selected-rule measurements. Each result applies only to its stated checks, inputs, and measurement mode.

- Configuration coverage includes 22 malformed-JSON cases, five invalid-UTF-8 cases, 12 recursion-depth cases, five ignored-value cases, and 12 consumed-special-value cases. The exact 100,000-unclosed-array regression now matches Python’s structured RecursionError without a native stack abort. Directory config-path errors now match in JSON, Full, and compact modes for four live cases; the Unix focused test also checks symlink canonicalization.

## Next work

Full output now defaults when `--output-format` is omitted for the verified stdin cases. Cwd/ancestor and XDG/HOME config discovery now match for their recorded cases. No-config-found bundled default-rule execution remains separate. The latest regex rule verifies its specific unescaped-dot behavior; this does not establish general regex-engine equivalence. `cli.combined_checks` passes three live cases covering registry-order truncation, final position sorting, and duplicate retention; the cumulative gate passes 578/578 after `skunked_terms`. The benchmark has isolated and combined profiles, but both candidate fingerprints predate the current Rust implementation; rerun performance measurements on the final parity-frozen source before making current-candidate claims. Next specify `oxymorons` from the pinned source; treat the `?` in `advancing backwards?` as a regex quantifier, not literal punctuation.

# Migration progress

## Current state

- The immutable reference is Proselint `0.16.0`, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; its upstream suite passed 351 tests and 15 subtests.
- The rule inventory contains 124 registered entries across 106 unique paths (110 enabled by default and 14 disabled by default).
- The Rust port has 59 accepted capabilities and 526 accepted compatibility cases after the latest cumulative gate. `misc.capitalization.months` passed its 444-case cumulative gate; `psychology.p_equals_zero` passed focused tests 5/5 and independent live comparison 8/8; `spelling.ance_ence` passed focused tests 9/9 and independent live comparison 10/10; `misc.composition` passed focused tests 23/23 and independent live comparison 38/38; `cli.combined_checks` passed three live cases; `terms.eponymous_adjectives` passed four live cases; `spelling.ally_ly` passed ten live cases; and `spelling.ely_ly` passed nine focused live cases and independent verification. See [`../artifacts/strict-gate-ely-ly-20261106.json`](../artifacts/strict-gate-ely-ly-20261106.json), [`../artifacts/independent-ely-ly-20261106/verification-evidence.json`](../artifacts/independent-ely-ly-20261106/verification-evidence.json), and [`../artifacts/ely-ly-green-20261106.json`](../artifacts/ely-ly-green-20261106.json).
- The current manifest contains 532 cases: 526 implementation-verified cases, four per-file configuration cases not yet accepted as a capability, and two harness smoke cases outside production comparison.
- Accepted coverage includes thirteen bounded CLI capabilities and selected rules in annotations, cliches, misc, psychology, terms, and typography. The inventory remains the source of truth for exact coverage.
- This is a partial port. Most registered checks, broad default configuration behavior, and full CLI compatibility have not been accepted. Passing selected-rule configurations does not establish full parity.
- See [`BENCHMARKS.md`](BENCHMARKS.md) for separate isolated-rule and combined selected-rule measurements. Each result applies only to its stated checks, inputs, and measurement mode.

- Configuration coverage includes 22 malformed-JSON cases, five invalid-UTF-8 cases, 12 recursion-depth cases, five ignored-value cases, and 12 consumed-special-value cases. The exact 100,000-unclosed-array regression now matches Python’s structured RecursionError without a native stack abort. Directory config-path errors now match in JSON, Full, and compact modes for four live cases; the Unix focused test also checks symlink canonicalization.

## Next work

Full output now defaults when `--output-format` is omitted for the verified stdin cases. Cwd/ancestor and XDG/HOME config discovery now match for their recorded cases. No-config-found bundled default-rule execution remains separate. The latest regex rule verifies its specific unescaped-dot behavior; this does not establish general regex-engine equivalence. `cli.combined_checks` passes three live cases covering registry-order truncation, final position sorting, and duplicate retention; the cumulative gate passes 526/526. The benchmark has isolated and combined profiles. Next specify `spelling.ve_of` against the pinned oracle.

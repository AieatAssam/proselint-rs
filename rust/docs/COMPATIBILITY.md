# Compatibility contract

The pinned reference CLI is `proselint` (also available as `python -m proselint`). It supports `check`, `version`, and `dump-config`; `check` accepts configuration and output-format options, files or directories, and stdin. Directory traversal, text decoding, newline handling, quote suppression, match positions, maximum finding limits, sorting, and serialization each have observable behavior that must be checked against the live reference before claiming compatibility.

The pinned inventory contains 124 registered entries across 106 unique check paths, including duplicate paths, helper types, matcher options, flags, offsets, and data fields. See [`../inventory.json`](../inventory.json).

## Verified scope

The Rust port currently has 41 accepted capabilities with 369 live compatibility cases. Coverage includes bounded JSON CLI behavior, selected-rule configurations, and a set of individual rules. The exact accepted rules, cases, and evidence are recorded in [`../status.json`](../status.json) and live reports under [`../artifacts/`](../artifacts/). The malformed-configuration slice verifies Python-compatible JSON syntax errors (22 cases) and invalid UTF-8 error objects (five cases); independent combined evidence is [`../artifacts/independent-invalid-config/live-report.json`](../artifacts/independent-invalid-config/live-report.json).

## Unverified scope

The latest 369-case cumulative live gate passed with zero mismatches; see [`../artifacts/strict-gate-config-errors-20261106.json`](../artifacts/strict-gate-config-errors-20261106.json). The accepted cases do not cover the entire default configuration, every registered check, all output formats, every file traversal and decoding edge case, complete configuration discovery and merging, or all error and multi-file behavior. Unicode and ordering behavior outside explicitly covered cases also remains unverified. Treat uncovered behavior as unknown; do not infer full parity from isolated rule or CLI passes.

The inventory contains 69 unique check paths that are not yet accepted. The 375-case manifest also contains four separately passing per-file configuration cases that are not yet accepted as a capability, plus two harness smoke cases outside production comparison. An earlier independently-run strict gate passed 342 cases; the current 369-case gate is linked above. Earlier evidence is [`../artifacts/independent-review-fixes/final-gate-348.json`](../artifacts/independent-review-fixes/final-gate-348.json) and the separate [per-file comparison](../artifacts/independent-review-fixes/per-file-targeted-348.json). These results do not establish full CLI or rule-inventory parity.

Every compatibility claim requires a fresh live comparison with the pinned Python executable. Cached snapshots are useful evidence for review but cannot establish parity by themselves.

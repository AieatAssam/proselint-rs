# Compatibility contract

The pinned reference CLI is `proselint` (also available as `python -m proselint`). It supports `check`, `version`, and `dump-config`; `check` accepts configuration and output-format options, files or directories, and stdin. Directory traversal, text decoding, newline handling, quote suppression, match positions, maximum finding limits, sorting, and serialization each have observable behavior that must be checked against the live reference before claiming compatibility.

The pinned inventory contains 124 registered entries across 106 unique check paths, including duplicate paths, helper types, matcher options, flags, offsets, and data fields. See [`../inventory.json`](../inventory.json).

## Verified scope

The Rust port currently has 28 accepted capabilities with 198 live compatibility cases. Coverage includes bounded JSON CLI behavior, selected-rule configurations, and a set of individual rules. The exact accepted rules, cases, and evidence are recorded in [`../status.json`](../status.json) and the cumulative reports under [`../artifacts/`](../artifacts/).

## Unverified scope

The accepted cases do not cover the entire default configuration, every registered check, all output formats, every file traversal and decoding edge case, complete configuration discovery and merging, or all error and multi-file behavior. Unicode and ordering behavior outside explicitly covered cases also remains unverified. Treat uncovered behavior as unknown; do not infer full parity from isolated rule or CLI passes.

Every compatibility claim requires a fresh live comparison with the pinned Python executable. Cached snapshots are useful evidence for review but cannot establish parity by themselves.

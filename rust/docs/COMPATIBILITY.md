# Compatibility contract (reconnaissance)

The reference CLI entry point is `proselint` (`pyproject.toml`) and `python -m proselint`. `proselint/command_line.py` exposes `check`, `version`, and `dump-config`, with `--config`, `--output-format {full,json,compact}`, `--verbose`, and `--version`. `check` accepts `--demo` and zero or more paths; no paths means stdin.

`proselint/tools.py` handles file traversal and linting. Explicit paths are accepted regardless of extension; directory traversal selects `.md`, `.txt`, `.rtf`, `.html`, `.tex`, and `.markdown`. Nonempty text and empty files receive one leading and trailing newline; empty stdin remains empty. Match indices and columns are Python string (Unicode code-point) positions. Quote filtering is enabled by default. The `max_errors` cap is applied before sorting by line and column, preserving generator order for ties.

`proselint/config/paths.py` and `proselint/config/__init__.py` define config discovery and merge behavior. The CLI supports full, compact, and JSON output via `proselint/log.py`; exact serialization and failure semantics remain to be recorded through live CLI probes.

Rule registration is declared by `proselint/checks/__init__.py` and nested package registries, flattened in order by `proselint/registry/__init__.py`. Fast matching requires Google RE2; Fancy patterns use Python `re`. The pinned executable snapshot reports 124 registered entries across 106 unique check paths; 110 entries are enabled by default and 14 disabled. The full ordered entry inventory, including duplicate paths, helper type, matcher options, flags, offsets and data field counts, is in `rust/inventory.json`.

All statements above are source reconnaissance, not differential acceptance. JSON aggregation, duplicate source keys, decoding/newline translation, and config edge cases still need explicit live cases. See `rust/oracle.lock.json`, live reports, and the machine-readable inventory for verified evidence as those artifacts are produced. No Rust capability is accepted yet.

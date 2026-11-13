# Compatibility

`proselint-rs` tracks Python Proselint 0.16.0. It supports the Rust CLI and many of the same lint checks, but it is not a drop-in replacement in every environment or command.

The current accepted scope is **1,560 of 1,562 cases** across the CLI and lint rules; two oracle-smoke cases are excluded. The latest full cumulative gate passed 1,550 cases, before the most recent accepted additions and CLI fixes, so a full gate is still due. Full compatibility is not established.

Focused comparisons against the pinned Python CLI verify that directory scans report root-level files before descending into child directories, and that a permission-denied file is reported while scanning continues. On Unix, SIGINT and SIGTERM print Python-compatible warnings and exit with status 3. Per-file check patterns normalize dot/repeated/trailing separators, match slash-containing patterns against trailing path components, and follow Python character-class rules (`!` negates; `^` is literal; a leading `]` can be included in a class). Absolute patterns also match normalized paths. Unix non-UTF-8 filenames retain their bytes in JSON file URIs and Full/Compact output labels.

## Known differences

- `proselint-rs --version` reports the date-based Rust product version and the upstream version it tracks. Its output intentionally differs from Python Proselint. The user accepts this engine-specific product-version difference as an intentional divergence; it does not block release acceptance, and the exact difference remains documented.
- Version JSON and verbose output do not invent Python runtime details.
- Demo JSON identifies the Rust program's bundled demo source rather than Python's installed source file.
- Configuration discovery can differ when `HOME` is unset and the operating system has no passwd home entry.
- Windows per-file glob separator behavior has a regression test but was not run against a Windows Python/Rust environment.
- Per-file error messages for filenames containing non-UTF-8 bytes remain unverified; JSON URIs and Full/Compact finding labels are covered on Unix.

These differences affect specific CLI and environment cases; they do not mean that the lint rules are disabled. See the release notes for the version and compatibility scope of a particular build.

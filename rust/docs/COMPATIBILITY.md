# Compatibility

`proselint-rs` tracks Python Proselint 0.16.0. It supports the Rust CLI and many of the same lint checks, but it is not a drop-in replacement in every environment or command.

The current accepted scope is **1,560 of 1,562 cases** across the CLI and lint rules; two oracle-smoke cases are excluded. The latest full cumulative gate passed 1,550 cases, before the most recent accepted additions and CLI fixes, so a full gate is still due. Full compatibility is not established.

Focused comparisons against the pinned Python CLI verify that directory scans report root-level files before descending into child directories, and that a permission-denied file is reported while scanning continues. Per-file check patterns match Python `Path.match`: absolute patterns work with relative CLI paths, slash-containing patterns match trailing path components, and `!` (not `^`) negates a character class.

## Known differences

- `proselint-rs --version` reports the date-based Rust product version and the upstream version it tracks. Its output intentionally differs from Python Proselint. The user accepts this engine-specific product-version difference as an intentional divergence; it does not block release acceptance, and the exact difference remains documented.
- Version JSON and verbose output do not invent Python runtime details.
- Demo JSON identifies the Rust program's bundled demo source rather than Python's installed source file.
- Configuration discovery can differ when `HOME` is unset and the operating system has no passwd home entry.
- SIGINT and SIGTERM handling has not been verified against Python Proselint.

These differences affect specific CLI and environment cases; they do not mean that the lint rules are disabled. See the release notes for the version and compatibility scope of a particular build.

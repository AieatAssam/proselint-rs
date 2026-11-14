# Compatibility

`proselint-rs` tracks Python Proselint 0.16.0. The functional CLI conversion is complete for the verified Unix scope: all 106 unique registered rule paths are implemented, and the current release gate passes **1,560/1,560 accepted cases across 158 capabilities**. The manifest contains 1,562 cases; two oracle-smoke cases are excluded. This does not claim byte-for-byte parity for Python runtime identity or unverified platforms.

The gate and full Python harness suite pass on the current source fingerprint. Focused comparisons also verify directory traversal order, continuation after unreadable files, Unix SIGINT/SIGTERM behavior, pathlib-style per-file glob matching, symlink-loop paths, and Unix non-UTF-8 filenames in JSON, Full, and Compact output.

## Known differences

- `proselint-rs --version` reports the date-based Rust product version and the upstream version it tracks. Its output intentionally differs from Python Proselint. The user accepts this engine-specific product-version difference as an intentional divergence; it does not block release acceptance, and the exact difference remains documented.
- Version JSON and verbose output do not invent Python runtime details.
- Demo JSON identifies the Rust program's bundled demo source rather than Python's installed source file.
- Configuration discovery can differ when `HOME` is unset and the operating system has no passwd home entry.
- Windows signal handling is not implemented; Windows per-file glob separator behavior has a regression test but was not run against a Windows Python/Rust environment.
- Per-file error messages for filenames containing non-UTF-8 bytes remain unverified; JSON URIs and Full/Compact finding labels are covered on Unix.

These differences affect specific CLI and environment cases; they do not mean that the lint rules are disabled. See the release notes for the version and compatibility scope of a particular build.

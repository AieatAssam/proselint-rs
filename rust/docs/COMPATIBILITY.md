# Compatibility

`proselint-rs` tracks Python Proselint 0.16.0. It supports the Rust CLI and many of the same lint checks, but it is not a drop-in replacement in every environment or command.

The current live comparison covers **1,464 accepted cases** across the CLI and lint rules. All accepted cases pass. **11 additional cases are excluded** because their behavior differs from or depends on the Python runtime. Full compatibility is not established.

## Known differences

- `proselint-rs --version` reports the date-based Rust product version and the upstream version it tracks. Its output intentionally differs from Python Proselint. The user accepts this engine-specific product-version difference as an intentional divergence; it does not block release acceptance, and the exact difference remains documented.
- Version JSON and verbose output do not invent Python runtime details.
- Demo JSON identifies the Rust program's bundled demo source rather than Python's installed source file.
- Configuration discovery can differ when `HOME` is unset and the operating system has no passwd home entry.

These differences affect specific CLI and environment cases; they do not mean that the lint rules are disabled. See the release notes for the version and compatibility scope of a particular build.

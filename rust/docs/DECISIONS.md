# Compatibility decisions

- The target is the native Rust library and CLI, compared with the immutable Proselint `0.16.0` baseline recorded in [`../oracle.lock.json`](../oracle.lock.json).
- Python is used as the development reference and test harness. The Rust runtime does not call Python.
- Rust dependencies are locked in `Cargo.lock`; the Python oracle's source and dependency lock remain pinned and unchanged.
- The user explicitly requested a product-specific standalone `--version` string. This one output intentionally differs from pinned Proselint; the Python expectation remains unchanged and the live mismatch is retained as evidence. The `version` subcommand remains in exact upstream parity.
- Where Rust behavior has not been tested against the live baseline, it is not claimed as compatible.
- A capability is accepted only after upstream behavior is specified, tests demonstrate the pre-implementation failure, implementation tests pass, live differential cases pass, and independent verification records the same source fingerprint.
- Partial coverage must not be described as full Proselint parity. See [`PROGRESS.md`](PROGRESS.md) and [`COMPATIBILITY.md`](COMPATIBILITY.md).

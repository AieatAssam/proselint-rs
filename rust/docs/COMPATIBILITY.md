# Compatibility status

The reference is the pinned Proselint 0.16.0 CLI at revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`. The Rust program is a native implementation; this document describes verified behavior and known differences, not a claim of complete compatibility.

## Current verified scope

The current accepted scope is **1,422 cases across 152 capabilities**. The manifest contains **1,436 cases**; 14 are outside that accepted scope. The latest cumulative release gate passed all 1,422 accepted cases at source fingerprint `c1c5c7636db6004c303da59f61015abd9ec32dcb564fdc5fd3e38f0782344c94`, with zero mismatches, timeouts, candidate errors, or oracle-integrity errors. Two earlier same-scope attempts had two pinned-reference timeouts each, on different cases; a final unchanged run passed. Reports are [`release-gate-final-retry-20261110.json`](../artifacts/release-gate-final-retry-20261110.json), [`release-gate-current-20261110.json`](../artifacts/release-gate-current-20261110.json), and [`release-gate-retry-20261110.json`](../artifacts/release-gate-retry-20261110.json).

The full-gate attestation passed fmt, Clippy, locked release build, **901 Rust tests across 145 targets**, pinned-oracle integrity, and comparator self-tests 23/23. The full Python harness suite passed **127/127** using the matching release attestation. The three requested rule acceptances were independently refreshed against the same candidate: uncomparables 11/11, restricted.elementary 3/3, and restricted.top1000 3/3. A newly added symlink-loop fix passed focused owner live parity 3/3 but remains outside the accepted scope pending independent verification.

The inventory has 124 registration entries and 106 unique rule paths. Those paths have selected-configuration acceptance evidence. This does not establish exhaustive default-configuration behavior or every interaction between rules. After the latest passing gate, a bounded benchmark passed parity preflight on three representative selected-rule workloads; its current medians and limits are in [`BENCHMARKS.md`](BENCHMARKS.md). It is not a default-configuration or whole-program performance claim.

## Known differences and gaps

- The requested Rust product `--version` text differs from upstream Proselint’s version output.
- JSON version metadata and `version --verbose` include Python runtime details that Rust does not fabricate.
- Demo JSON includes an installation-specific Python source path that differs from Rust’s `demo.md` source label.
- `cli.config.no_passwd_home_absent` remains a confirmed Linux mismatch when the process has no HOME and no passwd entry.
- Fourteen manifest cases remain outside the strict gate: the 11 previously unaccepted cases and three focused symlink-loop cases awaiting independent acceptance. Additional CLI, configuration, platform, default-configuration, and cross-rule behavior is not fully covered.
- The existing six unaccepted capability IDs are `cli.config.no_passwd_home`, `cli.demo.json`, `cli.product_version_flag`, `cli.version.json_metadata`, `cli.version.verbose_runtime`, and `harness.oracle_smoke`; `cli.symlink_loop_path` is a seventh pending independent acceptance. The product version flag is an authorized divergence; the other exclusions or gaps remain explicit in the ledger.
- Runtime independence passed for one Linux release binary/input in a minimal chroot with no Python or reference executable accessible; Windows/macOS and other execution paths were not covered.

Do not interpret a passing focused rule, selected configuration, cumulative scope, or benchmark as full Proselint compatibility. The exact case and capability ledger is maintained in [`status.json`](../status.json); historical verification attempts remain in [`status-history.jsonl`](../status-history.jsonl).

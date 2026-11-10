# Compatibility status

The reference is the pinned Proselint 0.16.0 CLI at revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`. The Rust program is a native implementation; this document describes verified behavior and known differences, not a claim of complete compatibility.

## Current verified scope

The latest full release gate passes **1,421/1,421 accepted live cases across 151 capabilities**. It used the current 1,432-case manifest; 11 cases across six unaccepted capability IDs remain outside the strict gate. The run had zero mismatches, timeouts, candidate errors, or oracle errors. Pinned Proselint 0.16.0 integrity was verified. Report: [`release-gate-final-20261110.json`](../artifacts/release-gate-final-20261110.json); detailed counts and timings are in [`status.json`](../status.json) and the history in [`status-history.jsonl`](../status-history.jsonl).

The same release gate passed fmt, Clippy, the locked release build, and **900 Rust tests across 145 targets**. The Python harness suite passed **125 tests**, and its Linux runtime-independence test passed against this source fingerprint. The three requested rule acceptances were independently rechecked against the same candidate: uncomparables 11/11, restricted.elementary 3/3, and restricted.top1000 3/3. The six new invalid-config/version-precedence CLI cases also passed owner and independent live comparison.

The inventory has 124 registration entries and 106 unique rule paths. Those paths have selected-configuration acceptance evidence. This does not establish exhaustive default-configuration behavior or every interaction between rules. A bounded benchmark was run only after the passing gate; it covers two selected rules and three recorded workloads, not whole-program or default-configuration performance. See [`BENCHMARKS.md`](BENCHMARKS.md).

## Known differences and gaps

- The requested Rust product `--version` text differs from upstream Proselint’s version output.
- JSON version metadata and `version --verbose` include Python runtime details that Rust does not fabricate.
- Demo JSON includes an installation-specific Python source path that differs from Rust’s `demo.md` source label.
- `cli.config.no_passwd_home_absent` remains a confirmed Linux mismatch when the process has no HOME and no passwd entry.
- Eleven manifest cases remain outside the strict gate. Additional CLI, configuration, platform, default-configuration, and cross-rule behavior is not fully covered.
- The six unaccepted capability IDs are `cli.config.no_passwd_home`, `cli.demo.json`, `cli.product_version_flag`, `cli.version.json_metadata`, `cli.version.verbose_runtime`, and `harness.oracle_smoke`. The product version flag is an authorized divergence; the other exclusions or gaps remain explicit in the ledger.
- Runtime independence passed for one Linux release binary/input in a minimal chroot with no Python or reference executable accessible; Windows/macOS and other execution paths were not covered.

Do not interpret a passing focused rule, selected configuration, cumulative scope, or benchmark as full Proselint compatibility. The exact case and capability ledger is maintained in [`status.json`](../status.json); historical verification attempts remain in [`status-history.jsonl`](../status-history.jsonl).

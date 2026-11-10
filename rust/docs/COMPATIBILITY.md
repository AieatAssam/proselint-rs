# Compatibility status

The reference is the pinned Proselint 0.16.0 CLI at revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`. The Rust program is a native implementation; this document describes verified behavior and known differences, not a claim of complete compatibility.

## Current verified scope

The latest full release gate passed **1,421/1,421 cases across 151 capabilities** against the 1,432-case manifest at source fingerprint `3f2a46f90d18544bf8067630fe5649f79a68c44e0d4f7a8f41d7587abbe6c881`, with zero mismatches, timeouts, candidate errors, or oracle errors. Since that gate, one default-config cross-rule case has passed focused owner and independent live comparison. The current accepted scope is **1,422 cases across 152 capabilities** in a 1,433-case manifest; that extension still needs the next cumulative checkpoint before release. Eleven cases across six unaccepted capability IDs remain outside the gate. The pinned Proselint 0.16.0 baseline was verified for both runs. Evidence and scope freshness are recorded in [`status.json`](../status.json) and [`status-history.jsonl`](../status-history.jsonl); the previous gate report is [`release-gate-final-20261110.json`](../artifacts/release-gate-final-20261110.json).

That release gate passed fmt, Clippy, the locked release build, and **900 Rust tests across 145 targets**. The Python harness suite passed **125 tests** at the same earlier source fingerprint; after the one added harness case, its focused scope test passes 1/1 and the full suite is pending the next checkpoint. The three requested rule acceptances were independently rechecked against that candidate: uncomparables 11/11, restricted.elementary 3/3, and restricted.top1000 3/3. The six invalid-config/version-precedence CLI cases and the new default-config cross-rule case also passed independent live comparison.

The inventory has 124 registration entries and 106 unique rule paths. Those paths have selected-configuration acceptance evidence. This does not establish exhaustive default-configuration behavior or every interaction between rules. The benchmark records the prior gate fingerprint and covers two selected rules and three workloads, not whole-program or default-configuration performance; rerun it against the current source before release. See [`BENCHMARKS.md`](BENCHMARKS.md).

## Known differences and gaps

- The requested Rust product `--version` text differs from upstream Proselint’s version output.
- JSON version metadata and `version --verbose` include Python runtime details that Rust does not fabricate.
- Demo JSON includes an installation-specific Python source path that differs from Rust’s `demo.md` source label.
- `cli.config.no_passwd_home_absent` remains a confirmed Linux mismatch when the process has no HOME and no passwd entry.
- Eleven manifest cases remain outside the strict gate. Additional CLI, configuration, platform, default-configuration, and cross-rule behavior is not fully covered.
- The six unaccepted capability IDs are `cli.config.no_passwd_home`, `cli.demo.json`, `cli.product_version_flag`, `cli.version.json_metadata`, `cli.version.verbose_runtime`, and `harness.oracle_smoke`. The product version flag is an authorized divergence; the other exclusions or gaps remain explicit in the ledger.
- Runtime independence passed for one Linux release binary/input in a minimal chroot with no Python or reference executable accessible; Windows/macOS and other execution paths were not covered.

Do not interpret a passing focused rule, selected configuration, cumulative scope, or benchmark as full Proselint compatibility. The exact case and capability ledger is maintained in [`status.json`](../status.json); historical verification attempts remain in [`status-history.jsonl`](../status-history.jsonl).

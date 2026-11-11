# Compatibility status

The reference is the pinned Proselint 0.16.0 CLI at revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`. The Rust program is a native implementation; this document describes verified behavior and known differences, not a claim of complete compatibility.

## Current verified scope


The latest full cumulative release gate passed **1,452/1,452 accepted live cases across 153 capabilities**, from a 1,463-case manifest (11 excluded). It recorded zero mismatches, timeouts, candidate/oracle errors, or blockers; the pinned Proselint 0.16.0 oracle was verified and comparator checks passed 29/29. Current source fingerprint: `9360dfeb3bc37d2cbeb18e14e56f280754bc822905a3ba128db6c34c446cd39f`; release binary SHA-256: `2619f521682d662ebc0af3e23eac74207d148906d73e854b99e3d521bd961a24`. Evidence: [`release-candidate-gate-20261111-clean-harness.json`](../artifacts/release-candidate-gate-20261111-clean-harness.json) and [`20261111T092101683349Z-build.attestation.json`](../artifacts/20261111T092101683349Z-build.attestation.json).

The gate passed **917 Rust tests across 145 targets**, formatter, warning-free Clippy, and locked release build. The full Python harness then passed **144/144** in 75.022 seconds using the current attestation. Two initial case-inventory assertions were corrected to select explicit-config cases; their pinned diagnostic outputs and comparator rules did not change. Final suite log: [`full-harness-suite-20261111-clean.log`](../artifacts/full-harness-suite-20261111-clean.log).

Independent Luna low verification against this attestation passed `rule.uncomparables` **11/11**, `rule.restricted.elementary` **3/3**, and `rule.restricted.top1000` **3/3**. Each passed comparator checks 29/29; oracle integrity passed before and after. Evidence: [`independent-final-three-clean-20261111/verification-summary.json`](../artifacts/independent-final-three-clean-20261111/verification-summary.json). The latest default-dispatch acceptances also pass: `misc.not_guilty` 6/6, `dates_times.dates.month_of_year` 5/5, and `psychology.mental_telepathy` 8/8, including the exact order of its same-span overlapping diagnostics.

After the gate passed, the bounded combined selected-rule benchmark passed its exact live preflight 3/3 and completed in 34.187 seconds under its 1,800-second cap. It used 25 paired samples and three warmups for three workloads. Results are in [`BENCHMARKS.md`](BENCHMARKS.md); they do not establish whole-program performance.

The accepted gate excludes **11 cases across six capability IDs**: `cli.config.no_passwd_home`, `cli.demo.json`, `cli.product_version_flag`, `cli.version.json_metadata`, `cli.version.verbose_runtime`, and `harness.oracle_smoke`. The Rust product-version flag is an authorized divergence; the other CLI cases remain unresolved or environment-specific. Full compatibility is not established.

## Known differences and gaps

- The requested Rust product `--version` text differs from upstream Proselint’s version output.
- JSON version output is now valid; the pinned response's `python: [3,12,3]` field is absent from Rust by design, so four exact cases remain mismatches. `version --verbose` also includes Python runtime details that Rust does not fabricate.
- Demo JSON includes an installation-specific Python source path that differs from Rust’s `demo.md` source label.
- `cli.config.no_passwd_home_absent` remains a confirmed Linux mismatch when the process has no HOME and no passwd entry.
- The manifest contains 1,436 cases. The current attempted scope excludes 11 cases across six capability IDs: `cli.config.no_passwd_home`, `cli.demo.json`, `cli.product_version_flag`, `cli.version.json_metadata`, `cli.version.verbose_runtime`, and `harness.oracle_smoke`. The symlink-loop capability is included in the passing 1,425-case cumulative gate. The product version flag is an authorized divergence; the other exclusions remain unresolved. The benchmark recorded in [`BENCHMARKS.md`](BENCHMARKS.md) is current for this fingerprint.
- Runtime independence passed for one Linux release binary/input in a minimal chroot with no Python or reference executable accessible; Windows/macOS and other execution paths were not covered.

Do not interpret a passing focused rule, selected configuration, cumulative scope, or benchmark as full Proselint compatibility. The exact case and capability ledger is maintained in [`status.json`](../status.json); historical verification attempts remain in [`status-history.jsonl`](../status-history.jsonl).

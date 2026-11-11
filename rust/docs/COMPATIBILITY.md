# Compatibility status

The reference is the pinned Proselint 0.16.0 CLI at revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`. The Rust program is a native implementation; this document describes verified behavior and known differences, not a claim of complete compatibility.

## Current verified scope

The latest full cumulative release gate passed at source fingerprint `5d37323e3e8d891dff2d53e8929641d74cb809e111d6d36534fbb27224b501f0`: **1,446/1,446 accepted cases across 153 capabilities**, from a 1,457-case manifest. Pinned Proselint 0.16.0 oracle integrity passed; the gate recorded zero mismatches, timeouts, candidate/oracle errors, or blockers. The Rust suite passed **914 tests across 145 targets**, comparator self-tests passed **29/29**, and the full Python harness passed **141/141**. The gate report is [`release-candidate-gate-20261111.json`](../artifacts/release-candidate-gate-20261111.json), with full attestation [`20261111T063908061867Z-build.attestation.json`](../artifacts/20261111T063908061867Z-build.attestation.json).

After that gate, the manifest gained no-config cases for `misc.not_guilty` and `dates_times.dates.month_of_year`. The current fingerprint is `877971c9905f980c9a48801f02ac1cc9b74173f11788c617e1479cdffb164d98`. The focused comparisons pass **6/6** and **5/5** respectively, both independently verified; each focused Rust CLI test passes **1/1** and each focused harness pair passes **2/2**. Current accepted scope is **1,450/1,461**; the last cumulative gate is stale by four cases. Evidence is in [`not-guilty-default-dispatch-20261111/`](../artifacts/not-guilty-default-dispatch-20261111/), [`independent-not-guilty-default-20261111/`](../artifacts/independent-not-guilty-default-20261111/), [`month-of-year-default-dispatch-20261111/`](../artifacts/month-of-year-default-dispatch-20261111/), and [`independent-month-of-year-default-20261111/`](../artifacts/independent-month-of-year-default-20261111/).

The bounded benchmark was run after the preceding parity gate passed. It used a 3/3 live preflight, 25 paired samples per workload, three warmups, and a 1,800-second overall deadline. The current source differs only by the later no-guilty cases/tests; the Rust binary and production rules are unchanged. Results are selected-workload measurements, not a whole-program claim; see [`BENCHMARKS.md`](BENCHMARKS.md).

The accepted gate excludes **11 cases across six capability IDs**: `cli.config.no_passwd_home`, `cli.demo.json`, `cli.product_version_flag`, `cli.version.json_metadata`, `cli.version.verbose_runtime`, and `harness.oracle_smoke`. The product version flag is an authorized divergence. JSON version metadata and verbose runtime output also differ from Python behavior. These exclusions and the current gate scope are recorded in [`status.json`](../status.json); complete Proselint compatibility is not established.

## Known differences and gaps

- The requested Rust product `--version` text differs from upstream Proselint’s version output.
- JSON version output is now valid; the pinned response's `python: [3,12,3]` field is absent from Rust by design, so four exact cases remain mismatches. `version --verbose` also includes Python runtime details that Rust does not fabricate.
- Demo JSON includes an installation-specific Python source path that differs from Rust’s `demo.md` source label.
- `cli.config.no_passwd_home_absent` remains a confirmed Linux mismatch when the process has no HOME and no passwd entry.
- The manifest contains 1,436 cases. The current attempted scope excludes 11 cases across six capability IDs: `cli.config.no_passwd_home`, `cli.demo.json`, `cli.product_version_flag`, `cli.version.json_metadata`, `cli.version.verbose_runtime`, and `harness.oracle_smoke`. The symlink-loop capability is included in the passing 1,425-case cumulative gate. The product version flag is an authorized divergence; the other exclusions remain unresolved. The benchmark recorded in [`BENCHMARKS.md`](BENCHMARKS.md) is current for this fingerprint.
- Runtime independence passed for one Linux release binary/input in a minimal chroot with no Python or reference executable accessible; Windows/macOS and other execution paths were not covered.

Do not interpret a passing focused rule, selected configuration, cumulative scope, or benchmark as full Proselint compatibility. The exact case and capability ledger is maintained in [`status.json`](../status.json); historical verification attempts remain in [`status-history.jsonl`](../status-history.jsonl).

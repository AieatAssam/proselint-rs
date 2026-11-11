# Compatibility status

The reference is the pinned Proselint 0.16.0 CLI at revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`. The Rust program is a native implementation; this document describes verified behavior and known differences, not a claim of complete compatibility.

## Current verified scope

The frozen release candidate at source fingerprint `1d080c6ed1e73e4c4aa941cb8d730e548313181f1e214e21d767de305b3131e5` passed the full release gate on 2026-11-11: **1,425/1,425 selected cases across 153 capabilities**, from a 1,436-case manifest. The pinned Proselint 0.16.0 baseline at revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c` passed integrity verification. The gate recorded zero mismatches, timeouts, candidate/oracle errors, or blockers. Rust checks and locked release build passed; the Rust suite passed **903 tests across 145 targets**; comparator self-tests passed **29/29**; and the full Python harness suite passed **132/132**.

Independent Luna live verification on the same full-gate attestation accepted uncomparables **11/11**, restricted.elementary **3/3**, and restricted.top1000 **3/3**. Their reports are [`uncomparables`](../artifacts/independent-current-release-rules-20261111-low/rule.uncomparables.json), [`restricted.elementary`](../artifacts/independent-current-release-rules-20261111-low/rule.restricted.elementary.json), and [`restricted.top1000`](../artifacts/independent-current-release-rules-20261111-low/rule.restricted.top1000.json). The full gate is [`release-gate-current-candidate-final-20261111.json`](../artifacts/release-gate-current-candidate-final-20261111.json), with attestation [`20261111T005654833296Z-build.attestation.json`](../artifacts/20261111T005654833296Z-build.attestation.json).

The bounded benchmark ran only after that parity pass. It used a 3/3 live preflight, 25 paired samples per workload, 3 warmups, and a 1,800-second overall limit. Results are limited to selected combined-rule workloads; see [`BENCHMARKS.md`](BENCHMARKS.md).

The accepted gate excludes **11 cases across six capability IDs**: `cli.config.no_passwd_home`, `cli.demo.json`, `cli.product_version_flag`, `cli.version.json_metadata`, `cli.version.verbose_runtime`, and `harness.oracle_smoke`. The product version flag is an authorized divergence. JSON version metadata and verbose runtime output also differ from Python behavior. Other exclusions remain gaps; the current gate does not establish complete Proselint compatibility. Exact case and capability state is recorded in [`status.json`](../status.json), and historical attempts remain in [`status-history.jsonl`](../status-history.jsonl).

## Known differences and gaps

- The requested Rust product `--version` text differs from upstream Proselint’s version output.
- JSON version output is now valid; the pinned response's `python: [3,12,3]` field is absent from Rust by design, so four exact cases remain mismatches. `version --verbose` also includes Python runtime details that Rust does not fabricate.
- Demo JSON includes an installation-specific Python source path that differs from Rust’s `demo.md` source label.
- `cli.config.no_passwd_home_absent` remains a confirmed Linux mismatch when the process has no HOME and no passwd entry.
- The manifest contains 1,436 cases. The current attempted scope excludes 11 cases across six capability IDs: `cli.config.no_passwd_home`, `cli.demo.json`, `cli.product_version_flag`, `cli.version.json_metadata`, `cli.version.verbose_runtime`, and `harness.oracle_smoke`. The symlink-loop capability has current focused independent evidence but awaits a passing cumulative gate. The product version flag is an authorized divergence; the other exclusions or gaps remain explicit in the ledger. The last benchmark is for the older candidate fingerprint and is not current for the source described here.
- Runtime independence passed for one Linux release binary/input in a minimal chroot with no Python or reference executable accessible; Windows/macOS and other execution paths were not covered.

Do not interpret a passing focused rule, selected configuration, cumulative scope, or benchmark as full Proselint compatibility. The exact case and capability ledger is maintained in [`status.json`](../status.json); historical verification attempts remain in [`status-history.jsonl`](../status-history.jsonl).

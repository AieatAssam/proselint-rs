# Compatibility status

The reference is the pinned Proselint 0.16.0 CLI at revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`. The Rust program is a native implementation; this document describes verified behavior and known differences, not a claim of complete compatibility.

## Current verified scope

The latest strict release gate passed **1,415 of 1,415 active live cases across 150 capabilities**, with no mismatches, errors, or timeouts. The manifest contains 1,426 cases; 11 are excluded and remain unresolved or intentionally divergent. The run used source fingerprint `96160ce82689b5363a896a36d09b0fa10703418c586d972e856eff24ffcc7518`; evidence is in [`status.json`](../status.json) and [`surrogate-key-equality-release-cumulative-final-20261110.json`](../artifacts/surrogate-key-equality-release-cumulative-final-20261110.json).

The inventory has 124 registration entries and 106 unique rule paths. Those paths have selected-configuration acceptance evidence. This does not establish exhaustive default-configuration behavior or every interaction between rules. The current benchmark measures only an explicitly selected pair of rules; see [`BENCHMARKS.md`](BENCHMARKS.md).

## Known differences and gaps

- The requested Rust product `--version` text differs from upstream Proselint’s version output.
- JSON version metadata and `version --verbose` include Python runtime details that Rust does not fabricate.
- Demo JSON includes an installation-specific Python source path that differs from Rust’s `demo.md` source label.
- `cli.config.no_passwd_home_absent` remains a confirmed Linux mismatch when the process has no HOME and no passwd entry.
- Eleven manifest cases remain outside the strict gate. Additional CLI, configuration, platform, default-configuration, and cross-rule behavior is not fully covered.
- The required runtime-independence check against an environment without Python or the reference executable has not yet been recorded.

Do not interpret a passing focused rule, selected configuration, cumulative scope, or benchmark as full Proselint compatibility. The exact case and capability ledger is maintained in [`status.json`](../status.json); historical verification attempts remain in [`status-history.jsonl`](../status-history.jsonl).

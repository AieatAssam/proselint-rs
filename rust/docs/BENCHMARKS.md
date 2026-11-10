# Python and Rust benchmark

# Latest verified release-candidate comparison (2026-11-10)

This run measured source fingerprint `96160ce82689b5363a896a36d09b0fa10703418c586d972e856eff24ffcc7518` against pinned Proselint 0.16.0 (`dbed789caae662d06c7c8a5a13dd31f1acd36f5c`). The explicit configuration enabled `misc.suddenly` and `psychology.p_equals_zero`; this is not default-config or whole-program performance. Live CLI parity passed all three workloads before timing. The runner collected 25 paired samples per workload after one warmup pair, with a 1,800-second overall deadline; total run time was 42.258 seconds (2026-11-10 06:54:34–06:55:16 UTC).

| Workload | Input | Python median | Rust median | Python / Rust |
|---|---:|---:|---:|---:|
| Small authored positive example | 82 B | 226.690 ms | 4.746 ms | 47.76× |
| Ordinary prose with sparse positive examples | 9,280 B | 253.030 ms | 7.999 ms | 31.63× |
| Dense generated two-rule trigger stream | 6,400 B | 316.221 ms | 17.387 ms | 18.19× |

The run used Cargo.lock SHA-256 `a246342e918651174f9fcadb2a6a54a2fea0b431fe6b479bc4da263bcad0a485`, release binary SHA-256 `e8b94ae987876a915ba15ae4781bdbcacb3fb80254b09e3a0f095b36d1401297`, and build attestation [`20261110T054820351775Z-build.attestation.json`](../artifacts/20261110T054820351775Z-build.attestation.json). RSS was unavailable. The complete samples, input hashes, environment and preflight evidence are in [`surrogate-key-equality-combined-benchmark-20261110.json`](../artifacts/surrogate-key-equality-combined-benchmark-20261110.json) (SHA-256 `466e42f6cd7210343d7ad75d62897af2b7cb064909bbeef0167e219603da931f`). Results apply only to these inputs, configuration, host and measurement method.

## Candidate freshness

The 2026-11-07 combined-profile run measures the Rust tree at source fingerprint `bd184c3775035296bb3dde5d8145e83fea429bcb43522aaa4ef5654813699770` against pinned Python Proselint 0.16.0. The earlier combined-profile run and isolated `misc.suddenly` figures later in this document are historical and use older candidates. Neither selected profile measures full default-config Proselint behavior.

Every benchmark run performs an exact live parity preflight before timing. The runner enforces an overall 30-minute deadline by default across build, preflight, and sampling (`--overall-timeout-seconds` changes the limit); expiration is reported as `BLOCKED` and produces no timing claim. On POSIX the supervisor terminates the benchmark process group and closes pipes; descendants that detach from that group are outside cleanup. The current Windows fallback terminates only the direct child and does not guarantee descendant cleanup. Correctness gates do not run benchmarks.


## Latest combined selected-rule comparison (2026-11-07)

This run compares the pinned Python executable and Rust CLI with only `misc.suddenly` and `psychology.p_equals_zero` enabled. It is a selected-rule CLI measurement, not whole-program parity or speed. It completed in 41.757 seconds under a 900-second overall deadline. Exact parity preflight passed all three representative workloads (small authored positive example, ordinary generated prose, and dense generated triggers); the runner also checked parity during every warmup and timed sample. It collected 25 paired samples per workload after three warmup pairs.

| Workload | Input | Findings | Python median | Rust median | Python / Rust | Python IQR | Rust IQR |
|---|---:|---:|---:|---:|---:|---:|---:|
| Small authored positive example | 82 B | 2 | 218.400 ms | 4.576 ms | 47.7× | 212.244–267.213 ms | 4.119–5.159 ms |
| Ordinary generated prose | 9,280 B | 128 | 250.153 ms | 7.092 ms | 35.3× | 240.100–279.112 ms | 6.358–7.413 ms |
| Dense generated trigger stream | 6,400 B | 512 | 278.862 ms | 12.601 ms | 22.1× | 271.699–283.672 ms | 12.218–13.326 ms |

The full report contains paired samples, per-sample output hashes and environment details: [`../artifacts/benchmark-combined-shared-matcher-20261107.json`](../artifacts/benchmark-combined-shared-matcher-20261107.json) (SHA-256 `b376d19fcdfb2cc9844f4834cbbedd45087bea1a137a84788f20d200b4a86465`). Its [release attestation](../artifacts/20261107T030926022281Z-benchmark-62f00bf8-release.attestation.json) records the CLI/helper hashes and selected build targets. Baseline revision: `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; Rust toolchain 1.99.1; Cargo.lock SHA-256 `2c259e8c4b7a13a83bfa7690425ad428efcc4622e7c7a1f48f8b2f8d052b3136`; release binary SHA-256 `d956cdd716d12eace13cff0ba250977949c100ff2493ddcc2794171633d6483b`. RSS was unavailable. The target-specific release build took 0.918 seconds and is excluded from CLI medians. The 900-second overall deadline is an upper bound; measured wall time was 41.757 seconds.


## Result

This comparison measures the accepted `misc.suddenly` rule with an isolated configuration. It does not measure the whole Proselint program or the Rust port as a whole. At that earlier measurement date, the port had selected compatibility coverage and most of the 124 registered rule entries were not implemented. Current coverage and unresolved CLI cases are tracked in [`../status.json`](../status.json).

All three inputs passed a live comparison against the pinned Python executable before timing. The runner then checked parity on every warmup and every measured CLI sample. The complete report, including all 25 raw paired samples per workload and measurement mode, is [`../artifacts/python-rust-benchmark-final-20261105.json`](../artifacts/python-rust-benchmark-final-20261105.json) (SHA-256 `0961da1d92b8b5215786b6095bb8d7273ecb2da64dca5464e7524c354f2574af`).

The table reports median elapsed time. `Python / Rust` is the ratio of the two medians. The fresh-process results include process startup and JSON CLI work. In-memory results are the internal elapsed time for 1,000 repeated checks; helper startup, file and configuration reads, and result serialization are outside that timed interval.

| Workload | Input | Fresh Python | Fresh Rust | Python / Rust | 1,000 in-memory scans: Python | Rust | Python / Rust |
|---|---:|---:|---:|---:|---:|---:|---:|
| Pinned upstream demo | 6,095 B, 2 findings | 215.418 ms | 2.551 ms | 84.4× | 1.416 s | 93.824 ms | 15.1× |
| Ordinary generated prose | 28,531 B, 4 findings | 231.017 ms | 3.373 ms | 68.5× | 3.754 s | 383.168 ms | 9.8× |
| Dense generated input | 21,504 B, 512 findings | 295.318 ms | 10.119 ms | 29.2× | 25.218 s | 2.146 s | 11.8× |

The raw report also records each sample's duration, execution order, output hashes, status, and distributions. For example, the fresh-process median interquartile ranges were 211.852–223.594 ms vs 2.447–2.693 ms on the upstream demo, 222.631–239.268 ms vs 2.901–4.759 ms on ordinary generated prose, and 271.360–308.567 ms vs 9.711–11.618 ms on the dense input. In-memory IQRs are included in the report.

## Method and inputs

- Each implementation used the same selected config: only `checks.misc.suddenly` enabled, `max_errors` 1,000, JSON output. Config SHA-256: `88952337fee39358c3cbaec5ce9f14e99ded7b53b7fb7b8f4162ad5aba54ab46`.
- Fresh-process runs used each CLI with the same config and document files. The runner excluded three paired warmup rounds, then recorded 25 paired samples per workload, alternating which implementation ran first. It measured process spawn through exit, including startup, reading, checking, JSON serialization, and output capture. Builds were completed and attested before timing; build time is excluded.
- The in-memory helper loaded the document and config before starting its timer, then ran 1,000 checks per sample. It used three paired warmups and 25 paired timed samples per workload with alternating order. Every returned diagnostics list and its locally computed canonical SHA-256 matched between Python and Rust.
- The runner performed an exact live CLI parity precheck on all three inputs. It also compared outputs after every warmup and timed CLI invocation. All 3 precheck cases passed; there were no mismatches or blockers.
- The upstream input is `proselint/demo.md` from the pinned baseline, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`, distributed under BSD-3-Clause. The two generated inputs are deterministic project fixtures. Their provenance, generation descriptions, license, lengths, and SHA-256 hashes are in the report and [`../harness/benchmark_inputs/workloads.json`](../harness/benchmark_inputs/workloads.json).

## Environment and reproducibility

The run occurred on 2026-11-05 from 10:08:33 to 10:25:01 UTC on Linux x86_64 (`7.2.9-arch1-1`), with four CPUs reported and CPU model `AMD RX-427BB with AMD Radeon(tm) R7 Graphics`. The pinned baseline was Proselint 0.16.0, Python 3.12.3, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`. The Rust candidate was a locked release build using rustc and Cargo 1.99.1. The report records the Python executable and source hashes, Rust binary and helper hashes, Cargo lock hash, source fingerprint, tool versions, parity report, and raw samples.

Peak memory was not measured. The report marks RSS unavailable because the run did not use a per-process sampler.

These numbers apply to this machine, configuration, rule, inputs, and measurement method. They should not be generalized to other rules, default configurations, full documents at other sizes, or overall application performance. Timing varies with host load; use the raw samples and spread alongside the medians.

## Combined selected-rule profile

This separate profile measures the explicit combination `misc.suddenly` and `psychology.p_equals_zero`. It uses a three-workload project-authored fixture set; every input contains positive examples for both rules. It does not enable the bundled default configuration. All three inputs passed exact live CLI comparison against the pinned Python executable before timing, and output/status parity was checked on every warmup and measured sample.

| Workload | Input / findings | Python CLI median | Rust CLI median | Python / Rust | Python IQR | Rust IQR |
|---|---:|---:|---:|---:|---:|---:|
| Small authored positive example | 82 B / 2 | 214.615 ms | 4.152 ms | 51.7× | 32.250 ms | 0.704 ms |
| Ordinary generated prose | 9,280 B / 128 | 243.326 ms | 6.404 ms | 38.0× | 15.554 ms | 0.640 ms |
| Dense generated trigger stream | 6,400 B / 512 | 273.034 ms | 12.736 ms | 21.4× | 20.665 ms | 1.886 ms |

The report contains all 25 paired samples per workload, alternating which CLI runs first. One warmup pair per workload is excluded. The measured region runs each CLI from process spawn through exit, including startup, file/config reading, checking, JSON serialization, and output capture. The complete report is [`../artifacts/combined-benchmark-full-20261106.json`](../artifacts/combined-benchmark-full-20261106.json) (SHA-256 `f5f20bc192b5962afecae69340f0927b5e828a938bf3e6d3177fc7a6139270c3`); the separate parity-only preflight is [`../artifacts/combined-benchmark-parity-only-20261106.json`](../artifacts/combined-benchmark-parity-only-20261106.json) (SHA-256 `eb3d6c973d03d8488f139d65ecfae5999c31768d96b6a446c31eb195a0fbf5f0`). Workload hashes, sizes, expected findings, generator descriptions, selected config, and positive examples are in [`../harness/benchmark_inputs/combined_workloads.json`](../harness/benchmark_inputs/combined_workloads.json).

The timed run completed in about 27 seconds on 2026-11-06 (UTC) on the same four-CPU Linux x86_64 host described above. The pinned baseline is Proselint 0.16.0, Python 3.12.3, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; the candidate is a locked release build with rustc and Cargo 1.99.1. RSS was unavailable. This profile measures only fresh-process CLI behavior; the in-memory table above applies only to the isolated `misc.suddenly` rule. The speed ratios describe these inputs on this host and do not establish whole-application speed.

# Python and Rust benchmark

## Candidate freshness

These are reproducible historical comparisons against pinned Python Proselint 0.16.0, not measurements of the current Rust tree or the full rewrite. The isolated-rule report used Rust source fingerprint `f0aa5422…`; the combined selected-rule report used `e4072d0c…`. The current parity-gated candidate has fingerprint `b502f9337…` (spelling.in_un gate, 2026-11-06). Because production source changed after both timing runs, their speed ratios must not be presented as current-candidate performance. Rerun the parity precheck and benchmark against the final frozen source after implementation is complete.


## Result

This comparison measures the accepted `misc.suddenly` rule with an isolated configuration. It does not measure the whole Proselint program or the Rust port as a whole. The port currently has selected compatibility coverage; most of the 124 registered rule entries are not implemented.

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

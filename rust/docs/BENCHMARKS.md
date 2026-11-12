# Python and Rust benchmark

## Current verified candidate (2026-11-12 UTC)

The cumulative release-profile gate passed **1,540/1,540 live cases across 158 capabilities**, with no mismatches, timeouts, oracle errors, candidate errors, or blocked cases. The manifest has 1,542 cases; two oracle-smoke cases remain excluded. The pinned baseline is Proselint 0.16.0 at revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`. Candidate release binary SHA-256: `19801fa7d84255c4b6b110fb772e308fe4ab426aa7bfb71dc955353fc7683bdb`. The gate passed **936 Rust tests across 146 targets**. Five stale harness assertions were corrected and the status pointer was refreshed to the current attestation; the full harness suite then passed **184/184** in **83.628s**, including runtime-independence. Its current-source release build attestation is [`20261112T091257283181Z-build.attestation.json`](../artifacts/20261112T091257283181Z-build.attestation.json), fingerprint `f6cb0e28272bda3edf7b413628e25cfb2494957e3559ebbb41d56bd3c8c67e28`; the strict gate ran immediately before those test-only corrections at fingerprint `659b54b6f7562d189cdf8ac8bb36b5f76320bf76ffe1757f1e191c518f8f6de2`. The test corrections changed no manifest cases, comparisons, candidate source, or binary.

Two bounded benchmark profiles completed after the gate. Both used live pinned-Python parity preflights (**3/3**), 25 paired fresh-process samples per workload, three warmups, and a 600-second overall limit. The combined profile enables only `misc.suddenly` and `psychology.p_equals_zero`; it measures fresh CLI runs. The separate `misc.suddenly` profile additionally measures 100 repeated in-memory checks per sample. Neither result represents bundled defaults or whole-program behavior.

| Combined CLI workload | Input | Findings | Python median | Rust median | Python / Rust | Python IQR | Rust IQR |
|---|---:|---:|---:|---:|---:|---:|
| Small authored positive | 82 B | 2 | 191.021 ms | 4.158 ms | 45.9× | 10.153 ms | 0.490 ms |
| Ordinary generated prose | 9,280 B | 128 | 203.532 ms | 6.862 ms | 29.7× | 16.850 ms | 1.095 ms |
| Dense generated triggers | 6,400 B | 512 | 230.704 ms | 14.363 ms | 16.1× | 10.571 ms | 5.574 ms |

The isolated-rule measurements report fresh-process medians and then 100-scan in-memory medians. In-memory timing excludes helper startup, file reads, setup, result hashing, and JSON output; each Python/Rust output and checksum matched.

| `misc.suddenly` workload | Input | Fresh Python | Fresh Rust | Fresh ratio | 100 scans Python | 100 scans Rust | In-memory ratio |
|---|---:|---:|---:|---:|---:|---:|---:|
| Pinned upstream demo | 6,095 B | 186.187 ms | 3.438 ms | 54.2× | 98.981 ms | 9.916 ms | 10.0× |
| Ordinary generated prose | 28,531 B | 188.594 ms | 4.059 ms | 46.5× | 305.721 ms | 42.784 ms | 7.1× |
| Dense generated input | 21,504 B | 225.213 ms | 12.896 ms | 17.5× | 2,096.027 ms | 207.639 ms | 10.1× |

The strict gate took **1,953.048s**: candidate checks/tests/build **889.671s**, cumulative live comparison **1,060.158s**, oracle integrity **2.478s**, and comparator self-tests **0.143s**. The combined benchmark took **34.125s** (setup/build 2.816s, preflight 6.276s, fresh-process samples 22.485s). The isolated-rule benchmark took **121.087s** (setup/build 2.766s, preflight 6.141s, fresh-process samples 22.316s, in-memory samples 87.143s). Host: Linux x86_64, four CPUs reported; oracle Python 3.12.3; Rust 1.99.1. RSS was unavailable because no per-process sampler is configured.

One initial attempt at the isolated-rule profile used 1,000 in-memory repetitions and reached its **600s** limit during the dense Python workload. It is `BLOCKED`, with no performance claim; the successful rerun used the profile's 100-repetition setting. Evidence: [cumulative gate](../artifacts/optimized-test-profile-cumulative-gate-20261112.json), SHA-256 `a938e0f8fce4786e2aeff55294df7c4944e697907032fd4f8677fa73129ace60`; [harness log](../artifacts/full-harness-suite-optimized-gate-20261112.log), SHA-256 `3b5f79f0f1a4171082c2dcaac6e97687299f752689452ac060e6cd9d5a06d896`; [combined benchmark](../artifacts/benchmark-combined-profile-gate-20261112.json), SHA-256 `9c3e114789d207d1bf6acfab61c25f45d29f6fdc1f8609d139608ca1a3d2fe48`; [isolated-rule benchmark](../artifacts/benchmark-misc-suddenly-100-repetitions-gate-20261112.json), SHA-256 `c713450c3e2a2377b29142e7f7b651ec802bf8e824ab04477b43a02f49453ad2`; [bounded-out attempt](../artifacts/benchmark-optimized-test-profile-gate-20261112.json), SHA-256 `30aba104efa32755d55969b2e379732e621de1a5ba12cf6b3fe7f1fab5464d20`.

These results describe the stated selected rules, inputs, configuration, and host. They are not default-config or universal speed claims.

## Previous verified candidate benchmark (2026-11-12 before five-extension checkpoint)

The release-profile cumulative gate passed **1,530/1,530 selected live cases across 158 capabilities**, with zero mismatches, timeouts, or oracle/candidate errors. The manifest contains 1,532 cases; two oracle-smoke cases remain explicitly excluded. Baseline: pinned Proselint 0.16.0 at revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`. Gate candidate binary SHA-256: `19801fa7d84255c4b6b110fb772e308fe4ab426aa7bfb71dc955353fc7683bdb`. The gate passed **931 Rust tests across 146 targets**. The full Python harness suite passed **179/179** in **88.709s** after three selected-config ID assertions were updated to exclude separately tested no-config cases. Those corrections changed no oracle outputs, comparator rules, manifest cases, or Rust source. A current-tree build attestation after those test-only corrections has fingerprint `eec3c0183e91016ef743a56e2d4d57883f063395e83affbde3b844e846d850a2` and the same binary SHA-256.

After parity passed, the bounded selected-rule CLI benchmark passed its live preflight **3/3** and completed in **35.055s** under a **600s** overall deadline. It used 25 paired fresh-process samples and three warmups per workload. The selected checks were `misc.suddenly` and `psychology.p_equals_zero`.

| Workload | Input | Python median | Rust median | Python / Rust | Python IQR | Rust IQR |
|---|---:|---:|---:|---:|---:|---:|
| Small authored positive example | 82 B | 193.470 ms | 4.355 ms | 44.4× | 7.107 ms | 0.740 ms |
| Ordinary generated prose | 9,280 B | 206.916 ms | 7.052 ms | 29.3× | 5.450 ms | 2.769 ms |
| Dense generated trigger stream | 6,400 B | 237.329 ms | 14.671 ms | 16.2× | 7.647 ms | 3.626 ms |

The gate took **1,691.682s**: locked Rust checks/tests/build 700.855s, live comparison 988.699s, oracle integrity 1.532s, and comparator checks 0.124s. Benchmark stages were setup/build 2.848s, parity preflight 6.495s, and timed measurements 22.885s. Host: Linux x86_64, four reported CPUs; oracle Python 3.12.3; Rust 1.99.1. RSS was unavailable because the harness has no per-process sampler. These timings cover only the stated selected-rule workloads on this host; they are not default-config or universal performance claims.

Evidence: [cumulative gate](../artifacts/five-default-dispatch-extension-gate-20261112-final.json), SHA-256 `6304910dae7d74599f04453506b2f4c5227303bdc64660f799316dd75c644b32`; [full harness log](../artifacts/full-harness-suite-five-default-dispatch-extension-final-20261112.log), SHA-256 `57368925cfb275d36759a42433492cf5b6f7c5ff2c2c99a7e9c2a5b713328f05`; [benchmark report](../artifacts/benchmark-five-default-dispatch-extension-20261112.json), SHA-256 `3c60a0d6cae7684db321486ad1994b8a2f53d79bbd5f471f2979222fa1f8962a`.

## Previous verified release candidate (2026-11-12 before five-extension checkpoint)

The release-profile cumulative gate passed **1,508/1,508 accepted cases across 158 capabilities**, from a 1,510-case manifest with two excluded oracle-smoke cases. There were no mismatches, timeouts, or oracle/candidate errors. The pinned baseline is Proselint 0.16.0 at revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`. Candidate source fingerprint: `4e0157d2b820f6aa65d91bc30299a0a21180840c788d7c99ae9a2916fadbae9f`; release binary SHA-256: `19801fa7d84255c4b6b110fb772e308fe4ab426aa7bfb71dc955353fc7683bdb`; Cargo.lock SHA-256: `f8e4ad390ee6793c9e7e90d70c91ec8a6da39eb00cc87fe464dba1c91e1068fd`. The gate passed **923 Rust tests across 145 targets**, and the complete harness suite passed **169/169**.

After the gate, the bounded combined selected-rule CLI benchmark passed its exact live parity preflight **3/3**. The run lasted **34.172s** under its 600-second overall deadline: setup/build 2.820s, preflight 6.191s, and timed samples 22.399s. It used 25 paired fresh-process samples after three warmups per workload. Only `misc.suddenly` and `psychology.p_equals_zero` were enabled.

| Workload | Input | Python median | Rust median | Python / Rust | Python IQR | Rust IQR |
|---|---:|---:|---:|---:|---:|---:|
| Small authored positive example | 82 B | 185.078 ms | 4.206 ms | 43.8× | 184.221–189.529 ms | 4.082–5.024 ms |
| Ordinary generated prose | 9,280 B | 197.637 ms | 6.585 ms | 30.0× | 195.386–204.278 ms | 6.398–9.722 ms |
| Dense generated trigger stream | 6,400 B | 236.029 ms | 13.496 ms | 17.0× | 228.474–244.461 ms | 13.041–16.468 ms |

The host was Linux x86_64 with four reported CPUs; the oracle used Python 3.12.3 and the candidate used Rust 1.99.1. RSS was unavailable because no per-process sampler is configured. The gate report is [`../artifacts/current-version-full-gate-20261112.json`](../artifacts/current-version-full-gate-20261112.json), SHA-256 `c7714d65d16b111df797017650c9eb2b67f50333395ad07482d3d6d7b2a5efda`; the benchmark report is [`../artifacts/benchmark-current-version-20261112.json`](../artifacts/benchmark-current-version-20261112.json), SHA-256 `1660ef30101753020a0f92c80645208e6cb063be7c47434c0cf66259d9370049`. These measurements cover selected rules and three workloads on one host; they are not default-config or universal performance claims.

## Latest verified candidate (2026-11-11 UTC)

The release-profile live gate passed **1,464/1,464 accepted cases across 153 capabilities**, with 11 explicit exclusions from the 1,475-case manifest and zero timeouts or mismatches. The reference is the immutable Proselint 0.16.0 baseline at revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`. Candidate source fingerprint: `8fc5bb76d0770f62dd3800ec07f1699d3f0f86eb8ca7a7d58056534e0d457c69`; release binary SHA-256: `2619f521682d662ebc0af3e23eac74207d148906d73e854b99e3d521bd961a24`; Cargo.lock SHA-256: `71e0eaa2629ee98f770a155cd73dad9a6dd31947cc9ea9e5312b533fd70a6a03`. The current lock resolves `ar_archive_writer` 0.5.5. The gate passed **923 Rust tests across 145 targets**, and the full harness suite passed **147/147** with the current attestation.

After that gate, a bounded comparison passed its exact live parity preflight **3/3** and completed in about **35.8 seconds** under the 1,800-second overall limit. It used 25 paired fresh-process samples and three warmups per workload. The workloads enabled only `misc.suddenly` and `psychology.p_equals_zero`; these measurements do not represent default configuration or whole-program performance.

| Workload | Input | Python median | Rust median | Python / Rust | Paired samples |
|---|---:|---:|---:|---:|---:|
| Small authored positive example | 82 B | 198.858 ms | 4.550 ms | 43.7× | 25 |
| Ordinary generated prose | 9,280 B | 219.586 ms | 7.291 ms | 30.1× | 25 |
| Dense generated trigger stream | 6,400 B | 261.394 ms | 16.134 ms | 16.2× | 25 |

The runner was Linux x86_64 with four reported CPUs. The oracle used Python 3.12.3 and Rust used toolchain 1.99.1. Setup/build took 3.301s, parity preflight 6.682s, and fresh-process samples 24.240s. RSS was unavailable because the harness has no per-process sampler. The gate report SHA-256 is `74d98416f86e536aa4150768e20f00f18b48caf778bd87dd3d34ccdea5810b14`; the benchmark report SHA-256 is `5a6a8b965cbd88ab6e12a592ee733da5c1dad57404a557d21ba97429a10abcbc`. These are selected-rule results on one host, not a universal speed claim.

## Current release candidate with collusion default dispatch (2026-11-11 11:12 UTC)

The release-profile cumulative gate passed **1,454/1,454 accepted cases across 153 capabilities**, from a **1,465-case manifest with 11 exclusions**, before this bounded benchmark. The pinned Python baseline is Proselint 0.16.0 at revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`. Candidate source fingerprint: `6ba1f33c62cf4592fb946b95ad0d6356390ac88c17273c3299173f3fe40f9264`; release binary SHA-256: `2619f521682d662ebc0af3e23eac74207d148906d73e854b99e3d521bd961a24`. Linux x86_64, four reported CPUs; Python 3.12.3 oracle and Rust 1.99.1. The run finished in **37.182s**, below the 1,800-second cap, with live parity preflight **3/3**, 25 paired samples, and three warmups per workload.

| Workload | Fresh Python median | Fresh Rust median | Python / Rust |
|---|---:|---:|---:|
| Small authored positive example | 197.016 ms | 4.691 ms | 42.0× |
| Ordinary generated prose | 209.986 ms | 6.835 ms | 30.7× |
| Dense generated trigger stream | 240.602 ms | 15.121 ms | 15.9× |

Only `misc.suddenly` and `psychology.p_equals_zero` were enabled. These results describe three selected-rule CLI workloads, not default-config or whole-program performance. RSS was unavailable. The release gate is [`release-candidate-gate-20261111-collusion-release.json`](../artifacts/release-candidate-gate-20261111-collusion-release.json), SHA-256 `6b6df8172de5daa4de04a2dcaa5f4471e4cf5df04afed10f91446df1460c3675`; the benchmark report is [`release-candidate-benchmark-20261111-collusion.json`](../artifacts/release-candidate-benchmark-20261111-collusion.json), SHA-256 `ff9abac679aeb3634a6b00dc139f6e880fa5afc297ea4d6efd20de1cb641443d`.

## Current release candidate after harness inventory correction (2026-11-11 09:44 UTC)

The current release gate passed **1,452/1,452 accepted cases across 153 capabilities**, from a 1,463-case manifest with 11 exclusions, before this bounded benchmark. Pinned Proselint 0.16.0 (revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`) was verified. Candidate source fingerprint: `9360dfeb3bc37d2cbeb18e14e56f280754bc822905a3ba128db6c34c446cd39f`; binary SHA-256: `2619f521682d662ebc0af3e23eac74207d148906d73e854b99e3d521bd961a24`. Linux x86_64, four reported CPUs; Python 3.12.3 oracle and Rust 1.99.1. The bounded run finished in **34.187s**, below the 1,800-second overall limit, with exact live preflight **3/3**, 25 paired samples and three warmups for each workload.

| Workload | Input | Fresh Python median | Fresh Rust median | Python / Rust |
|---|---:|---:|---:|---:|
| Small authored positive example | 82 B | 189.419 ms | 4.318 ms | 43.9× |
| Ordinary generated prose | 9,280 B | 206.944 ms | 7.001 ms | 29.6× |
| Dense generated trigger stream | 6,400 B | 231.429 ms | 13.649 ms | 17.0× |

Only `misc.suddenly` and `psychology.p_equals_zero` were enabled. This is selected-rule CLI performance, not default-config or whole-program performance. RSS was unavailable. Setup/build took 2.728s, parity preflight 6.026s, and timed samples 22.591s. Raw samples and commands are in [`release-candidate-benchmark-20261111-clean-harness.json`](../artifacts/release-candidate-benchmark-20261111-clean-harness.json), SHA-256 `c90013b093034dfa2a27b410e1c04cadf6d9df8c23efcb3b94ffb8cc4ebe44f8`. The full gate is [`release-candidate-gate-20261111-clean-harness.json`](../artifacts/release-candidate-gate-20261111-clean-harness.json), SHA-256 `39c5caae429cd56f5315e03dfe93940574b248561fa2a597600ccf7aeb5f95c3`.

## Previous candidate before harness inventory correction (2026-11-11 08:19 UTC)

The release candidate passed the full cumulative parity gate immediately before this benchmark: **1,452/1,452 accepted cases across 153 capabilities**, with 11 excluded cases and zero mismatches, timeouts, or process/oracle errors. The pinned baseline is Proselint 0.16.0, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; source fingerprint `277a3ddfb1d64ace2bc6817a1029d6ffbc8c9a0846b71140abe0f29ec6061d1c`; release binary SHA-256 `2619f521682d662ebc0af3e23eac74207d148906d73e854b99e3d521bd961a24`. The bounded run completed in 35.732s, used the exact 3/3 live preflight, 25 interleaved sample pairs and three warmups per workload, under a 1,800-second overall limit. Host: Linux x86_64, four reported CPUs; oracle Python 3.12.3; Rust 1.99.1.

| Workload | Input | Fresh Python median | Fresh Rust median | Python / Rust |
|---|---:|---:|---:|---:|
| Small authored positive example | 82 B | 197.617 ms | 4.720 ms | 41.9× |
| Ordinary generated prose | 9,280 B | 209.965 ms | 6.888 ms | 30.5× |
| Dense generated trigger stream | 6,400 B | 243.020 ms | 14.997 ms | 16.2× |

The comparison enables only `misc.suddenly` and `psychology.p_equals_zero`; it is not whole-program/default-config performance. Medians use 25 samples per side/workload. RSS was unavailable because the harness has no per-process sampler. Setup/build took 2.970s, live preflight 6.480s, and fresh-process measurements 23.560s. Raw durations, hashes, commands, and workload details are in [`release-candidate-benchmark-20261111-final.json`](../artifacts/release-candidate-benchmark-20261111-final.json), SHA-256 `733589697b41277a27f348a1e543599109442effa4cfb70573ae0c3cd98364f9`. The gate report is [`release-candidate-gate-20261111-final.json`](../artifacts/release-candidate-gate-20261111-final.json), SHA-256 `63a49ac73a86335c8ecebff4a830561a56f7f07035913f1d71d2876baf9dcd21`.

## Earlier release candidate (2026-11-11 03:56 UTC)

This section records an earlier checkpoint and is superseded by the current verified release candidate section below. The candidate passed the accepted-scope release gate at **1,432/1,432 cases across 153 capabilities** before measurement. Both profiles passed exact live parity preflight **3/3** against pinned Proselint 0.16.0, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`. Source fingerprint: `cb5262bf3e58c7207879a9c8de08dfdbb40d20cd7eaf1022e75ed1054a0093e2`; binary SHA-256: `2619f521682d662ebc0af3e23eac74207d148906d73e854b99e3d521bd961a24`; build attestation: [`20261111T035653523505Z-build.attestation.json`](../artifacts/20261111T035653523505Z-build.attestation.json), SHA-256 `2309de6623947a0e5c64b7957486d9c96faabd88c42ca1193b8cf853df7d0692`. Host: Linux x86_64, four reported CPUs; pinned Python 3.12.3. Each profile used 25 paired samples per workload, 3 warmups, and a 1,800-second overall deadline.

### Isolated `misc.suddenly`

Fresh-process timings include process startup, file/config reads, checking, JSON serialization, and output capture. In-memory timings cover 1,000 scans per sample; outputs and checksums matched in every sample. Total elapsed time was 782.470s: setup/build 3.138s, live preflight 6.645s, fresh-process samples 22.770s, and in-memory samples 747.341s.

| Workload | Input | Fresh Python | Fresh Rust | Ratio | In-memory Python (1,000 scans) | In-memory Rust | Ratio |
|---|---:|---:|---:|---:|---:|---:|---:|
| Pinned upstream demo | 6,095 B | 198.403 ms | 3.600 ms | 55.11× | 1,122.087 ms | 102.705 ms | 10.93× |
| Ordinary generated prose | 28,531 B | 200.331 ms | 4.092 ms | 48.95× | 3,136.952 ms | 435.078 ms | 7.21× |
| Dense generated triggers | 21,504 B | 243.563 ms | 14.136 ms | 17.23× | 22,214.980 ms | 2,271.964 ms | 9.78× |

Full samples and preflight evidence: [`benchmark-minimal-manifest-misc-suddenly-20261111.json`](../artifacts/benchmark-minimal-manifest-misc-suddenly-20261111.json), SHA-256 `9f55c8954de99dfbbbefa7c2f7ce1fc26f9a0c69632039555dfbd32e4eb8e097`.

### Combined selected-rule CLI profile

This profile enabled `misc.suddenly` and `psychology.p_equals_zero` only. Total elapsed time was 35.099s: setup/build 2.909s, preflight 6.385s, and timed samples 23.078s.

| Workload | Input | Python median | Rust median | Python / Rust |
|---|---:|---:|---:|---:|
| Small authored positive example | 82 B | 198.489 ms | 4.574 ms | 43.39× |
| Ordinary generated prose | 9,280 B | 208.732 ms | 7.144 ms | 29.22× |
| Dense generated trigger stream | 6,400 B | 236.616 ms | 13.969 ms | 16.94× |

Full samples and parity evidence: [`benchmark-minimal-manifest-combined-20261111.json`](../artifacts/benchmark-minimal-manifest-combined-20261111.json), SHA-256 `7973f954e19f91e8e00b9cdc1f3eef5660cbd125c3a1f5fb07e5b02d42ead038`.

RSS was unavailable. These are measurements of selected rules, explicit configurations, and the listed inputs on this host. They do not establish default-configuration, whole-program, or universal performance. Raw paired samples, command lines, hashes, environment, and parity evidence are in the linked JSON reports.

## Historical measurement (2026-11-10; prior harness fingerprint)

This earlier benchmark measured source fingerprint `96160ce82689b5363a896a36d09b0fa10703418c586d972e856eff24ffcc7518` before the runtime-independence harness test was added. It is historical; current-candidate results are recorded above. It used pinned Proselint 0.16.0 (`dbed789caae662d06c7c8a5a13dd31f1acd36f5c`). The explicit configuration enabled `misc.suddenly` and `psychology.p_equals_zero`; this is not default-config or whole-program performance. Live CLI parity passed all three workloads before timing. The runner collected 25 paired samples per workload after one warmup pair, with a 1,800-second overall deadline; total run time was 42.258 seconds (2026-11-10 06:54:34–06:55:16 UTC).

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

## 2026-11-11: month/year checkpoint bounded benchmark

After the current cumulative live gate passed, the combined selected-rule profile was measured against the pinned Python executable. The exact workload passed live parity preflight **3/3** and every warmup/measured invocation remained parity checked. The report contains **25 paired fresh-process samples per workload**, ran within the configured 1,800-second deadline, and took 33.207 seconds total (3.048s setup/build, 6.607s parity preflight, 23.552s measurement). Report: [`../artifacts/month-year-benchmark-combined-20261111.json`](../artifacts/month-year-benchmark-combined-20261111.json), SHA-256 `9c111b12bcde888a5105904b6c91ebb88452080088c97fb5771e10714db2ce37`.

| Workload | Python median | Rust median | Samples per side |
|---|---:|---:|---:|
| `combined_small` | 198.124 ms | 4.454 ms | 25 |
| `combined_ordinary_generated` | 208.985 ms | 7.449 ms | 25 |
| `combined_dense_generated` | 237.440 ms | 15.071 ms | 25 |

This is a fresh-process CLI comparison for the selected combined rules on the three listed workloads. It does not establish whole-program or universal speedup. Per-process RSS was unavailable because the harness has no process-attributed RSS sampler.

## 2026-11-11: current verified release candidate

The accepted-scope cumulative release gate passed **1,446/1,446 cases across 153 capabilities** before timing. Candidate fingerprint: `5d37323e3e8d891dff2d53e8929641d74cb809e111d6d36534fbb27224b501f0`; release binary SHA-256: `2619f521682d662ebc0af3e23eac74207d148906d73e854b99e3d521bd961a24`. Baseline: Proselint 0.16.0, revision `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`, Python 3.12.3. Host: Linux x86_64, four reported CPUs. Rust toolchain: 1.99.1. The selected combined CLI workload passed exact live parity preflight **3/3**. The benchmark completed in 34.745s under the 1,800-second overall deadline, with 25 paired fresh-process samples and three warmups per workload. Fresh-process timing includes process startup, configuration and file reads, scanning, serialization, and output capture.

| Workload | Input | Python median | Rust median | Paired samples |
|---|---:|---:|---:|---:|
| `combined_small` | benchmark profile input | 195.162 ms | 4.389 ms | 25 |
| `combined_ordinary_generated` | benchmark profile input | 205.735 ms | 6.858 ms | 25 |
| `combined_dense_generated` | benchmark profile input | 234.790 ms | 14.807 ms | 25 |

The full report includes raw paired samples, workload hashes, exact commands, environment, and spread: [`release-candidate-benchmark-20261111.json`](../artifacts/release-candidate-benchmark-20261111.json), SHA-256 `a9837eb924f29408c1fa084ea29eba1778aff77998b114d29e014c452e91479e`. RSS was unavailable because the harness has no per-process sampler. These are selected configurations and workloads; no whole-program or universal speed claim follows.

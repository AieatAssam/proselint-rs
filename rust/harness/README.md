# Compatibility harness

The harness compares the Rust CLI with the immutable Proselint `0.16.0` executable recorded in [`../oracle.lock.json`](../oracle.lock.json). It validates both program output and process behavior. Cases are derived from pinned upstream behavior; they do not use Rust output as their expected value.

Each capability moves through upstream specification, behaviorally failing tests, implementation tests, fresh live comparison, independent verification, and recorded evidence. The cumulative gate covers accepted cases only. A passing subset does not establish full Proselint compatibility.

The comparator has negative self-tests that deliberately introduce output, status, schema, and harness-integrity failures. Those checks ensure that mismatches are detected instead of normalized away. Live cases use a bounded two-worker pool, keep oracle and candidate runs sequential within each case, preserve manifest order in reports, and isolate Python bytecode caches per case.

Case environment overrides distinguish an absent `HOME` (`null`) from an empty `HOME` (`""`). Manifest `preconditions` are documentary, so platform-specific cases requiring a UID without a passwd entry must be run inside the stated namespace and stay outside the accepted cumulative gate until their parity is resolved.

Run the Python harness unit suite with `python3 -m unittest discover -s rust/harness/tests -p 'test_*.py'`. Keep this separate from focused capability comparisons; run it with the cumulative gate after shared CLI, config, matcher, or output changes and at the scheduled 5–10-capability checkpoint. For isolated capabilities, retain focused and independent live comparisons without repeating the full cumulative gate.

`gate.py` reruns pinned-oracle integrity, comparator self-tests, and the cumulative live comparison on every invocation. By default it also runs fmt, Clippy, all Rust tests, and a locked debug build. Use `--profile release` to run those same checks and compare the optimized release binary; this keeps the same case deadlines, two-worker pool, and exact output comparisons while avoiding debug-build scan overhead. Attestations record and validate the profile and binary path. Reuse an attestation only with the same profile and only when its source fingerprint, `Cargo.lock`, binary hash/path, and pinned toolchain still match; live comparison and oracle integrity are never reused.

For test-first RED cases, `gate.py --attest-candidate-for-red` runs locked fmt, Clippy, and build without requiring the intentionally failing Rust tests to pass. Its `focused_candidate_build` attestation is valid only for capability-scoped live comparisons; the cumulative gate rejects it because it does not prove `cargo test --locked`.

## Performance comparison

[`benchmark.py`](benchmark.py) supports an isolated `misc.suddenly` profile and a combined `misc.suddenly` + `psychology.p_equals_zero` profile. It checks exact parity for every measured input before collecting paired, alternating fresh-process CLI samples. The isolated profile also includes repeated in-memory scans; combined-profile results are CLI-only. Inputs, attribution, hashes, commands, raw samples, summaries, environment details, and parity evidence are recorded in each report.

Build, parity preflight, and measurements share a 1800-second default overall deadline (`--overall-timeout-seconds` can set another positive limit). Use `--parity-only` for a bounded preflight without timing. A timeout blocks the run and produces no performance result.

The benchmark reports only workload-specific results for its selected checks. Neither profile implies a whole-program or universal speedup. See [`../docs/BENCHMARKS.md`](../docs/BENCHMARKS.md).

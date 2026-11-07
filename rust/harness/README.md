# Compatibility harness

The harness compares the Rust CLI with the immutable Proselint `0.16.0` executable recorded in [`../oracle.lock.json`](../oracle.lock.json). It validates both program output and process behavior. Cases are derived from pinned upstream behavior; they do not use Rust output as their expected value.

Each capability moves through upstream specification, behaviorally failing tests, implementation tests, fresh live comparison, independent verification, and recorded evidence. The cumulative gate covers accepted cases only. A passing subset does not establish full Proselint compatibility.

The comparator has negative self-tests that deliberately introduce output, status, schema, and harness-integrity failures. Those checks ensure that mismatches are detected instead of normalized away.

Run the Python harness unit suite with `python3 -m unittest discover -s rust/harness/tests -p 'test_*.py'`. Keep this separate from focused capability comparisons; run it with the cumulative gate after shared CLI, config, matcher, or output changes and at the scheduled 5–10-capability checkpoint.

## Performance comparison

[`benchmark.py`](benchmark.py) supports an isolated `misc.suddenly` profile and a combined `misc.suddenly` + `psychology.p_equals_zero` profile. It checks exact parity for every measured input before collecting paired, alternating fresh-process CLI samples. The isolated profile also includes repeated in-memory scans; combined-profile results are CLI-only. Inputs, attribution, hashes, commands, raw samples, summaries, environment details, and parity evidence are recorded in each report.

The benchmark reports only workload-specific results for its selected checks. Neither profile implies a whole-program or universal speedup. See [`../docs/BENCHMARKS.md`](../docs/BENCHMARKS.md).

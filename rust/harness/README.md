# Compatibility harness

The harness compares the Rust CLI with the immutable Proselint `0.16.0` executable recorded in [`../oracle.lock.json`](../oracle.lock.json). It validates both program output and process behavior. Cases are derived from pinned upstream behavior; they do not use Rust output as their expected value.

Each capability moves through upstream specification, behaviorally failing tests, implementation tests, fresh live comparison, independent verification, and recorded evidence. The cumulative gate covers accepted cases only. A passing subset does not establish full Proselint compatibility.

The comparator has negative self-tests that deliberately introduce output, status, schema, and harness-integrity failures. Those checks ensure that mismatches are detected instead of normalized away.

## Performance comparison

[`benchmark.py`](benchmark.py) first checks exact parity for each measured input. It then compares the pinned Python CLI and release Rust CLI using paired, alternating fresh-process samples. A separate repeated in-memory scan measures the selected rule with process startup and input loading excluded from the internal timing. Inputs, attribution, hashes, commands, raw samples, summaries, environment details, and parity evidence are recorded in each report.

The benchmark reports only workload-specific results for the accepted `misc.suddenly` capability. It does not imply a whole-program or universal speedup. See [`../docs/BENCHMARKS.md`](../docs/BENCHMARKS.md).

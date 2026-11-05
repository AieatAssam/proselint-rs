# Python compatibility baseline

The compatibility reference is Proselint `0.16.0` at immutable upstream revision [`dbed789caae662d06c7c8a5a13dd31f1acd36f5c`](../oracle.lock.json). The pinned source tree SHA-256 is `73d46eb1a461a379eac0e3c3418e1f941b120a19e13d5ae0f4413ea2cee3b831`.

The reference's upstream test suite passed 351 tests and 15 subtests. The live comparator invokes the pinned executable and compares actual process results; stored snapshots and prior reports are evidence records, not substitutes for a live comparison. The oracle lock records the source, executable, and environment integrity data.

The inventory contains 124 registered entries across 106 unique check paths: 110 enabled by default and 14 disabled by default. See [`../inventory.json`](../inventory.json) for the ordered inventory. Rust support remains partial; see [`PROGRESS.md`](PROGRESS.md) for current coverage and [`BENCHMARKS.md`](BENCHMARKS.md) for measured results on the accepted benchmark capability.

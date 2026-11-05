# Migration execution plan

Follow the `rust/MIGRATION.md` state machine per capability. The immutable oracle, upstream tests, negatively tested comparator, and independent harness review are complete. Live cases now specify the no-findings JSON CLI contract and `annotations.misc`. The next phase is a minimal compiling Rust scaffold and behaviorally failing tests for the JSON CLI contract; no production capability is accepted until the full RED/GREEN/live/independent cycle passes.

After harness acceptance: build a native CLI/library scaffold; specify and verify input/output/position semantics; choose the first small real rule from the live rule inventory; then proceed through shared matcher helpers and bounded rule families. Configuration, multi-file/error behavior, all registered rules, and release behavior remain separate capability gates. Every accepted capability requires focused RED, GREEN, live baseline parity, independent verification on a frozen fingerprint, and recorded evidence.

The full port is not complete until every inventoried rule and required CLI behavior passes the cumulative live gate. Keep partial compatibility claims explicit.

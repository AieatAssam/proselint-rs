# Migration execution plan

Follow `rust/MIGRATION.md` state machine per capability. Current phase: pin the immutable Python oracle, run upstream tests, and build plus negatively test the live CLI comparator. No Rust production implementation begins before the comparator proves defect detection and the adviser reviews the evidence/contract.

After harness acceptance: build a native CLI/library scaffold; specify and verify input/output/position semantics; choose the first small real rule from the live rule inventory; then proceed through shared matcher helpers and bounded rule families. Configuration, multi-file/error behavior, all registered rules, and release behavior remain separate capability gates. Every accepted capability requires focused RED, GREEN, live baseline parity, independent verification on a frozen fingerprint, and recorded evidence.

The full port is not complete until every inventoried rule and required CLI behavior passes the cumulative live gate. Keep partial compatibility claims explicit.

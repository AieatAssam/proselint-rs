# Independent social_awareness.nword verification

Status: PASS (8/8 live differential cases; zero mismatches/errors).

Frozen source fingerprint: 546f0b68e3076502ba63b2d62622cc8b15d8224759d663debdeeae98a8340e3a (unchanged post-run: True).
Rust binary SHA-256: 177bcc41f31a3aca9ad2638c0a48e639533a1a54f6a59dba607fcce5776cc946; Cargo.lock SHA-256: 2c259e8c4b7a13a83bfa7690425ad428efcc4622e7c7a1f48b2f8d05278b3136.
Oracle revision: dbed789caae662d06c7c8a5a13dd31f1acd36f5c; source-tree SHA-256: 73d46eb1a461a379eac0e3c3418e1f941b120a19e13d5ae0f4413ea2cee3b831; executable SHA-256: b5f88a67377a8f3c32e1bffc41ca96fd8c111b127427459ffb2184e74c99af77; installed package-tree SHA-256: 6e6b1a625b7bf776549975cfd2d312d21906e5e6a96173d69c7596bbcc64d849; uv.lock SHA-256: 149ff01d4be05c243f3fb332c971091b961c77187af5be5261ba35386d0f29a1.
Oracle integrity verified before and after; metadata unchanged: True. Python 3.12.3, Proselint 0.16.0.

## Commands and results

- Pinned Cargo 1.99.1 build with --locked --manifest-path rust/Cargo.toml: exit 0; binary attested. Plain cargo attempt failed with exit 127 (not on PATH); first absolute Cargo attempt failed with exit 101 (rustc not on PATH); adding the pinned toolchain bin directory produced the successful build.
- python3 rust/harness/compare.py --self-test: exit 0, 23/23 checks passed.
- python3 -m unittest discover -s rust/harness/tests: exit 0, 57 tests passed in 12.481 s.
- Pinned cargo test --locked --manifest-path rust/Cargo.toml social_nword: exit 0, 5 passed, 0 failed; compile/test profile 57.06 s, test body 0.01 s.
- Live comparator for rule.social_awareness.nword using the attested candidate and assigned report path: exit 0, run 20261107T002039716537Z-180c816b, PASS 8/8, zero blocked/errors/mismatches; wall duration 9.769489 s.

## Cases and raw process durations

| Case | Oracle seconds | Rust seconds | Result |
|---|---:|---:|---|
| social_awareness_nword_upstream_example | 1.354804 | 0.009310 | PASS |
| social_awareness_nword_without_hyphen | 0.228614 | 0.008064 | PASS |
| social_awareness_nword_case_insensitive | 0.217914 | 0.008310 | PASS |
| social_awareness_nword_unanchored_inside_word | 0.218482 | 0.008157 | PASS |
| social_awareness_nword_space_is_near_miss | 0.228448 | 0.008988 | PASS |
| social_awareness_nword_unicode_codepoint_position | 0.257930 | 0.009373 | PASS |
| social_awareness_nword_quoted_suppressed | 0.229147 | 0.008308 | PASS |
| social_awareness_nword_max_errors_first_match | 0.222433 | 0.007805 | PASS |

Manifest comparison was unchanged: exact exit status, JSON stdout structure, byte-exact stderr; no expected-output or normalizer edits during verification. The inspected test diff adds exactly these eight manifest IDs and updates the capability count; no assertion was relaxed. Candidate nword source has no Python/process invocation or fixture-answer fallback. Runtime search found only std::process::ExitCode, no process spawning.

## Metadata and evidence limits

Assigned parity_verifier role configuration is gpt-6-luna/high. Direct runtime model/effort dispatch metadata is not exposed, so that configuration cannot be independently audited; no low-effort claim is made. Per-case durations are process wall times, not performance evidence.

Evidence files: preflight.json, postflight.json, candidate.attestation.json, comparator-self-test.json, harness-unittests.log, rust-focused-tests.log, cargo-build.log and live-report.json. Comparator raw case artifacts are in /home/dev/proselint-rs/rust/artifacts/20261107T002039716537Z-180c816b.

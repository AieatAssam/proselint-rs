# Live oracle and differential harness

The comparison authority is the executable installed from the archived source revision recorded in `../oracle.lock.json`. The harness never reads expectations from Rust. `setup_oracle.py` creates the immutable source under `rust/.cache/oracle-src/source`, provisions the project runtime and test dependencies from the checkout's `uv.lock`, verifies import paths, and runs the upstream pytest suite.

## Provision and verify

Run from the repository root:

```sh
python3 rust/harness/setup_oracle.py
python3 rust/harness/setup_oracle.py --verify-only
```

On Linux x86_64, setup bootstraps uv 0.13.7 into `rust/.cache/tools` after checking the archive SHA-256 recorded in the setup script. Other platforms need uv 0.13.7 available on `PATH`. The oracle environment explicitly selects CPython 3.12.3 with `uv sync --python 3.12.3`; setup checks the created interpreter version. The oracle lock contains relative paths; scripts resolve them from `rust/`. It records the source archive and tree hashes, `uv.lock` hash, installed executable and package hashes, all installed Python dependency versions, the Python/platform versions, and all imported `proselint` module paths. The verifier checks those values before each live batch.

Source hashing excludes only `.git`, `.venv`, `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.hypothesis`, and `.pdm-build`. These are VCS, environment, test, or package-build outputs. The installed `proselint` package has a separate content hash. The project declares `pdm-backend>=2.0.0`, which is not listed in `uv.lock`; uv resolves it in its isolated package-build environment. The observed backend version and metadata hash are recorded alongside the installed package tree hash, so any change blocks verification rather than changing the reference silently.

The reference environment sets `PYTHONPATH` unset, `PYTHONNOUSERSITE=1`, isolated `HOME` and `XDG_CONFIG_HOME`, and UTF-8 locale variables. Setup verifies every loaded module under `proselint.*` is within the oracle venv's installed package and checks the executable, interpreter version, build-backend metadata, package tree, and dependency set on every later integrity check.

## Cases and live comparison

Cases live in `cases/manifest.json`. Each entry records a unique case and capability ID, provenance, input bytes/layout, argv, output mode, expected oracle outcome category, timeout, environment/config scenario, workspace mutation policy, and comparison policy. File contents are base64 so the fixture bytes are unchanged. Every files-mode fixture must be referenced by an argv `{file:path}` marker or exact path token, or named in `unused_files`; output format, the single `--output-format` option, and stdout comparison mode must agree. The two `harness.oracle_smoke` cases exercise empty stdin and a file path containing spaces and non-ASCII characters. The first production case sets are `cli.json_no_findings` (three cases) and `rule.annotations.misc` (seven cases). Their live oracle self-comparisons remain harness-health evidence and are never counted as Rust parity.

Run comparator discrimination tests:

```sh
python3 rust/harness/compare.py --self-test
python3 -m unittest discover -s rust/harness/tests
```

Run the pinned oracle against itself as a live CLI smoke test:

```sh
python3 rust/harness/compare.py --oracle-self-check --live \
  --report rust/artifacts/oracle-self-check.json
```

That report has `scope: harness_health_only`; it is not a Rust acceptance result. The command below compares one capability against a Rust binary that was built and attested from the current source tree:

```sh
python3 rust/harness/compare.py --capability <capability-id> --live \
  --candidate-argv-json '["/absolute/path/to/proselint-rs"]' \
  --candidate-attestation rust/artifacts/<build-attestation>.json \
  --report rust/artifacts/<unique-run>.json
```

The attestation must match the current source fingerprint, `Cargo.lock` hash, binary path and binary hash. The candidate binary hash, attestation file hash, and attestation contents are rechecked after the batch, followed by Rust source and oracle integrity checks. Reference argv overrides must exactly equal the command pinned in `oracle.lock.json`; extra arguments that could redirect configuration or input are rejected. The candidate executable argv is a JSON array and is passed directly with `shell=False`; it can include additional executable arguments.

JSON comparison decodes UTF-8, rejects duplicate object keys and validates the upstream wire schema in `docs/wire-schema.md`. It ignores object key order and insignificant JSON whitespace only. Diagnostic arrays, field presence, nulls, values, order and duplicates remain exact. Text output compares bytes. Every case writes raw stdout/stderr, input bytes, fixture layout, commands, cwd, isolated environment, status, field diffs, and a replay manifest under a unique `rust/artifacts/<run-id>/` directory. A replay record gives the saved case manifest and exact candidate argv; replay requires the same candidate binary and build attestation to remain available.

The only implemented normalisation is opt-in, case-specific mapping of the temporary workspace URI prefix in top-level `result` source keys. It is disabled in the current cases. Raw output bytes are always retained.

## Cumulative gate and benchmark

```sh
python3 rust/harness/gate.py --live --report rust/artifacts/<unique-gate>.json
```

The strict gate reads `status.json` and includes only production capabilities explicitly marked `independently_verified` or `recorded`; harness-only cases are excluded. It fails if there are zero accepted capabilities, any accepted capability has no live cases, or any case is missing or mismatches. It runs comparator self-tests, `cargo fmt --check`, Clippy with warnings denied, Rust tests, and a locked Rust build, then creates an attestation and performs the live comparison. A zero exit means every required case passed and both source/oracle integrity checks held.

The benchmark entry point is present but deliberately does not produce timing data yet:

```sh
python3 rust/harness/benchmark.py --live --report rust/artifacts/<unique-benchmark>.json
```

It returns `BLOCKED` until a behavior-equivalent capability is independently verified and its cases are explicitly marked benchmark-eligible. No performance result or speed claim is implied by synthetic harness tests.

Reports validate nested case, execution, candidate, baseline, replay, self-test, error, and environment schemas before writing. Reports use statuses `PASS`, `MISMATCH`, `NOT_IMPLEMENTED`, `ORACLE_ERROR`, `CANDIDATE_ERROR`, `TIMEOUT`, and `BLOCKED`. The `counts` object counts case outcomes; `run_level_blocker` adds a separate `BLOCKED` count for setup, integrity, or source-freeze failures. Missing oracle, stale candidate, malformed manifest, zero selected cases, spawn error, unexpected process exit, and timeout cannot pass. Normal proselint finding status 1 and documented CLI error statuses 0–4 are comparison outcomes, not harness crashes.

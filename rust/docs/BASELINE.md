# Pinned baseline

Baseline source revision: `dbed789caae662d06c7c8a5a13dd31f1acd36f5c` (checkout HEAD at migration start; tracked upstream tree clean).

The upstream package metadata declares proselint `0.16.0`, Python `>=3.10`, and `pytest` as its test command (`pyproject.toml`). Dependency lockfiles present: `uv.lock` and `flake.lock`; their SHA-256 hashes at reconnaissance were `149ff01d4be05c243f3fb332c971091b961c77187af5be5261ba35386d0f29a1` and `98de6c7a434e4e22070bd0b916489ba4342e0fa3ce74320251e8331abbff9ba7` respectively. `pyproject.toml` SHA-256: `daa233f8ba41440d1b60e77217a1b23236934eed7e882320d57167a391275cd3`.

The reference source is materialized under ignored `rust/.cache/oracle-src/source` from `git archive` at the pinned revision. The isolated environment uses Python 3.12.3 and uv 0.13.7, with package imports confirmed inside the snapshot and upstream pytest passing (351 tests plus 15 subtests; exit 0). See `rust/oracle.lock.json` for source/archive digests, locked dependency versions, environment setup and test output paths. The comparator and oracle self-check were independently verified; this is harness health only, not Rust parity.

At reconnaissance, `uv`, `pytest`, and Cargo were not on PATH, and importing the live checkout failed because `google-re2` was unavailable. A repository-local Rust toolchain has since been installed under ignored `rust/.cache/`: rustc 1.99.1 (`58cf8f9c3`, 2026-10-13) and cargo 1.99.1 (`5f94df478`, 2026-08-27). It does not modify system PATH or system tools.

The reference must be materialized from this immutable revision outside the working source tree. Python source and lockfiles remain unchanged. The initial tracked upstream files were clean; supplied untracked migration scaffolding is not part of the oracle.

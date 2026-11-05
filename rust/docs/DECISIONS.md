# Decisions and unresolved compatibility questions

- Baseline target: clean tracked upstream HEAD `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; migration scaffolding is untracked and excluded.
- Runtime scope: native Rust CLI and reusable Rust library; Python remains development-only for the pinned oracle and harness.
- Dependency policy: preserve the proselint source snapshot and its `uv.lock` exactly as the oracle; resolve current compatible Rust crate dependencies and commit `Cargo.lock` when the Rust crate is created.
- No compatibility deviations have been approved.
- The isolated Python oracle and Rust 1.99.1 toolchain are provisioned under ignored `rust/.cache`; setup commands and integrity metadata are recorded in the harness and oracle lock.
- Named Luna roles were launched through the collaboration agent interface. Their supplied role configuration specifies `gpt-6-luna` at high effort, but child sessions did not expose model/effort metadata, so effective routing cannot be independently audited. The Sol adviser session exposed `gpt-6.1-sol`; its effort value was not visible, while its role configuration specifies low. No model substitution was made.
- Git author identity is configured locally from the connected GitHub profile and the email the user supplied. The user authorized committing and pushing migration milestones.

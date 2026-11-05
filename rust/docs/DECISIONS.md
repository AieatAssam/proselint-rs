# Decisions and unresolved compatibility questions

- Baseline target: clean tracked upstream HEAD `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; migration scaffolding is untracked and excluded.
- Runtime scope: native Rust CLI and reusable Rust library; Python remains development-only for the pinned oracle and harness.
- No compatibility deviations have been approved.
- Environment/tooling remains unresolved until the isolated oracle and Rust build tools are provisioned or an exact blocker is recorded.
- Agent model routing is configured in `.codex/agents/*.toml`; visible per-session model/effort metadata has not yet been confirmed by the CLI.

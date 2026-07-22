# AGENTS.md

Agent instructions for **glance** live in two files — start there:

- [**CLAUDE.md**](CLAUDE.md) — build/verify commands, conventions, and architecture gotchas (the operational guide).
- [**CONTRIBUTING.md**](CONTRIBUTING.md) — where docs live, the working rhythm, and the perf/vendoring rules.

Quick start: `cargo fmt && cargo clippy --all-targets && cargo test < /dev/null` must pass before any commit. (Run tests with stdin redirected — the no-file path reads stdin and can otherwise hang.)

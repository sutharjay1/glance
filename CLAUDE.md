# CLAUDE.md — glance

Fast terminal markdown viewer (Rust). This is the quick operational guide for agents; see
[CONTRIBUTING.md](CONTRIBUTING.md) for where docs live and the working rhythm, and
[docs/adr/](docs/adr/) for why decisions were made.

## Build & verify (run before every commit)

- `cargo fmt` · `cargo clippy --all-targets` (must be clean — CI is `-D warnings`) · `cargo test < /dev/null`
- **Always `cargo test < /dev/null`.** The no-file `run(&[])` path reads stdin, so a bare `cargo test` can hang in a non-EOF / backgrounded shell.
- `cargo install --path .` — reinstall the `glance` binary to `~/.cargo/bin` after changes.
- `glance --timing <file>` — prints parse+layout time (first-paint proxy; keep well under 80 ms).

## Conventions

- Conventional commits (`feat(scope):`, `fix:`, `docs:`). **No AI-attribution trailers** in commits; PR bodies are summary + test plan only.
- Match surrounding style: `//!` module docs + `///` item docs; keep pure/testable logic separate from I/O and unit-test the pure part (snapshots via `insta` at `tests/snapshots/`, widths 44/80/120).
- Binary size is a product metric (release profile is `lto`/`strip`/`panic=abort`). **Size-spike a new dep** — add it, measure the `--release` delta, decide, then wire it — and record material ones in an ADR (e.g. [ADR 0006](docs/adr/0006-bundle-tls-for-remote-images.md)).
- Never put syntax highlighting, grammar loading, or image fetch/decode on the first-paint path ([ADR 0004](docs/adr/0004-perf-architecture-upfront.md)) — that is mdterm's core bug. Do it on a background worker.

## Architecture gotchas

- Three background workers (auto-reload, syntect highlight, image fetch) share one event-loop poll; each drains its channel per tick and patches the layout in place.
- `Block::Prerendered(Vec<Line>)` is the seam for ports (JSON, mermaid) to reuse the whole viewer pipeline — produce `Vec<Line>`, wrap it, done.
- `crossterm` reads keys from `/dev/tty`, **not** stdin fd 0 — that's what lets streaming (`llm | glance`) take the document on stdin while keys keep working.
- Inline `$…$` math is transformed on the **raw source before pulldown** (`md::math::preprocess_math`, called in `md::parse::parse`); otherwise a subscript `_` is parsed as markdown emphasis.

## Tracking

Update Linear (team Jay, project `glance`, issues `JAY-*`) + [`ROADMAP.md`](ROADMAP.md) + [`docs/STATUS.md`](docs/STATUS.md) as modules land; add an ADR for a new decision.

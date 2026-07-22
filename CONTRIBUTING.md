# Contributing to glance

Solo/small-team workflow. This file tells a human *or* an agent how to navigate the project.

## Where things live
| You want… | Look at |
|---|---|
| The overall plan & phase status | [`ROADMAP.md`](ROADMAP.md) |
| Why a decision was made | [`docs/adr/`](docs/adr/) |
| What "done" means (feature gate) | [`docs/parity-checklist.md`](docs/parity-checklist.md) |
| What happened recently | [`docs/STATUS.md`](docs/STATUS.md) |
| Live issues / current cycle | Linear project `glance` (team Jay) |
| The full approved plan | `~/.claude/plans/glance-build-mellow-pascal.md` |
| Benchmarks vs the reference | `docs/benchmarks.md` (Phase 0) |
| Attribution for vendored code | `vendor/NOTICE` |

## Working rhythm
1. Pick the next task from `ROADMAP.md` / the Linear cycle.
2. **TDD for pure logic** (`cargo test`, red-green), **`insta` snapshots** for paint, **`expectrl` PTY** for the event loop.
3. Update the Linear issue status as you go; tick the checkbox in `ROADMAP.md` / parity-checklist when a phase item lands.
4. Add a `docs/STATUS.md` entry at the end of a session.
5. New decision worth remembering → add an ADR.

## Conventions
- `cargo fmt` + `cargo clippy -D warnings` must pass. CI gates fmt/clippy/test/insta/PTY (ubuntu + macos; windows non-gating).
- Vendored modules stay isolated in `src/ports/` and `src/term/images/`, adapted only at seams. Keep `vendor/NOTICE` current.
- Perf is a feature: don't put syntax highlighting or grammar loading on the startup path. Keep first-paint < 80 ms (`--timing`).

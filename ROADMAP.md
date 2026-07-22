# glance — Roadmap

> Fast terminal markdown viewer in Rust. Parity with `bahdotsh/mdterm` v2.0.0 + fixes for its
> four weaknesses (slow first paint, blocking stdin, broken copy, no callouts).
> Launch pitch: *"the markdown viewer that renders your LLM's output live."*

**Source of truth:** this file (durable) + Linear project `glance` (live issues/cycles).
**Full plan:** `~/.claude/plans/glance-build-mellow-pascal.md`
**Decisions:** [`docs/adr/`](docs/adr/) · **Parity gate:** [`docs/parity-checklist.md`](docs/parity-checklist.md) · **Dev log:** [`docs/STATUS.md`](docs/STATUS.md)

Legend: ⬜ not started · 🟨 in progress · ✅ done

---

## Phase 0 — benchmark + vendor survey  ⬜
Ground the README comparison table and de-risk the vendored modules.
- ⬜ Build reference (`cargo build --release`), hyperfine launch→render (pipe + export) → `docs/benchmarks.md`
- ⬜ Skim `viewer.rs` key handling → seed `docs/parity-checklist.md`
- ⬜ Vendor survey: read `image.rs`/`diagram.rs`/`json.rs`/math tables; deps + entry points; draft `vendor/NOTICE`
- ⬜ Copy reference `test.md` → `tests/fixtures/`
**Exit:** benchmarks recorded; parity checklist seeded; vendor survey + NOTICE drafted; fixture committed.

## Phase 0.5 — scaffold  ⬜
- ⬜ `cargo init --bin`; deps; release profile (`lto`, `codegen-units=1`, `strip`, `panic=abort`)
- ⬜ CI (fmt, clippy, test, insta, PTY); matrix ubuntu+macos gating, windows non-gating
- ⬜ `cargo-dist` release config

## Phase 1 — viewer core (week 1–2)  ⬜  ← risky foundation
Term layer → parse → layout → paint → render → navigation → pipe mode → config.
**Exit:** `glance README.md` first-paint < 80 ms; smooth scroll; clean pipe; all §5 nav keys; snapshots green at 44/80/120.

## Phase 2 — interactivity (week 3)  ⬜
Search, TOC, fuzzy filter, link picker + local-file nav, copy stack (OSC 52 + fallbacks + `p` + click), help, theme toggle + OSC 11, line numbers, tabs, auto-reload.
**Exit:** parity on nav/copy/link UX; copy verified macOS, Linux X11+Wayland, SSH.

## Phase 3 — highlight + images (week 4)  ⬜
syntect (lazy, worker, viewport-first) + Kitty & half-block image ladder (vendored).
**Exit:** code-heavy + image-heavy docs still first-paint < 80 ms.

## Phase 4 — differentiators (week 5)  ⬜
Streaming stdin + follow (the launch demo), slide mode, HTML export.
**Exit:** `llm | glance` renders live + auto-follows; slides navigate; `--export html` works.

## Phase 5 — ports + launch (week 6)  ⬜
Vendor + adapt mermaid/json/math; README parity + benchmark tables; vhs GIFs; crates.io + Homebrew + `cargo-dist` binaries; Show HN.
**Exit:** all 25 reference features present; installable; launch assets published.

---

### The four weaknesses we're fixing (thesis)
| # | mdterm weakness | Fixed in |
|---|---|---|
| 1 | Slow first paint (eager syntax load + whole-doc highlight) | Phase 1 (viewport-first) + Phase 3 (lazy syntect) |
| 2 | Blocking stdin (`read_to_string`, no streaming) | Phase 4 (streaming + follow) |
| 3 | Broken copy off macOS/X11 | Phase 2 (OSC 52 + full fallback chain) |
| 4 | No GitHub callouts | Phase 1 (callout panels) |

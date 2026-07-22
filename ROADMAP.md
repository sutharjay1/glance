# glance — Roadmap

> Fast terminal markdown viewer in Rust. Parity with `bahdotsh/mdterm` v2.0.0 + fixes for its
> four weaknesses (slow first paint, blocking stdin, broken copy, no callouts).
> Launch pitch: *"the markdown viewer that renders your LLM's output live."*

**Source of truth:** this file (durable) + Linear project `glance` (live issues/cycles).
**Full plan:** `~/.claude/plans/glance-build-mellow-pascal.md`
**Decisions:** [`docs/adr/`](docs/adr/) · **Parity gate:** [`docs/parity-checklist.md`](docs/parity-checklist.md) · **Dev log:** [`docs/STATUS.md`](docs/STATUS.md)

Legend: ⬜ not started · 🟨 in progress · ✅ done

---

## Phase 0 — benchmark + vendor survey  ✅ (2026-07-22)
Ground the README comparison table and de-risk the vendored modules.
- ✅ Built reference (release, 9.7 MB, ~20 s); hyperfine → [`docs/benchmarks.md`](docs/benchmarks.md). **Finding:** mdterm's ~60 ms is *fixed startup cost* (eager syntax/theme load), flat across doc size → validates lazy-highlight design.
- ✅ Mined `viewer.rs` key handling → [`docs/parity-notes-from-source.md`](docs/parity-notes-from-source.md); folded gaps into the checklist (added `m` mouse-toggle, JSON-viewer keys, checkbox-toggle + OSC 22 behaviors; clarified accent).
- ✅ Vendor survey → [`docs/vendor-survey.md`](docs/vendor-survey.md) + [`vendor/NOTICE`](vendor/NOTICE). **Finding:** `image.rs` = Low coupling (portable as-is, P3); `json.rs` + `diagram.rs` = must vendor **together** (json reaches into diagram's `Canvas`); math = copy verbatim.
- ✅ Copied reference `test.md` → `tests/fixtures/` (+ generated `big-5k.md`).
**Exit:** ✅ benchmarks recorded; parity notes captured; vendor survey + NOTICE drafted; fixtures committed.

## Phase 0.5 — scaffold  ✅ (2026-07-22)
- ✅ `cargo init --bin` + `lib.rs`/thin `main.rs` split (logic testable without spawning); release profile (`lto`, `codegen-units=1`, `strip`, `panic=abort`) → 295 KB scaffold binary
- ✅ First TDD unit: `text::width` ASCII fast path (plan §8 hotspot) — 7 tests green, fmt+clippy clean
- ✅ CI (`.github/workflows/ci.yml`): fmt/clippy/test, ubuntu+macos gating, windows non-gating (insta/PTY wired in Phase 1)
- ◐ Deps added incrementally per phase (unicode-width now) to keep builds fast; `cargo-dist` config deferred to Phase 5 (launch)

## Phase 1 — viewer core (week 1–2)  🟨  ← risky foundation
Term layer → parse → layout → paint → render → navigation → pipe mode → config. Built one TDD module per loop iteration.
- **term:** ✅ `caps` · ✅ `ansi` (downsampling) · ✅ `osc` (8/52/11) · ✅ `input` (crossterm→Event map) · ⬜ `TerminalGuard` (with event-loop wiring, PTY-tested)
- **md:** ✅ `parse` · 🟨 `layout` (wrap/indents/callouts/tokenizer done; `(block,width)` cache + viewport-first + DocLayout indices + tables next) · ✅ `highlight` (micro-tokenizer: 6 langs, wired into code blocks)
- **render/view:** ✅ `paint` (ANSI + OSC 8 link runs + `(url)` fallback) · ⬜ `render` (damage diff) · ⬜ `state`+`app` (event loop) · ⬜ navigation
- 🎉 **end-to-end pipe render works**: `glance file.md` parses→lays out→paints (colored on TTY, clean plain when piped). Interactive TUI still to come.
- **modes/cli:** ⬜ pipe/TTY · ⬜ `cli` (lexopt) · ⬜ `config` (toml)
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

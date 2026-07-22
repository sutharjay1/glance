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

## Phase 1 — viewer core (week 1–2)  ✅ (2026-07-22) — interactive TUI, 112 tests + 4 snapshot goldens, 35× faster than mdterm
Term layer → parse → layout → paint → render → navigation → pipe mode → config. Built one TDD module per loop iteration.
- **term:** ✅ `caps` · ✅ `ansi` (downsampling) · ✅ `osc` (8/52/11) · ✅ `input` (crossterm→Event map) · ⬜ `TerminalGuard` (with event-loop wiring, PTY-tested)
- **md:** ✅ `parse` · ✅ `layout` (wrap/indents/callouts/tokenizer + `DocLayout` indices; cache/viewport/tables deferred as optimizations) · ✅ `highlight` (6 langs, wired)
- **render/view:** ✅ `paint` · ✅ `render` (damage diff + sync output) · ✅ `state` (navigation) · ✅ `app` (event loop + `TerminalGuard` + panic hook)
- 🎉 **interactive TUI works**: `glance file.md` opens an alt-screen viewer — scroll/page/`g`/`G`/`[`/`]` heading jumps/wheel, quits on `q`, restores the terminal cleanly (PTY-verified). Piped/`--pipe` → clean render. **Remaining for Phase 1 exit:** proper `cli` (lexopt) + `config` (toml), insta snapshots at 44/80/120, first-paint `--timing` gate.
- **modes/cli:** ✅ pipe/TTY dispatch · ✅ `cli` (lexopt) · ✅ `config` (toml, CLI-overrides) · ✅ `--timing` (**0.92 ms** first-paint, 35× faster than mdterm end-to-end; 831 KB binary) · ⬜ insta snapshots (44/80/120)
- 🏆 **Perf thesis proven**: glance 1.7 ms vs mdterm 59.1 ms pipe render (35×); binary 831 KB vs 9 MB (11×). See [docs/benchmarks.md](docs/benchmarks.md). Installed to `~/.cargo/bin/glance`.
**Exit:** `glance README.md` first-paint < 80 ms; smooth scroll; clean pipe; all §5 nav keys; snapshots green at 44/80/120.

## Phase 2 — interactivity (week 3)  ✅
- ✅ **search**: `Search` core (regex + literal fallback, `n`/`N`, jump+center) + `/` prompt input mode + live reverse-video match highlighting + `query n/m` status line
- ✅ TOC (`o`) · ✅ fuzzy filter (`:`) · ✅ link picker (`f`) + local-file nav (follow `.md`, platform opener) + `Backspace` history
- ✅ copy stack (OSC 52 first + pbcopy/wl-copy/xclip/xsel/clip + `c`/`Y`/`p` + toasts) — **fixes weakness #3**; ✅ click-to-copy (left-click a code block → copy + toast)
- ✅ help overlay (`h`/`?`) · ✅ theme toggle (`t`) · ✅ line numbers (`l`, gutter + `-l`/config) · ✅ tabs (`Tab`/`Shift+Tab`, per-file scroll, `[n/N name]` status) · ✅ auto-reload (`notify` + 120 ms debounce, preserves scroll + search) · ✅ OSC 11 auto-theme (bg query + DSR fence → luminance; `-T`/config overrides)
**Exit:** parity on nav/copy/link UX; copy verified macOS, Linux X11+Wayland, SSH (interactive paths PTY-tested in a later pass).

## Phase 3 — highlight + images (week 4)  ✅
- ✅ syntect highlighter **core** (`md::syntect_hl`, parsing-only, scope→`Role`, lazy `OnceLock` off the startup path) + ✅ background highlight worker (`view::highlighter`, visible-first, in-place patch)
- ✅ image ladder: capability probe (`ImageProtocol`) · half-block renderer (`▀` fg/bg pixels; `Style` gained `fg`/`bg` RGB) · Kitty encoder (chunked base64 PNG APC) · standalone-image detection (`ImageRef`) · **background fetch/decode/render worker** (`view::images`: `ureq` http/https + fs local, decode, half-block, cached by (url,cols)) · **relayout-on-ready** (placeholder→N rows via `layout_document_with` + resolved-images map; scroll clamped). Kitty *display* integration (raw passthrough + row reservation) is a documented fast-follow — encoder is done+tested.
**Exit met:** 4.1 MB total (2.2× < mdterm), first-paint **0.65 ms** / **13 ms** (5k-line) — all heavy work off the hot path.

## Phase 4 — differentiators (week 5)  ✅
- ✅ **streaming stdin** (`src/stream.rs`): the `llm | glance` live-render demo. Reader thread → channel; `StreamState.append` re-parses only the active tail past the last **fence-aware** stable boundary; auto-follow the bottom, scroll-up pauses with a `▼ paused (G to follow)` pill, `G`/End resumes. Keys come from `/dev/tty` (crossterm), so piped stdin + interactive input coexist.
- ✅ **slide mode** (`-s`, `view::slides`): split on `---` (pure `split_slides` + clamped `Slides` nav); one slide/screen, vertically centered, `slide n/N` footer; `→`/Space/l/j next, `←`/h/k/b prev, g/G ends.
- ✅ **HTML export** (`--export html`, `src/export.rs`): pulldown-cmark HTML + **inlined** theme CSS (brand accent `#FF5800`, `color-scheme` dark/light) → a single self-contained file to stdout, no external refs.
**Exit met:** `llm | glance` renders live + auto-follows; slides navigate; `--export html` produces a standalone themed file.
Streaming stdin + follow (the launch demo), slide mode, HTML export.
**Exit:** `llm | glance` renders live + auto-follows; slides navigate; `--export html` works.

## Phase 5 — ports + launch (week 6)  🟨
- ✅ **math port** (`src/md/math.rs`): inline `$…$` LaTeX → Unicode (Greek, operators/relations, arrows, super/subscripts). Pre-parse pass (`preprocess_math`) transforms math on the raw source **before** pulldown so subscript `_` isn't mis-read as emphasis; code spans/blocks protected; a mathy-content heuristic leaves `$5` currency literal. 12 tests. · ⬜ JSON viewer (`glance data.json`) · ⬜ mermaid box-art
- ⬜ **Launch** (needs the user — outward-facing): README parity + benchmark tables; vhs GIFs; crates.io + Homebrew + `cargo-dist` binaries; Show HN.
**Exit:** all reference features present; installable; launch assets published.

---

### The four weaknesses we're fixing (thesis)
| # | mdterm weakness | Fixed in |
|---|---|---|
| 1 | Slow first paint (eager syntax load + whole-doc highlight) | Phase 1 (viewport-first) + Phase 3 (lazy syntect) |
| 2 | Blocking stdin (`read_to_string`, no streaming) | Phase 4 (streaming + follow) |
| 3 | Broken copy off macOS/X11 | Phase 2 (OSC 52 + full fallback chain) |
| 4 | No GitHub callouts | Phase 1 (callout panels) |

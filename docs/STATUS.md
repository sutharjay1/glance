# glance — status log

Newest first. One entry per working session. Template at the bottom.
Weekly summaries can be generated with the `operations:status-report` skill.

---

## 2026-07-22 — Phase 1: layout::DocLayout (indexed layout) · JAY-91
**Phase:** 1 (🟨) · **Focus:** markdown pipeline (layout complete) · **Branch:** `feature/jay-91-phase-1-viewer-core`

- Shipped `layout_document` → `DocLayout { lines, text, headings, code_blocks, links }`: top-level headings + code blocks indexed with line positions, links collected from all lines' spans (merging same-href runs). This is what the event loop consults for `[`/`]`, `/` search, `o` TOC, `f` link picker, and click hit-testing.
- **Design call:** deferred the `(block,width)` cache + viewport-first background layout — a full-doc layout is cheap and scrolling just slices `lines[top..]`, so no cache is needed to scroll smoothly. Will add only if a perf gate on a huge doc demands it. Tables still deferred.
- **84 tests green**, fmt + clippy clean (commit a02c660). `layout` module now functionally complete.

**Next:** `view::render` — frame builder (DocLayout + viewport → painted visible lines) + damage diff (rewrite only changed rows) + synchronized-output writer. Pure/testable core before the event loop wiring.

**Blockers:** none.

---

## 2026-07-22 — Phase 1: md::highlight (instant tokenizer) · JAY-91
**Phase:** 1 (🟨) · **Focus:** markdown pipeline · **Branch:** `feature/jay-91-phase-1-viewer-core`

- Shipped `md::highlight`: a tiny language-agnostic lexer (keyword/string/comment/number/function/plain) with per-language specs for js/ts, python, rust, go, bash, sql (+aliases). Line + multi-line block comments, escape-aware strings, case-insensitive SQL keywords. **No grammar loading** — safe on the first-paint path (syntect patches over it in Phase 3).
- Added code-token roles (`style`) + colors (`theme` dark/light) + `paint` mapping; wired into `layout` so recognized-lang code blocks render colored (unknown langs stay plain).
- Bug caught by the full-chain integration test: `pick_color` resolved `style.code` before the role, so all code tokens went generic-green — fixed to resolve token roles first. (Also: amended the commit after catching a failing test I'd committed — process note.)
- **80 tests green**, fmt + clippy clean (commit 192f10f).

**Next:** finish `layout` — `DocLayout` (lines + plain text + heading/code/link indices for search, nav, hit-testing) + `(block,width)` cache. Pure, testable; unblocks the event loop.

**Blockers:** none.

---

## 2026-07-22 — Phase 1: paint + theme → 🎉 end-to-end render · JAY-91
**Phase:** 1 (🟨) · **Focus:** render · **Branch:** `feature/jay-91-phase-1-viewer-core`

- Shipped `paint` (`Line → ANSI`) + `theme` (dark/light, brand accent `#FF5800`). Link runs → single OSC 8 (internal link spaces carry href); dim ` (url)` fallback when hyperlinks off; `no_wrap` suppresses both; `ColorDepth::None` = clean plain text. Extended `osc` with `link_open`/`LINK_CLOSE`.
- **Wired a provisional pipe render into `run()`** — `glance file.md` now parses → lays out → paints to stdout (colored on a TTY, plain when piped/`--no-color`). First visible end-to-end output.
- Running it caught two real bugs unit tests missed: (1) multi-word links split into multiple OSC 8 runs → internal link spaces now carry the href; (2) word-splitting fabricated spaces before punctuation (`code ,`) → tokenizer rewritten to preserve whitespace boundaries with explicit `Space` tokens. Regression tests added.
- **69 tests green**, fmt + clippy clean. Commits 4baaf19 (layout+style), 5a42628 (paint+theme+wiring).

**Next:** `md::highlight` — regex micro-tokenizer (js/ts, py, rust, go, bash, sql) for instant code coloring; pure and testable.

**Blockers:** none.

---

## 2026-07-22 — Phase 1: md::layout (wrap engine) + style model · JAY-91
**Phase:** 1 (🟨) · **Focus:** markdown pipeline · **Branch:** `feature/jay-91-phase-1-viewer-core`

- Added `style` module — the `Line`/`Span`/`Style`/`Role` model (semantic styles + link href), the handoff between layout and paint.
- Shipped `md::layout` core: `Block × width → Vec<Line>`. Word-wrap uses the `text::width` ASCII fast path + incremental accumulation (plan §8). Hanging indents via first/cont prefixes (markers never repeat); blockquote bars + callout panels (icon+NAME header, barred body); ordered/task list markers; code as `no_wrap` lines; thematic breaks fill width.
- TDD: 12 tests incl. a **wrap-invariant property test** (`assert_within`: no line exceeds width) run at widths 10/20/40/80 and the real fixture at 44/80/120, plus a text round-trip. **62 total green**, fmt + clippy clean.
- Deferred to next layout iteration: `(block,width)` cache, viewport-first slicing, DocLayout indices (headings/codeBlocks/links for search/nav/hit-testing), tables.

**Next:** `paint` — `Line → ANSI` (theme + `term::ansi` downsampling + OSC 8 link runs). Unlocks a visible end-to-end pipe-mode render.

**Blockers:** none.

---

## 2026-07-22 — Phase 1: md::parse (Block tree) · JAY-91
**Phase:** 1 (🟨) · **Focus:** markdown pipeline · **Branch:** `feature/jay-91-phase-1-viewer-core`

- Shipped `md::parse`: pulldown-cmark GFM → typed `Block`/`Inline` tree via a stack-based event folder. Includes ESC/C0 **sanitize** at the boundary (weakness #4 hardening), GitHub **callout** detection, task-list markers, ordered-list start, fenced-code lang.
- TDD paid off — two real pulldown behaviors caught by failing tests: (1) tight-list items emit bare text with no `Paragraph` → added implicit-paragraph handling; (2) `[!NOTE]` tokenizes into 3 Text events (`[`,`!NOTE`,`]`) → marker reconstruction across the leading Text run.
- Added `pulldown-cmark` 0.12. 12 tests incl. a real mdterm-fixture smoke test. **50 total green**, fmt + clippy clean. Tables parsed-but-skipped (dedicated follow-up with layout).

**Next:** `md::layout` — Block × width → `Vec<Line>` with hanging indents, callout panels, layout cache `(block,width)`, viewport-first. The core of the perf architecture (ADR 0004).

**Blockers:** none.

---

## 2026-07-22 — Phase 1: term::input (crossterm mapping) · JAY-91
**Phase:** 1 (🟨) · **Focus:** terminal layer (done, testable parts) · **Branch:** `feature/jay-91-phase-1-viewer-core`

- Shipped `term::input`: app-level `Key`/`Mouse`/`Event` model + pure `map_event(crossterm::Event) → Option<Event>`. crossterm owns decoding (ADR 0003); we normalize to a small stable vocabulary. Key releases ignored (avoids Windows double-processing); Ctrl-letters lowercased; scroll/left-click mapped with coords for hit-testing.
- Added `crossterm` 0.28 dep. TDD: 8 new tests (char, ctrl, nav keys, release-ignored, scroll/click, right-click-ignored, resize, paste-ignored). **37 total green**, fmt + clippy clean.
- Terminal layer's *testable* surface complete. `TerminalGuard` (RAII raw-mode restore) + raw-mode setup are I/O wiring → built with the event loop and PTY-tested, not unit-tested in isolation.

**Next:** pivot to the markdown pipeline — `md::parse` (pulldown-cmark GFM → Block tree; ESC/C0 sanitize; callout + task detection). High TDD value.

**Blockers:** none.

---

## 2026-07-22 — Phase 1: term::osc (OSC 8/52/11) · JAY-91
**Phase:** 1 (🟨) · **Focus:** terminal layer · **Branch:** `feature/jay-91-phase-1-viewer-core`

- Shipped `term::osc`: OSC 8 hyperlinks, OSC 52 clipboard (base64 via new `base64` dep, with `clipboard_within` cap so large copies fall back to native — fixes weakness #3), OSC 11 background query + `parse_bg_response` + `is_dark` (feeds auto dark/light theme, §4.5).
- Nice tie-in: `parse_bg_response` test decodes Catppuccin `#1E1E2E` → `is_dark` = true, matching the Phase 0 theme finding.
- TDD: 7 new tests (framing, base64, cap, 4-/2-digit channel parse, malformed rejection, luminance). **29 total green**, fmt + clippy clean.

**Next:** `term::input` — crossterm `Event` → app `Event`/`Key` mapping (testable pure mapping fn; adds the `crossterm` dep). Then pivot to `md::parse`.

**Blockers:** none.

---

## 2026-07-22 — Phase 1: term::ansi (color downsampling) · JAY-91
**Phase:** 1 (🟨) · **Focus:** terminal layer · **Branch:** `feature/jay-91-phase-1-viewer-core`

- Shipped `term::ansi`: `Rgb` + SGR builders (`fg`/`bg`/`sgr`/`RESET`) with **truecolor→256→16 downsampling**. 256-cube uses the canonical tmux algorithm (6×6×6 cube vs gray-ramp, pick nearest); 16-color is nearest-of-standard-palette. This is the mechanism behind ADR 0004's "author once in truecolor, render everywhere" — consumes the `ColorDepth` from `caps`.
- TDD: 7 new tests (cube corners, exact-cube-not-grayed, mid-gray→ramp, primaries, fg/bg per depth, sgr wrapping). **22 total green**, fmt + clippy clean.

**Next:** `term::osc` — OSC 8 hyperlinks, OSC 52 clipboard (+ base64 + chunking), OSC 11 background query (pure string building; will add the tiny `base64` dep).

**Blockers:** none.

---

## 2026-07-22 — Phase 1 start: term::caps · JAY-91
**Phase:** 1 (🟨) · **Focus:** terminal layer · **Branch:** `feature/jay-91-phase-1-viewer-core`

- Scaffolded `src/term/` and shipped `term::caps` — color-depth detection (`None/Ansi16/Ansi256/TrueColor`) from `NO_COLOR`/`COLORTERM`/`TERM`, as a pure `detect_color_depth()` + a thin `Capabilities::from_env()`. Foundation for the theme downsampling in ADR 0004.
- TDD: 8 new tests (precedence, case-insensitivity, 256/truecolor variants, dumb/absent). **15 tests total green**, fmt + clippy clean.

**Next:** `term::ansi` — semantic `Style → SGR` + truecolor→256→16 downsampling (consumes `ColorDepth`).

**Blockers:** none.

---

## 2026-07-22 — Phase 0 complete (benchmark + vendor survey) · JAY-89
**Phase:** 0 → done · **Focus:** de-risk before coding · **Branch:** `feature/jay-89-phase-0-benchmark-reference-vendor-survey`

- Built mdterm reference (release, **9.7 MB**, ~20 s). Toolchain: rustc/cargo 1.97.1.
- **Benchmarks** ([`benchmarks.md`](benchmarks.md)): mdterm export ~60 ms regardless of doc size (7.9 KB vs 384 KB → 60.2 vs 61.8 ms). The ~60 ms floor is **fixed eager syntax/theme load**, not doc work — quantifies weakness #1 and validates the lazy-highlight architecture. glance target: well under 60 ms first paint.
- **Parity mine** of `viewer.rs` ([`parity-notes-from-source.md`](parity-notes-from-source.md)): found missing `m` mouse-toggle, JSON-viewer keys (`L/H/D`), checkbox-toggle-writes-file, OSC 22 hover cursor; confirmed our accent `#FF5800` is an intentional divergence (mdterm = Catppuccin-Mocha `#89B4FA`); confirmed `--pipe`/`-p`/OSC 52 are genuinely our differentiators. Folded into `parity-checklist.md`.
- **Vendor survey** ([`vendor-survey.md`](vendor-survey.md) + [`vendor/NOTICE`](../vendor/NOTICE)): image = Low coupling (portable, P3); json+diagram = High/Medium and **must vendor together**; math = verbatim. mdterm is MIT © 2026 Gokul.
- Fixtures: `tests/fixtures/mdterm-test.md`, `mdterm-test-linked.md`, generated `big-5k.md`.

**Next:** Phase 0.5 — scaffold the cargo project (deps, release profile, CI) then begin Phase 1 (term layer, TDD). Blocker cleared: `hyperfine` installed via brew.

**Blockers:** none.

---

## 2026-07-22 — Project kickoff & tracking setup
**Phase:** pre-Phase-0 · **Focus:** planning + project scaffolding

- Approved the full 6-phase build plan (`~/.claude/plans/glance-build-mellow-pascal.md`).
- Stack decided: **Rust**, clean-room + vendor mdterm's strong modules (ADR 0001–0002).
- Set up tracking: repo docs (`ROADMAP.md`, `docs/adr/`, `docs/parity-checklist.md`, this log) + Linear project `glance` (team Jay) + graphify.
- `git init` done. No code yet.

**Next:** Phase 0 — build the reference, benchmark, vendor survey; then scaffold the cargo project.

**Blockers:** confirm `hyperfine` / `vhs` availability for Phase 0 benchmarks + Phase 5 GIFs.

---

### Entry template
```
## YYYY-MM-DD — <title>
**Phase:** <n> · **Focus:** <area>
- <what changed / decided / learned>
**Next:** <the one or two next actions>
**Blockers:** <anything stuck, or "none">
```

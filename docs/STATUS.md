# glance — status log

Newest first. One entry per working session. Template at the bottom.
Weekly summaries can be generated with the `operations:status-report` skill.

---

## 2026-07-22 — Phase 2: OSC 11 auto-theme · JAY-92
**Phase:** 2 (🟨) · **Focus:** interactivity · **Branch:** `feature/jay-91-phase-1-viewer-core`

- The pure half (OSC 11 reply parser `parse_bg_response` + `is_dark` luminance) already shipped in Phase 1 and is unit-tested (4/8/16-bit hex, BEL vs ST terminators, malformed rejection). This iteration adds the **thin I/O**: `term::osc::detect_dark_background()` — before the alt-screen, writes `ESC]11;?BEL` **plus a DSR `ESC[6n` fence** and reads stdin until the guaranteed `R` terminator. The fence is the key trick: every VT terminal answers DSR in query order, so the read always terminates even when OSC 11 is ignored — **no reader thread, no timeout, no keystroke theft**. Gated on both std streams being TTYs; `#[cfg(not(unix))]` stub returns `None` (best-effort, default theme).
- **Explicit choice wins**: `config::has_theme_key()` (pure, tested) + `theme_is_configured()` detect an explicit `theme` in config; `lib` sets `theme_explicit = parsed.theme.is_some() || config::theme_is_configured()` and only auto-detects when it's false — `-T`/`--theme` and a configured theme both override detection. Falls back to the explicit/default value when the terminal doesn't answer.
- **166 total green**, fmt + clippy clean. Interactive query path is I/O — PTY-tested later.

**Next:** the last Phase 2 item — click-to-copy (mouse-click a code block to copy via the existing copy stack + toast; map click Y → doc line → nearest code block using `DocLayout` indices). Then Phase 2 is COMPLETE → Phase 3 (syntect lazy highlight + image ladder).

---

## 2026-07-22 — Phase 2: auto-reload (notify + debounce) · JAY-92
**Phase:** 2 (🟨) · **Focus:** interactivity · **Branch:** `feature/jay-91-phase-1-viewer-core`

- New `view::watch` module, split so the tricky part is testable: **`Debouncer`** (pure, clock-injected) coalesces a save burst per path — a path is *ready* only after it's been quiet ≥120 ms (`saturating_duration_since`), so editor write→rename→truncate bursts yield one reload, not several mid-write flashes (3 unit tests: quiet-window, timer-reset, independent paths). **`FileWatcher`** is thin `notify` wiring — watches each open file's **parent directory** (editors replace files by rename, which breaks a watch on the inode) and forwards only events whose canonical path is one we hold. `notify::recommended_watcher` is immediate-mode, so our debouncer owns the policy.
- `ViewerState::reload()` re-reads its own file **in place** preserving scroll + active search (re-parse → re-layout → clamp `top` → re-run stored `Search`); an **empty read is ignored** (mid-write truncation guard). `canonical_path()` matches watch events to tabs. `Tabs::paths()`/`reload_path()` reload every tab on a changed path and report whether the **active** one changed (only that needs a repaint; background tabs refresh silently).
- Event loop now has two wake sources: with files open it `event::poll(50 ms)` so reloads fire while idle; with no file (piped stdin) it keeps the original blocking `event::read()` — zero idle CPU. A reload sets a `reloaded` toast + full repaint.
- Added `notify = "6"`. 1 reload test (temp file, scroll+search preserved, empty-read ignored) + 3 debouncer tests. **165 total green**, fmt + clippy clean. Interactive watch path is I/O — PTY-tested later, like the rest of the loop.

**Next:** Phase 2 remainders — OSC 11 auto-theme (query terminal bg → pick dark/light unless `-T` overrides), click-to-copy (mouse-click a code block/section to copy). Then Phase 2 closes.

---

## 2026-07-22 — Phase 2: multi-file tabs (Tab/Shift+Tab) · JAY-92
**Phase:** 2 (🟨) · **Focus:** interactivity · **Branch:** `feature/jay-91-phase-1-viewer-core`

- New `view::tabs::Tabs` — a thin holder over `Vec<ViewerState>` + active index: `next`/`prev` (wrapping, no-op for a single tab), `active`/`active_mut`, `resize_all`, and a `label()` → `[2/3 name.md]` (None for one tab). Kept separate so cycling is unit-testable without a terminal (4 tests: single-tab no-cycle, wrap forward/back, label, independent per-tab scroll).
- `app::run` now takes `Vec<(Vec<Block>, Option<PathBuf>)>` and builds `Tabs` (one `ViewerState` per file → per-file scroll/search/line-number state preserved for free). `Tab`/`BackTab` are intercepted in a **guard arm before** `let state = tabs.active_mut()` — sidesteps the borrow conflict of switching tabs while the active tab is mutably borrowed. `draw` takes `&Tabs`; `status_line` shows the tab label below toast/search (`.or(tab_label)`).
- `lib.rs`: reads **all** `parsed.files` (first is already in `input`), skipping unreadable ones with a `… (skipped)` warning, and passes the docs vec to `app::run`.
- **161 total green**, fmt + clippy clean (commit c341f3d). Reinstalled.

**Next:** auto-reload (`notify` watcher + ~120 ms debounce; reload preserving scroll + active search; tolerate mid-write partial reads). Then the Phase 2 remainders: OSC 11 auto-theme, click-to-copy.

---

## 2026-07-22 — Phase 2: code line numbers (l) · JAY-92
**Phase:** 2 (🟨) · **Focus:** interactivity · **Branch:** `feature/jay-91-phase-1-viewer-core`

- `md::layout`: threaded `line_numbers` through `layout_document`/`layout_blocks`/`layout_block` (+ list/callout/blockquote recursion). New `layout_code` renders a right-aligned `N │ ` gutter (dim), reserving its width so code truncates to fit. `gutter_width` = digits + `" │ "`.
- `ViewerState`: `line_numbers` field + `toggle_line_numbers` (re-layout, anchor scroll); `l` in `on_key`. `new()` takes the flag; wired from `cli -l` / `config.line_numbers`. `render_document`+`app::run` take it; lib threads it.
- Churn note: threaded the flag through ~15 call sites (bulk-fixed test callers with sed). 1 test (gutter on/off). **157 total green**, fmt + clippy clean (commit 5bb4c08). Reinstalled.

**Next:** multi-file tabs (`Tab`/`Shift+Tab`, per-file scroll) — the last big Phase 2 item; then auto-reload, OSC 11 auto-theme, click-to-copy close it out.

**Blockers:** none.

---

## 2026-07-22 — Phase 2: help overlay (h/?) + theme toggle (t) · JAY-92
**Phase:** 2 (🟨) · **Focus:** interactivity · **Branch:** `feature/jay-91-phase-1-viewer-core`

- `overlays::help_lines()`: static, sectioned keybinding cheat-sheet. `app` `Mode::Help` — `h`/`?`/`F1` opens, any key closes; full-screen render.
- **Runtime theme toggle**: `run()` holds the theme mutably (built from a `theme_dark` bool); `t` cycles dark↔light — **repaint-only, no relayout** (proving the paint/layout split from ADR 0004) — with a toast. lib passes `theme_dark`.
- 1 test (help content). **156 total green**, fmt + clippy clean (commit 85b954f). Reinstalled.

**Next:** line numbers (`l`) — thread a `line_numbers` flag through `layout_document`→code rendering (gutter prefix reserving width); wire `-l`/config initial value. Then multi-file tabs (`Tab`), auto-reload, OSC 11 auto-theme.

**Blockers:** none.

---

## 2026-07-22 — Phase 2: copy stack (fixes weakness #3) · JAY-92
**Phase:** 2 (🟨) · **Focus:** interactivity · **Branch:** `feature/jay-91-phase-1-viewer-core`

- `view::copy`: `copy()` tries **OSC 52 first** (`osc::clipboard_within`, ~100 KB cap) → SSH/tmux-safe, no external tool; else native per platform (macOS `pbcopy`; Linux `wl-copy`→`xclip`→`xsel`; Windows `clip`) via **error-handled** `Command` pipes — a missing binary degrades, never crashes. Returns method + the OSC 52 sequence for the caller to emit. `toast()` text helper.
- `ViewerState`: `toast` field + `document_text` (`Y`), `nearest_code_block` by proximity to viewport top (`c`), `file_path_string` absolute (`p`).
- `app`: `c`/`Y`/`p` in Normal — `copy_to` writes the OSC 52 seq to the terminal + sets a toast; `p` with no path → `(stdin — no path)`. Any keypress dismisses the toast; a toast takes the status row.
- 7 tests (OSC 52 path, oversized-skips-OSC52, toast text, code-block proximity, path/doc). **155 total green**, fmt + clippy clean (commit a02d124). Reinstalled. **Reference weakness #3 fixed.**

**Next:** help overlay (`h`/`?`) listing keybindings; then theme toggle (`t`) + OSC 11 auto-detect, line numbers (`l`), tabs, auto-reload.

**Blockers:** none.

---

## 2026-07-22 — Phase 2: link picker (f) + local-file nav · JAY-92
**Phase:** 2 (🟨) · **Focus:** interactivity · **Branch:** `feature/jay-91-phase-1-viewer-core`

- `open` module: `classify(url, base)` → `Url` / `LocalFile(resolved-against-current-dir)` / `Other`; `is_markdown`; `open_command` (macOS `open` / Linux `xdg-open` / Windows `start`) + `open_url` (spawns stdio-nulled, **errors swallowed** — a missing opener degrades, never crashes; plan §4.1).
- `overlays::Links`: numbered link picker over `DocLayout.links`.
- `ViewerState` file nav: `path` + `history` stack + `load(path)` (re-parse/layout, push history) + `back()` (Backspace restores prior file+scroll); `current_dir` for relative link resolution. `new()` now takes `Option<PathBuf>`.
- `app`: `Mode::Links` — `f` opens; digits open directly; arrows+Enter open the selection; web/other→opener, local `.md`→load in-app, other local→opener. `Backspace`→back.
- 7 tests (classify/markdown/opener + picker). **149 total green**, fmt + clippy clean (commit 1882aba). Reinstalled.

**Next:** the copy stack — OSC 52 first + native fallbacks (pbcopy/wl-copy/xclip/xsel/PowerShell), `c` (nearest code block), `Y` (whole doc), `p` (file path), click-to-copy, toasts. Fixes reference weakness #3.

**Blockers:** none.

---

## 2026-07-22 — Phase 2: fuzzy heading filter (:) · JAY-92
**Phase:** 2 (🟨) · **Focus:** interactivity · **Branch:** `feature/jay-91-phase-1-viewer-core`

- `fuzzy::score`: case-insensitive subsequence match with bonuses (word-start, contiguous run, leading position); `None` on no match — good enough for heading lists.
- `overlays::Fuzzy`: filtered + ranked heading picker; `push`/`pop` edit the query and refilter (best score first, ties keep doc order), arrows select, `selected_line` jumps. Factored shared `heading_lines`/`window` helpers (used by both Toc + Fuzzy).
- `app`: `Mode::Fuzzy` — `:` opens, typed chars filter live, ↑/↓ select, Enter `center_on`s + closes, Esc closes; overlay render + `:query n/m` status.
- 9 tests (scorer + filter/rank/empty/pop). **143 total green**, fmt + clippy clean (commit 2b72b72). Reinstalled.

**Next:** link picker (`f`) — list `DocLayout.links`, number/arrow select, Enter opens URL (`open`/`xdg-open`) or follows a local `.md` file; then local-file nav + `Backspace` history.

**Blockers:** none.

---

## 2026-07-22 — Phase 2: TOC overlay (o) · JAY-92
**Phase:** 2 (🟨) · **Focus:** interactivity · **Branch:** `feature/jay-91-phase-1-viewer-core`

- Shipped `view::overlays::Toc`: pure heading picker over `DocLayout.headings` — depth-indented lines, `j`/`k` clamped selection, `selected_line` for jump, `view(w,h)` windows long lists keeping the selection visible, selected row full-width reverse-highlighted.
- `view::app`: `Mode::Toc` overlay — `o` opens (when headings exist), `j`/`k`/arrows move, Enter `center_on`s the heading + closes, `Esc`/`o`/`q` close; full-screen render + status line (`TOC n/m · j/k · Enter · Esc`).
- 6 tests (order, clamp, selected-line, empty, indent+highlight, windowing). **134 total green**, fmt + clippy clean (commit 58d367b). Reinstalled.

**Next:** fuzzy heading filter (`:`) — the TOC with a live text filter over headings (small fuzzy scorer). Then link picker (`f`), copy stack.

**Blockers:** none.

---

## 2026-07-22 — Phase 2: search UI (interactive) · JAY-92
**Phase:** 2 (🟨) · **Focus:** interactivity · **Branch:** `feature/jay-91-phase-1-viewer-core`

- Search is now **fully interactive**: `style.highlight` (reverse-video, depth-independent) + `paint` SGR 7; `render::highlight_line` splits painted spans at match byte-offsets; `build_frame` takes an optional `Search` and highlights matches on visible rows.
- `view::app`: `Mode::Search` input mode — `/` opens the prompt, typed chars build the query (Backspace edits), Enter runs the search (jump + highlight), Esc cancels; bottom status line shows the live prompt and a `query n/m` readout.
- 5 new tests incl. a deterministic build_frame→highlight→paint integration check (no PTY). **128 total green**, fmt + clippy clean (commit 5d1fc84). Reinstalled to `~/.cargo/bin/glance`.

**Next:** TOC overlay (`o`) — list headings from `DocLayout.headings`, arrow-select, Enter to jump. Then fuzzy filter (`:`), link picker (`f`), copy stack.

**Blockers:** none.

---

## 2026-07-22 — Phase 2: search core · JAY-92
**Phase:** 2 (🟨) · **Focus:** interactivity · **Branch:** `feature/jay-91-phase-1-viewer-core`

- Shipped `view::search`: pure `Search` over `DocLayout.text` — regex with literal fallback (invalid regex → escaped literal), all matches with (line, byte-range), `next`/`prev` wrapping cycle, `position()` for a `3/12` readout, `on_line()` for highlighting.
- `ViewerState` integration: `run_search` (jump + `center_on` first match), `search_next`/`search_prev` (cycle + recenter), `clear_search`; `n`/`N`/`Esc` wired in `on_key` (gated on active search); resize re-runs the search against the new layout.
- Added `regex` dep. TDD: 7 search + 5 state tests (regex, invalid-regex-fallback, cycle-wrap, empty, key handling). **124 total green**, fmt + clippy clean.

**Next:** wire the `/` prompt UI (input mode in `view::app`) + highlight matches in `view::render` (using `Search::on_line`) — makes search interactive.

**Blockers:** none.

---

## 2026-07-22 — ✅ Phase 1 COMPLETE (viewer core) · JAY-91
**Phase:** 1 → **done** · **Branch:** `feature/jay-91-phase-1-viewer-core` (renamed from jay-89)

- Shipped insta snapshot goldens (`tests/render_snapshots.rs` + 4 `.snap`) of the full pipeline over a curated fixture at widths 44/80/120 (plain) + one colored.
- The snapshot **immediately caught a real bug**: the ` (url)` suffix was appended by paint *after* wrapping → line overflow in pipe/no-OSC8 output. Fixed by baking suffixes into the block tree *before* layout (`parse::with_url_suffixes`); paint now only decides OSC 8. Width-44 invariant verified.
- **Phase 1 exit criteria all met:** first-paint **0.92 ms** (< 80 ms), smooth scroll, clean pipe, movement keys (`j/k/Space/b/d/u/g/G/[/]`/wheel), snapshots green at 44/80/120.
- **Perf validated head-to-head:** glance 1.7 ms vs mdterm 59.1 ms (35×), 831 KB vs 9 MB (11×).
- **113 tests + 4 goldens green**, fmt + clippy clean (commit ab51789). Installed to `~/.cargo/bin/glance`.

**Phase 1 modules (all ✅):** term (caps/ansi/osc/input) · md (parse/layout+DocLayout/highlight) · style · paint · theme · view (render/state/app+TerminalGuard) · cli · config.

**Next:** Phase 2 (JAY-92) — search (`/`), TOC (`o`), fuzzy (`:`), link picker (`f`) + local-file nav, copy stack (OSC 52 + fallbacks + `p` + click), help, theme toggle + OSC 11, line numbers, tabs, auto-reload. Continuing straight through (user override: don't stop at phase boundaries).

**Blockers:** none.

---

## 2026-07-22 — Phase 1: cli (lexopt) + config (toml) · JAY-91
**Phase:** 1 (🟨) · **Focus:** modes/cli · **Branch:** `feature/jay-91-phase-1-viewer-core`

- `cli::parse` (lexopt): `Args` with files + `-T/-w/-s/-l/-f/--export/--no-color/--pipe/-h/-V`; short/long/equals forms; bad value or unknown flag → error (exit 2).
- `config`: `~/.config/glance/config.toml` (`theme`, `line_numbers`, `width` — reference keys for migration) via serde+toml; missing/malformed → defaults; XDG_CONFIG_HOME or HOME path.
- `run()` now layers **CLI → config → defaults**; `-w` threads into both the TUI (`width_override`) and pipe render. Verified: `-w 40` wraps at 40, config loads via `XDG_CONFIG_HOME`, `-V` prints, `--frob` → exit 2.
- **111 tests green**, fmt + clippy clean (commit ac5b360).

**Next (Phase 1 exit):** insta snapshot tests at widths 44/80/120 (colored + `--no-color`) + a `--timing` first-paint measurement (< 80 ms gate). Then Phase 1 is complete.

**Blockers:** none.

---

## 2026-07-22 — Phase 1: view::state + view::app → 🎉 interactive TUI · JAY-91
**Phase:** 1 (🟨) · **Focus:** the interactive viewer · **Branch:** `feature/jay-91-phase-1-viewer-core`

- `view::state`: `ViewerState` + pure navigation (scroll/page/half-page, `g`/`G`, `[`/`]` heading jumps via DocLayout index, wheel, resize-relayout with scroll anchor) → `Action` (Redraw/Quit/Ignore). 9 tests, no terminal I/O (commit ec15174).
- `view::app`: **`TerminalGuard`** RAII (enter alt-screen + raw + mouse + hide-cursor; Drop restores) + **panic hook** doing the same restore (release is `panic=abort`, so Drop won't run on panic — a crash must never break the terminal, plan §8). Event loop: crossterm event → `map_event` → state → `render` damage diff; resize forces full repaint. Wired into `run()`: TTY+file → TUI, else pipe (commit e908c77).
- **PTY smoke verified**: `printf 'jjjGq' | script … glance demo.md` → exit 0, and the capture contains alt-screen enter+leave, cursor hide+show, mouse capture, and synchronized-output sequences — clean setup and teardown.
- **99 tests green**, fmt + clippy clean. **glance is now a working interactive markdown viewer.**

**Next (Phase 1 exit):** `cli` (lexopt: `-w/-T/-l/--no-color/--pipe`) + `config` (`~/.config/glance/config.toml`); then insta snapshots at 44/80/120 and a `--timing` first-paint gate.

**Blockers:** none.

---

## 2026-07-22 — Phase 1: view::render (frame + damage diff) · JAY-91
**Phase:** 1 (🟨) · **Focus:** render · **Branch:** `feature/jay-91-phase-1-viewer-core`

- Shipped `view::render`: `build_frame` (slice DocLayout viewport → painted rows, blank past EOF) + `render(prev, next)` damage diff (rewrite only changed rows: cursor move + clear-to-EOL each, wrapped in synchronized output CSI ?2026). `render(None, next)` = the `--full-repaint` fallback. `max_top` scroll clamp.
- Added screen/cursor ops to `term::ansi` (sync begin/end, alt-screen, cursor show/hide, `move_to`, clear) — also feeds the upcoming TerminalGuard.
- TDD: 6 tests incl. **damage-diff-writes-only-changed-rows** and **identical-frames-write-nothing-but-sync** (proves a no-op frame emits zero row writes — the point of damage rendering). **90 total green**, fmt + clippy clean (commit 77ae41f).

**Next:** `view::state` — `ViewerState` + pure navigation (scroll/page/half-page/g/G, heading jumps via DocLayout indices). Testable state transitions before the terminal event loop (`app` + TerminalGuard, PTY-tested).

**Blockers:** none.

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

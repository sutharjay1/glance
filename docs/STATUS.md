# glance — status log

Newest first. One entry per working session. Template at the bottom.
Weekly summaries can be generated with the `operations:status-report` skill.

---

## 2026-07-22 — UX: persistent status/hint bar (discoverable exit) · post-launch
**Focus:** UX polish · **Branch:** `main`

- **Problem:** nothing on screen told the user how to quit or what the keys do. Added a **persistent bottom status/hint bar** (vim/less-style).
- The bottom row is now **reserved** for the bar — `run`/resize set the content viewport to `rows − 1` (`ViewerState.height`), so the bar never covers content (previously the status was overlaid on the last content row). `draw` appends the bar as an extra painted row → total = `height + 1` = terminal rows.
- `bar_text` returns the bar content: a **key legend** in Normal mode (`q quit · / search · o toc · f links · c copy · Y all · t theme · Tab files · h help`), swapped for a contextual prompt while an overlay/search/toast/stream-pill is active (each still surfaces `h help` or `Esc close`). Multi-tab label folds into the legend. `status_bar` paints it **reverse-video** (depth-independent) padded to full width. Replaces the old `status_line` overlay.
- 3 unit tests (legend has `q quit`/`h help`, tab label folds in, bar pads to exact width). **228 total green**, fmt + clippy clean, reinstalled.

---

## 2026-07-22 — Phase 5: mermaid port → **ALL PORTS DONE, BUILD FEATURE-COMPLETE** · JAY-95
**Phase:** 5 (🟨 — ports done, launch remains) · **Focus:** ports + launch · **Branch:** `feature/jay-91-phase-1-viewer-core`

- New `src/mermaid.rs` — fenced ```` ```mermaid ```` flowcharts → Unicode box-art. Pure `parse` handles the simple subset (`graph`/`flowchart` `TD`/`LR`; nodes `A[Label]`/`A(Label)`/`A{Label}`/bare; edges `A --> B`, `A -->|label| B`, `A --- B`, chains `A --> B --> C`), interning nodes in first-seen order + collecting directed edges. `render` draws each node as a bordered box (`┌─┐│└─┘`) stacked vertically with `│`/`▼` connectors for consecutive edges; branch/back edges listed below as `from ──▶ to`. **Anything unrecognized (sequenceDiagram, subgraphs, styling) → raw-source fallback** — never crashes or garbles. Wired in `layout_block`: a `lang=="mermaid"` code block renders via `mermaid::render` instead of the syntax highlighter. 6 tests (nodes/edges, chains+labels, non-flowchart→None, box-art contains labels+border+arrow, unknown→raw, branching→listed). Verified e2e (Start→Middle→End boxes with a `▼ done` labeled arrow).
- **225 total green**, fmt + clippy clean, reinstalled. Binary **4.34 MB**.

## **BUILD FEATURE-COMPLETE** 🎉
All 5 phases shipped on `feature/jay-91-phase-1-viewer-core`: Phase 1 viewer core · Phase 2 interactivity · Phase 3 syntect + images · Phase 4 streaming/slides/export · Phase 5 ports (math/json/mermaid). **225 tests, 4.34 MB (2.1× < mdterm's 9 MB), first-paint <1 ms.** The 4 mdterm weaknesses are all fixed (lazy highlight, streaming stdin, universal OSC-52 copy, GFM callouts).

**Remaining = LAUNCH (needs the user, outward-facing):** README with parity + benchmark tables (data already in `docs/benchmarks.md`); vhs GIFs of the `llm | glance` demo; `cargo publish` → crates.io; Homebrew tap formula; `cargo-dist` prebuilt binaries + GitHub release; Show HN + X thread. And: PR/merge the branch to `main`. Handed to the user for their accounts + timing.

---

## 2026-07-22 — Phase 5: JSON viewer port · JAY-95
**Phase:** 5 (🟨) · **Focus:** ports + launch · **Branch:** `feature/jay-91-phase-1-viewer-core`

- **Size check first:** `serde_json` spike → **+32 KB** (shares serde's infra, already a dep) — far under the 0.5 MB flag. Approved, spike reverted, dep kept.
- New `src/json.rs` — pure `json_to_lines(&Value, width) -> Vec<Line>`: pretty-print with 2-space indent per depth and semantic roles (keys→Heading, string values→Str, numbers→Number, bool/null→Keyword, braces/colons/commas→Dim). Empty containers inline (`{}`/`[]`); long lines truncated to width with `…`. `render(raw, width)` parses and falls back to raw + a `⚠ invalid JSON` note on error. 6 tests (typed roles, depth indent, array commas, empty containers, error fallback, truncation).
- **Wiring via a new `Block::Prerendered(Vec<Line>)`** passthrough variant (layout returns the lines verbatim) — so the JSON viewer reuses the *entire* pipeline (scroll, search, resize) with zero viewer changes. `lib.rs` feature-detects `.json` (case-insensitive): interactive → a one-block `Prerendered` doc through `app::run`; pipe → paint each line to stdout. Invalid JSON still views (raw + note).
- **219 total green** (+6), fmt + clippy clean, reinstalled. Binary 4.33 MB.

**Next:** mermaid port (simple `mermaid` fenced flowcharts → box-art; complex → raw fallback). **Then STOP** — remaining Phase 5 is launch (crates.io/Homebrew/cargo-dist/GIFs/Show HN), outward-facing, needs the user.

---

## 2026-07-22 — Phase 5: math port ($…$ → Unicode) · JAY-95
**Phase:** 5 (🟨) · **Focus:** ports + launch · **Branch:** `feature/jay-91-phase-1-viewer-core`

- New `src/md/math.rs` — inline `$…$` LaTeX → Unicode. `math_to_unicode`: a ~120-entry symbol table (lower/upper Greek, big operators ∑∏∫, relations ≤≥≠≈, arrows →⇒↔, binary ops ×·±, set/logic ∈∀∃∪∩, misc ∞√∂∇…) + super/subscript conversion (`^2`→², `_1`→₁ via the Unicode blocks; groups converted recursively then scripted, all-or-nothing per group; unknown `\cmd` → bare name).
- **The real subtlety** — markdown emphasis vs. math: pulldown parses `$x_i$`'s `_` as emphasis, which would split the math span across inlines. Fix: `preprocess_math` runs on the **raw source before pulldown** (wired in `parse::parse`), so `$\sum_{i=1}^{n}$` becomes `∑ᵢ₌₁ⁿ` (no underscores) before the parser sees it. Fenced + inline code are protected; a mathy-content heuristic (`\`, `^`, `_`, `{`) leaves `$5`/`$10` currency literal. Verified e2e: `∑ᵢ₌₁ⁿ xᵢ²`, `α ≤ β`, currency intact, `` `$x_i$` `` untouched.
- **12 math tests**; **213 total green**, snapshots unchanged, fmt + clippy clean, reinstalled. Attribution note in `vendor/NOTICE` (original impl, standard Unicode mappings).

**Next ports:** JSON viewer (`glance data.json` → colored/indented via our paint model, `.json` feature-detect in lib) and mermaid box-art (simplest fenced `mermaid` flowcharts). **Then STOP** — the rest of Phase 5 is launch (crates.io/Homebrew/cargo-dist/GIFs/Show HN), which is outward-facing and needs the user.

---

## 2026-07-22 — Phase 4: slide mode + HTML export → **PHASE 4 COMPLETE** · JAY-94
**Phase:** 4 (✅) · **Focus:** differentiators · **Branch:** `feature/jay-91-phase-1-viewer-core`

- **Slide mode** (`-s`): `view::slides::split_slides` (pure — splits blocks on `Block::ThematicBreak`, drops empties) + `Slides` nav (clamped next/prev/first/last) — 4 unit tests. `app::run_slides` is a minimal event loop: lay out the current slide, vertically center it, footer `slide n/N`; `→`/Space/l/j/↓ next, `←`/h/k/b/↑ prev, g/Home first, G/End last, q quits. Full repaint per transition. Wired in `lib.rs` (`-s` + interactive stdout).
- **HTML export** (`--export html`): new `src/export.rs::to_html` — pulldown-cmark's `push_html` (enabled the crate's `html` feature) on the original markdown, wrapped in a `<!doctype html>` doc with an **inlined `<style>`** derived from the theme (brand accent `#FF5800`, `color-scheme` dark/light, code/table/blockquote styling). Fully self-contained (no external links/scripts/fonts). Printed to stdout, exits — works regardless of TTY. 3 unit tests (self-contained doc, tables+code+accent, light/dark switch).
- **Bug caught + fixed:** an opportunistic "read stdin as document" in the final input-resolution path made the `no_args_does_not_panic` test **block on stdin** in non-EOF harnesses. Reverted it — `glance < x.md` / `cat x | glance` in a terminal already stream (stdout-TTY-gated branch), so only the rare fully-piped `cat x | glance | cat` reverts to prior behavior (no regression).
- **202 total green** (+4 slides, +3 export), fmt + clippy clean, reinstalled. Binary 4.26 MB (pulldown `html` feature).

**PHASE 4 COMPLETE** — streaming stdin + slide mode + HTML export. → Phase 5 (JAY-95): ports (math `$…$`→unicode, mermaid, json viewer) + launch (README parity/benchmark tables, cargo-dist, crates.io).

---

## 2026-07-22 — Phase 4: streaming stdin (the llm|glance demo) · JAY-94
**Phase:** 4 (🟨) · **Focus:** differentiators · **Branch:** `feature/jay-91-phase-1-viewer-core`

- **Key enabler confirmed by reading crossterm 0.28 source:** its event source reads from `tty_fd()` = `/dev/tty` (not stdin fd 0). So in streaming mode the piped document (stdin) and interactive keys (/dev/tty) coexist with **no manual tty handling**.
- New `src/stream.rs` (all pure bits tested): `stable_boundary(text)` — the **fence-aware** stable/active split (last blank line *outside* a ```` ``` ````/`~~~` fence; a blank inside a streamed code block is never a false boundary). `StreamState.append(bytes)` accumulates **bytes** (a chunk can split a UTF-8 char) and re-parses only the active tail, caching stable-prefix blocks. `StreamReader::spawn_stdin` (reader thread → channel, EOF closes it). `key_pauses_follow`/`key_resumes_follow`. 5 unit tests (boundary cases, fenced code, tail re-parse, streamed code block, follow keys).
- `view::app::run` gained a `stream` param: drains bytes each tick → `ViewerState::set_blocks` (relayout, clamp scroll) → auto-`to_bottom` while following; re-enqueues highlight/images (doc grew); repaints. Scroll-up pauses follow, `G`/End resumes; a persistent `stream_pill` (`▼ following` / `▼ paused (G to follow)`) shows in the status row (new `ViewerState` field, not cleared by keypress like `toast`). First paint isn't blocked — nothing renders until the first chunk.
- `lib.rs`: streaming detected when **stdin piped + stdout TTY + no file**; also wired plain **stdin pipe-mode** (`glance < x.md`, `cat x | glance | cat`) via `std::io::read_to_string(stdin)`.
- **195 total green**, fmt + clippy clean, reinstalled. Binary 4.23 MB (streaming adds no deps), first-paint 0.52 ms.

**Next:** slide mode (`-s`: `---` splits slides, remapped keys), then HTML export (`--export html`). Then Phase 4 done → Phase 5 (ports + launch).

---

## 2026-07-22 — Phase 3: background image worker → **PHASE 3 COMPLETE** · JAY-93
**Phase:** 3 (✅) · **Focus:** highlight + images · **Branch:** `feature/jay-91-phase-1-viewer-core`

- New `view::images::ImageLoader` — the background fetch/decode/render worker (highlighter's producer/mpsc/drain shape). Worker: fetch (`ureq` for http/https with a 20 MB cap; `std::fs` for local, `file://`-stripped and relatives resolved against the doc dir) → `image::load_from_memory` → `half_block` at target cols → send lines. Pure/tested `is_remote` + `resolve_local` (4+ cases).
- **The relayout wrinkle** (vs syntect's in-place patch): a rendered image is N rows from a 1-row placeholder, so `md::layout` gained `layout_document_with(blocks, w, ln, &ResolvedImages)` — a resolved image ordinal expands its placeholder to the rendered rows (indices recomputed; `ImageRef.end` grows). Tested: a fake 4-row resolve shifts the following heading down by 3. `ViewerState` holds a `resolved_images` map, `set_resolved_image(idx, lines)` inserts + relayouts (scroll clamped, search re-run), and clears it on resize/reload/nav (renders are width-specific).
- `view::app`: spawns the loader after first paint (gated on color depth), enqueues images visible-first (cached by (tab,idx,cols) so scroll/resize don't re-fetch), drains results each tick → `set_resolved_image` + full repaint when a visible image lands. Re-enqueues on resize/tab-switch/reload. **first-paint never blocks on fetch/decode.**
- Kitty *display* integration (raw passthrough + row reservation into the damage-diff renderer) is a documented fast-follow; the encoder (`kitty_png`) is done + tested. All images currently render via the universal half-block path (works in every color terminal, incl. Kitty).
- **Checkpoints:** binary **4.21 MB** (exactly the ADR 0006 target; 2.2× < mdterm's 9 MB); first-paint **0.65 ms** (test.md) / **13.2 ms** (5k-line) — images off the hot path. **190 total green**, fmt + clippy clean, reinstalled.

**PHASE 3 COMPLETE** — syntect (core + worker) + full image ladder. → Phase 4 (JAY-94): streaming stdin (the `llm | glance` live-render demo), slide mode, HTML export.

---

## 2026-07-22 — Phase 3: Kitty renderer + image node detection · JAY-93
**Phase:** 3 (🟨) · **Focus:** highlight + images · **Branch:** `feature/jay-91-phase-1-viewer-core`

- `term::images::kitty_png` — Kitty graphics-protocol encoder: base64 the PNG bytes, chunk into ≤4096-byte pieces, first chunk carries `a=T,f=100,m=…`, continuation chunks `m=1`/`m=0`, each framed `ESC_G … ESC\`. Pure/tested (single-chunk framing, multi-chunk `m` flag, empty input). Renderer choice will follow the probed `ImageProtocol` (Kitty → this; else `half_block`; None → placeholder).
- `md::layout`: standalone-image detection — a paragraph that is just `![alt](url)` (only whitespace/breaks around one image) becomes a dim `⌛ image: alt (url)` **placeholder line** + an `ImageRef {start,end,url,alt}` recorded in `DocLayout.images` (like `code_blocks`). An image mixed with real text stays inline (alt in the flow), not an ImageRef. 2 tests.
- **187 total green**, fmt + clippy clean. Binary unchanged (worker not wired yet).

**Next (closes Phase 3):** the background image worker — add `ureq` (the +1.2 MB TLS per ADR 0006), reuse the highlighter's producer/mpsc/drain pattern: fetch (ureq http/https, fs for local resolved against the doc dir) → decode → render (half_block at target cols, or Kitty) → patch. Because a rendered image is N lines but the placeholder is 1, image-ready triggers a **re-layout** with resolved images injected (indices recomputed, scroll clamped) rather than an in-place span patch. Enqueue visible images first; cache decoded result. first-paint never blocks on fetch. Then mark JAY-93 Done → Phase 4.

---

## 2026-07-22 — Phase 3: image ladder foundation (probe + half-block) · JAY-93
**Phase:** 3 (🟨) · **Focus:** highlight + images · **Branch:** `feature/jay-91-phase-1-viewer-core`

- **Size checkpoint → user decision ([ADR 0006](adr/0006-bundle-tls-for-remote-images.md)):** spiked the deps before building. `image` (png+jpeg, no defaults) = **+258 KB → 2.9 MB** (cheap). `ureq`+rustls for remote HTTPS = **+1.2 MB → 4.1 MB** (all TLS), over the 3.5 MB "erases the win" line — so I **stopped and asked**. User chose **include remote via rustls** (4.1 MB, still ~2.2× < mdterm's 9 MB). Reverted the spike; added `image` for real (used now), `ureq` lands with the fetch worker next.
- New `caps::ImageProtocol` (Kitty | HalfBlock | None) + pure `detect_image_protocol` (Kitty via `TERM` contains kitty / `KITTY_WINDOW_ID` / `TERM_PROGRAM` ghostty|wezterm; else half-block on any color; None without color) — 3 tests. `Capabilities` gained an `images` field.
- New `term::images::half_block` — the universal renderer: scale an image to `cols × (rows*2)` pixels and emit `▀` cells with `fg` = top sub-pixel, `bg` = bottom sub-pixel (doubles vertical resolution, works in any color terminal). Pure/testable (2×2 red-over-blue → fg=red/bg=blue) + `cell_size` aspect math. To carry raw per-cell RGB, `style::Style` gained explicit `fg`/`bg: Option<Rgb>` (overriding the role→theme mapping); `paint` emits them as truecolor SGR (downsampled). All existing sites use `Default` → unaffected.
- **183 total green**, fmt + clippy clean. Binary still 2.66 MB (renderer not yet reachable from `main` → `image` DCE'd; +258 KB lands when image nodes are wired next).

**Next:** Kitty renderer (base64 PNG in the APC graphics sequence, fast path when probed) + wire markdown image nodes — layout a placeholder line instantly, then a background worker (reuse the highlighter's patch pattern; add `ureq` here) fetches (fs/http) + decodes + scales + patches in. first-paint never blocks on fetch. Closes Phase 3.

---

## 2026-07-22 — Phase 3: background highlight worker (syntect live) · JAY-93
**Phase:** 3 (🟨) · **Focus:** highlight + images · **Branch:** `feature/jay-91-phase-1-viewer-core`

- New `view::highlighter` — the background worker, same shape as the auto-reload watcher (producer thread + two mpsc channels + a drain step in the loop). `Highlighter::spawn()` runs a thread that owns the `SyntaxSet` (its **first** `highlight` call lazily loads it — on that thread, off the startup path), renders each block to ready-to-patch `Line`s via `layout_code_with` (layout work also off the UI thread), and returns a `HighlightResult`. `blocks_by_priority(top,height)` orders on-screen blocks first (pure, 2 tests).
- `md::layout`: factored `render_code_rows` out of `layout_code` and exposed `layout_code_with(rows,…)` — same gutter/width/`no_wrap` handling, so a syntect render has identical line geometry (row count == source-line count) and can be patched in without shifting indices. All code spans marked `code`.
- `ViewerState`: `patch_code_block(idx, lines)` replaces a block's display lines in place (rejects wrong count → indices/search stay valid); `code_block_visible(idx)` gates repaints. `Tabs::get_mut`. (2 tests.)
- `view::app`: spawns the worker after first paint, enqueues the active tab's blocks (visible-first, skipping no-lang blocks), and drains results each loop tick — patching the owning tab and repainting only when the *active* tab's *visible* content changed. Stale-geometry results (post-resize / `l` toggle) are dropped by width+line-number checks; resize/tab-switch/reload re-enqueue. The loop now **always** `event::poll(50 ms)` (was watcher-gated) so highlight upgrades land while idle. Micro-tokenizer output shows instantly; syntect upgrades it progressively.
- **Checkpoints (both pass):** release binary **2.6 MB** (< 3 MB; 3.4× < mdterm's 9 MB); first-paint **3.7 ms** (test.md) / **12.9 ms** (5k-line, 384 KB) — syntect adds **0 ms** to first paint (worker-thread, lazy). **176 total green**, fmt + clippy clean, reinstalled.

**Next:** the image ladder — vendor `image.rs` → adapt to the `Line`/cell model, wire Kitty + half-block (capability probe in `caps.rs`), background fetch/decode/scale, placeholder line until ready, crop cache. Closes Phase 3.

---

## 2026-07-22 — Phase 3: syntect highlighter core (scope→role) · JAY-93
**Phase:** 3 (🟨) · **Focus:** highlight + images · **Branch:** `feature/jay-91-phase-1-viewer-core`

- **Size checkpoint first (spike):** added syntect, forced its default dump to link via a temp probe, measured `--release`: **+500 KB → 2.48 MB total** (baseline had already grown to 1981 KB from regex/notify/toml — the 831 KB figure was Phase-1-scaffold-only). 2.48 MB is still **3.6× smaller than mdterm's 9 MB** and well under the 3 MB stop-threshold, so I proceeded without interrupting. Reverted the spike; corrected `docs/benchmarks.md` size table.
- New `md::syntect_hl` — the *accurate* highlighter (75 languages). **Parsing-only** syntect (features `parsing`/`default-syntaxes`/`regex-fancy`, no theme engine, no onig C dep): each token's syntect **scope** is mapped to our existing `Role` enum (`comment`→Comment, `string`→Str, `constant.numeric`→Number, `entity.name.function`→Function, `keyword`/`storage`→Keyword), so glance's own dark/light + OSC 11 theming colors it and the binary stays small. `SyntaxSet` loads lazily via `OnceLock` — **never on the startup path** (mdterm's bug); the micro-tokenizer stays the instant cold path. `highlight(code, lang) -> Option<Vec<Vec<Span>>>` returns `None` for unknown langs (caller keeps the micro-tokenizer). 4 unit tests (unknown→None, rust keyword/string/comment roles, alias resolution, numbers).
- **172 total green**, fmt + clippy clean. Currently unused → DCE'd from the shipped binary (still ~2.0 MB); the +500 KB lands when the worker wires it in next.

**Next:** wire the **background highlight worker** — a thread that owns the `SyntaxSet`, receives (block content, lang, id) requests (visible blocks first), and sends highlighted spans back over a channel; the event loop (which already polls for watcher events) patches the affected code block's layout lines and repaints. Then the image ladder.

---

## 2026-07-22 — Phase 2: click-to-copy → **PHASE 2 COMPLETE** · JAY-92
**Phase:** 2 (✅) · **Focus:** interactivity · **Branch:** `feature/jay-91-phase-1-viewer-core`

- `ViewerState::code_block_at(row)` — pure hit-test: `doc_line = top + row`, find the `CodeRef` whose `start..end` contains it, return its content (2 unit tests: hit at top incl. first/last row + miss on a paragraph row; hit after scrolling, exercising the offset math). Reuses the existing copy stack + toast — no new copy code.
- `view::app`: a `Mouse::Click{row}` arm placed **above** the generic mouse arm intercepts left-clicks (wheel still scrolls via the fall-through). On a hit it calls `copy_to(out, state, "code block", …)` (OSC 52 write + native fallback + toast) and repaints.
- **168 total green**, fmt + clippy clean.

**PHASE 2 DONE** — search, TOC, fuzzy, links + local-file nav, copy stack (`c`/`Y`/`p` + click), help, theme toggle, line numbers, tabs, auto-reload, OSC 11 auto-theme, click-to-copy all shipped. Weakness #3 (broken copy) and #4 (no callouts, done in Phase 1) fixed.

**Next → Phase 3 (highlight + images):** start with **syntect lazy highlight** on a worker thread — never `load_defaults` on startup (mdterm's exact bug); the regex micro-tokenizer stays the instant cold path; highlight visible code blocks first and patch frames in. Then the image ladder (vendor `image.rs`, wire Kitty + half-block, capability probe in `caps.rs`, background fetch/decode/scale, placeholder until ready).

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

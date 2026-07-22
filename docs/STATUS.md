# glance — status log

Newest first. One entry per working session. Template at the bottom.
Weekly summaries can be generated with the `operations:status-report` skill.

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

# ADR 0002 — Clean-room build, vendor mdterm's strong modules

**Status:** Accepted (2026-07-22)

## Context
The reference (`bahdotsh/mdterm`, MIT, ~14.7k LOC) is a monolith — `viewer.rs` alone is 4,356 LOC,
which the spec explicitly dislikes for iteration. But its *hardest, most correct* modules are
exactly the ones our roadmap fears most: `image.rs` (the full Kitty/iTerm2/Sixel/half-block ladder),
`diagram.rs` (mermaid box-art), `json.rs`, and the math symbol tables. Two options:
fork-and-fix (drags the whole tree along) vs clean-room-and-vendor (take only the pearls).

## Decision
**Clean-room architecture** — our own event loop, layout, and renderer, structured cleanly.
**Vendor only the strong modules** into `src/ports/` (mermaid, json, math) and `src/term/images/`,
with MIT attribution in `vendor/NOTICE`. Rewrite exactly the four weaknesses + add callouts/streaming/OSC 52.

## Consequences
- ✅ Inherit hard-won correctness (images, ports) without the monolith's debt.
- ✅ Phases 3 and 5 become adaptation, not from-scratch work.
- ➖ Must adapt vendored code at seams to our `Line`/cell model; keep vendored files isolated.
- ➖ License hygiene required: `vendor/NOTICE` + isolation, surveyed in Phase 0.

## Phase 0 survey findings (2026-07-22) — see `docs/vendor-survey.md`
Coupling to mdterm internals (drives adaptation cost):
- **`image.rs` (~3299 LOC) — Low.** Zero `crate::` imports, no Theme/Style; only `crossterm::Color` at the edge. **Portable ~as-is (Phase 3).**
- **math (`render_math`, ~215 LOC) — Low.** Pure `&str → String`. Copy verbatim (Phase 5).
- **`diagram.rs` (~1135 LOC) — Medium.** Uses `Style`/`StyledSpan` + 4 Theme fields; owns the shared `Canvas`/`CardDrawRow` primitive.
- **`json.rs` (~2293 LOC) — High.** ~6 `crate::style` types, ~15 Theme slots, and **reaches into `diagram`'s `Canvas`/`CardDrawRow`.**

**Decision refinement:** vendor **`json.rs` + `diagram.rs` together as one unit** (Phase 5) — do not fork two canvases. License: mdterm is MIT © 2026 Gokul; full text in `vendor/NOTICE`.

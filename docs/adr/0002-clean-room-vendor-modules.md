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

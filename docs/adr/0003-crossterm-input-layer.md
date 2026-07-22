# ADR 0003 — Use crossterm for input; hand-write paint/damage

**Status:** Accepted (2026-07-22)

## Context
The spec wanted a fully hand-rolled ~400 LOC terminal layer. But its own #1 risk is
"raw-mode input edge cases (Windows ConPTY, paste, mouse encodings)." Hand-rolling that is
re-fighting a solved battle across three OSes.

## Decision
Use **`crossterm`** for raw mode, key/mouse decoding, capability bits, and Windows ConPTY.
**Hand-write only the paint + damage-diff renderer** on top of crossterm's `queue!` — the part
that actually differentiates glance (first-paint + damage frames).

## Consequences
- ✅ Kills the Windows/input risk; input becomes a thin adapter (`crossterm::Event → app Event`).
- ✅ Keep full control of the render path and the <80 ms first-paint budget.
- ➖ A dependency (small, ubiquitous) rather than zero-dep purity.

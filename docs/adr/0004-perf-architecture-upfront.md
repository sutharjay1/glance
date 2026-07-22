# ADR 0004 — Build the full perf architecture from Phase 1

**Status:** Accepted (2026-07-22)

## Context
Speed is the product's whole point (`<80 ms` first paint, live reflow). Options ranged from
"correctness first, optimize later" to "full perf architecture now." A later rewrite of the
render path would jeopardize the launch pitch.

## Decision
Bake the perf architecture in from Phase 1: **layout cache keyed `(block.id, width)`**,
**viewport-first layout** + background-chunked tail, **damage-conscious frames** (rewrite only
changed rows) inside **synchronized output** (`CSI ?2026`), and the **ASCII width fast path** with
incremental accumulation. Highlighting stays lazy (never on the startup path).

## Consequences
- ✅ The <80 ms target and jitter-free live reflow are structural, not bolted on.
- ➖ Damage-diff rendering is the most bug-prone path → keep a `--full-repaint` debug fallback and snapshot the diff logic.
- ➖ More upfront complexity than a naive full-repaint loop.

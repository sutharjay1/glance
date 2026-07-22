# Benchmarks — reference baseline (Phase 0)

Baseline measurements of the reference (`bahdotsh/mdterm` v2.0.0, release build) so we can
build the README comparison table once glance exists. **glance rows are TBD** — filled in as
phases land.

**Machine:** Apple M5 Pro · 15 cores · 24 GB · macOS (Darwin 25.5.0)
**Reference:** mdterm @ release profile (no LTO/strip/panic-abort), built in ~20 s
**Tool:** hyperfine 1.20.0, `--warmup 5`
**Date:** 2026-07-22

## Binary size
| Build | Size |
|---|---|
| mdterm (release, unoptimized profile) | **9.7 MB** |
| glance target (LTO + `codegen-units=1` + strip + panic=abort) | **~3–6 MB** (TBD) |

mdterm ships no size optimizations; glance's release profile should beat it comfortably while
staying far under a Bun-compiled binary (60–90 MB).

## Launch → render (non-interactive modes give clean, TTY-free numbers)

### Export (`--export html`)
| Doc | Size | mdterm | glance |
|---|---|---|---|
| mdterm-test.md | 7.9 KB | **60.2 ms** ± 0.8 | TBD |
| big-5k.md | 384 KB | **61.8 ms** ± 0.8 | TBD |

### Pipe (`--no-color` → `cat`)
| Doc | Size | mdterm | glance |
|---|---|---|---|
| mdterm-test.md | 7.9 KB | **63.3 ms** ± 1.1 | TBD |
| big-5k.md | 384 KB | **46.3 ms** ± 1.0 | TBD |

## Key finding — the weakness, quantified

**mdterm's time is dominated by fixed startup cost, not document work.** Export takes 60.2 ms
for a 7.9 KB doc and 61.8 ms for a 384 KB doc — **~identical despite 48× more content**. The
floor (~46–60 ms) is `SyntaxSet::load_defaults()` + `ThemeSet::load_defaults()` running at boot
(`markdown.rs:1545`), paid whether or not any code needs highlighting.

**Implication for glance:** keeping syntect off the startup path (lazy, worker-thread, viewport-first
— ADR 0004) should put our first paint well under mdterm's ~60 ms floor. The `<80 ms` target in the
ROADMAP is the *interactive first-paint* budget and includes parse + viewport layout; on a native
binary with no eager syntax loading, ~10–25 ms is the realistic expectation. This is the core of
the launch pitch and the reason the perf architecture is built in from Phase 1.

(Note: pipe/big-5k is *faster* than pipe/test.md because `mdterm-test.md` exercises images, mermaid,
math, and links — heavier per-block work — while big-5k.md is bulk text/code. Confirms the fixed
cost is startup, and the variable cost is feature-triggered, not size-triggered.)

## How to reproduce
```bash
BIN=reference/target/release/mdterm
hyperfine --warmup 5 --shell=none \
  -n "export test"  "$BIN --export html tests/fixtures/mdterm-test.md" \
  -n "export big"   "$BIN --export html tests/fixtures/big-5k.md"
hyperfine --warmup 5 \
  -n "pipe test" "$BIN --no-color tests/fixtures/mdterm-test.md </dev/null | cat" \
  -n "pipe big"  "$BIN --no-color tests/fixtures/big-5k.md </dev/null | cat"
```

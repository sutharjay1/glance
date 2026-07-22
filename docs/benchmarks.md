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
| mdterm (release, unoptimized profile) | 9.0 MB |
| **glance** — Phase 1 scaffold (LTO + `codegen-units=1` + strip + panic=abort) | 831 KB |
| **glance** — Phase 2 complete (adds regex/notify/toml/serde) | ~2.0 MB |
| **glance** — Phase 3 with syntect wired (fancy-regex, parsing-only, no themes) | 2.6 MB |
| **glance** — Phase 3 complete (+ image decode + rustls for remote images, [ADR 0006](adr/0006-bundle-tls-for-remote-images.md)) | **4.1 MB** |

glance is **~2.2× smaller** than mdterm with the full feature set — syntect's 75-language
highlighter, image decode (png+jpeg), and a bundled TLS stack for remote images — and far under a
Bun-compiled binary (60–90 MB), validating the Rust decision (ADR 0001). Every heavy feature is
loaded/executed **off the first-paint path** (ADR 0004): syntect on a worker (+500 KB, its dump is
flate2-compressed + embedded), image fetch/decode on a worker (+258 KB image, +1.2 MB rustls). So
the size buys capability without touching the startup latency the micro-tokenizer + instant layout
own — first paint stays **0.65 ms** (test.md) / **13 ms** (5k lines) regardless.

## Launch → render — head-to-head (2026-07-22, Apple M5 Pro)

Both binaries, pipe mode (launch → render → exit), `hyperfine --warmup 5 -N`:

| Doc | Size | mdterm | **glance** | glance advantage |
|---|---|---|---|---|
| mdterm-test.md | 7.9 KB | 59.1 ms ± 1.2 | **1.7 ms** ± 0.2 | **35× faster** |
| big-5k.md | 384 KB | 43.3 ms ± 1.3 | **10.4 ms** ± 0.8 | **4.2× faster** |

First-paint proxy (parse + viewport layout, `glance --timing`): **0.92 ms** for the reference
doc (0.29 ms warm), **8.2 ms** for the 384 KB / 11,758-line doc. The `<80 ms` ROADMAP budget is
beaten by ~85×.

## Key finding — the weakness, quantified

**mdterm's time is dominated by fixed startup cost, not document work.** Export takes 60.2 ms
for a 7.9 KB doc and 61.8 ms for a 384 KB doc — **~identical despite 48× more content**. The
floor (~46–60 ms) is `SyntaxSet::load_defaults()` + `ThemeSet::load_defaults()` running at boot
(`markdown.rs:1545`), paid whether or not any code needs highlighting.

**Confirmed for glance:** keeping syntect off the startup path (ADR 0004) + a native binary
removes both the runtime boot tax and the eager-load floor. Result: **0.92 ms** parse+layout for
the reference doc vs mdterm's ~60 ms — a ~65× reduction, and 35× faster end-to-end in pipe mode.
The perf architecture built in from Phase 1 delivered exactly the launch pitch.

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

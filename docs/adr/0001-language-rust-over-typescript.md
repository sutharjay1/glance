# ADR 0001 — Language: Rust over TypeScript

**Status:** Accepted (2026-07-22)

## Context
The original spec proposed Bun + TypeScript, arguing the reference's slowness is "eager work,
not language." That's only half true. The product's headline metrics are **first-paint latency**
and **binary size**, and both have language-level floors the spec ignored:
- Runtime boot tax: native ~1–5 ms vs Node ~30–50 ms / Bun ~10–20 ms before user code runs.
- Compiled size: Rust ~3–8 MB vs Bun's admitted 60–90 MB (embeds the runtime).
- Latency jitter: native has no GC/JIT variance — critical for the live-streaming reflow demo.

Priorities were explicitly ranked: **faster + smaller matter; ease-of-implementation does not.**

## Decision
Build glance in **Rust**.

## Consequences
- ✅ Owns the metrics we sell (startup, size, jitter, throughput).
- ✅ Same language as the reference → its strongest modules become vendorable (see ADR 0002).
- ➖ Slower to write; borrow-checker friction (mitigated by a value-oriented data model).
- ➖ Distribution is prebuilt binaries + Homebrew + `cargo install`, not `bunx` (npm reach was deprioritized).

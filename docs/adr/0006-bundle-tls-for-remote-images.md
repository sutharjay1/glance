# ADR 0006 — Bundle a TLS stack for remote images

**Status:** Accepted (2026-07-22)

## Context
The image ladder (Phase 3) can render both local/relative image paths and remote `http(s)`
image URLs (badges, hosted screenshots). Local decode via the `image` crate (png+jpeg only) is
cheap — **+258 KB**. Remote fetch needs an HTTP client; `ureq` with its bundled **rustls** TLS
stack adds **+1.2 MB** (2.9 MB → 4.1 MB), because HTTPS requires bundled crypto + root certs.
This directly tensions the product's headline: *small binary* (benchmarks lead with the size win
vs mdterm's 9 MB). Measured empirically via a size spike before building anything.

Options weighed:
- **Local-only now** (2.9 MB), remote as a fast-follow behind a cargo feature.
- **Remote via rustls** (4.1 MB) — full support out of the box, self-contained binary.
- **Remote via native-tls** (~3.4 MB) — smaller, but links a system TLS dependency (macOS
  Security.framework / Linux OpenSSL), adding cross-platform build/packaging variance.

## Decision
**Bundle rustls (via `ureq`) and support remote images out of the box** (user decision). A
self-contained 4.1 MB binary that "just works" on any markdown with remote images beats a smaller
binary that silently can't render half the images in real-world docs. 4.1 MB is still **~2.2×
smaller than mdterm's 9 MB**, so the size story survives.

## Consequences
- ✅ Remote and local images both work with zero system dependencies (static binary, easy distribution).
- ✅ Decision made on a measured number, not a guess (size spike, reverted before building).
- ➖ +1.2 MB is pure TLS; the "small binary" lead narrows from ~3.4× to ~2.2× vs mdterm.
- ➖ If size becomes a launch concern, revisit: a `remote-images` cargo feature could ship a
  2.9 MB local-only default build with rustls opt-in. Fetch stays off the first-paint path
  regardless (background worker), so this is purely a binary-size tradeoff, never a latency one.

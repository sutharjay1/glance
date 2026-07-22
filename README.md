# glance

**A fast terminal markdown viewer — the one that renders your LLM's output live.**

```sh
llm "explain rust lifetimes with code" | glance
```

`glance` renders Markdown in your terminal with syntax highlighting, images, live search,
and clickable links — starting in **under a millisecond** and staying smooth while a stream
of text pours in. It's a clean-room Rust reimplementation targeting feature parity with
[`mdterm`](https://github.com/bahdotsh/mdterm), while fixing its four biggest weaknesses.

---

## Why glance

| | mdterm | **glance** |
|---|---|---|
| First paint (7.9 KB doc) | 59.1 ms | **1.7 ms** — *35× faster* |
| Render (384 KB / 5k lines) | 43.3 ms | **10.4 ms** — *4.2× faster* |
| Binary size | 9.0 MB | **4.34 MB** — *2.1× smaller* |
| Streaming stdin (`llm \| glance`) | ✗ (blocks until EOF) | ✅ live, auto-follows |
| Copy over SSH / Wayland / tmux | ✗ | ✅ OSC 52 + native fallbacks |
| GitHub callouts (`> [!NOTE]`) | ✗ | ✅ |

`glance` is native and keeps every heavy feature — syntect's 75-language highlighter, image
decoding, TLS for remote images — **off the first-paint path** (loaded lazily on worker
threads). mdterm's slowness is `SyntaxSet::load_defaults()` at boot, paid on every launch
whether or not any code needs highlighting; glance never pays it up front.

Benchmarks: Apple M5 Pro, `hyperfine --warmup 5`. Full methodology in
[`docs/benchmarks.md`](docs/benchmarks.md).

---

## Install

Prebuilt binaries and `cargo install glance` / Homebrew are coming with the first release.
For now, from source (Rust toolchain required):

```sh
git clone <repo> && cd glance
cargo install --path .
```

The release profile (`lto`, `codegen-units=1`, `strip`, `panic=abort`) produces a ~4.3 MB
static binary.

---

## Usage

```sh
glance README.md               # view a file
glance a.md b.md               # multiple files → Tab / Shift+Tab
llm "..." | glance             # live-render a stream (auto-follows the bottom)
glance data.json               # colored, indented JSON viewer
glance -s slides.md            # slide mode (--- splits slides)
glance --export html doc.md    # self-contained themed HTML to stdout
cat doc.md | glance | less     # clean styled pipe output
```

### Keys

| | |
|---|---|
| `j`/`k`, `↑`/`↓`, wheel | scroll |
| `Space`/`b`, `d`/`u` | page / half-page |
| `g`/`G` | top / bottom |
| `[` / `]` | previous / next heading |
| `/` … `n`/`N` | search (regex), cycle matches |
| `o` | table of contents · `:` fuzzy heading filter |
| `f` | link picker · `Backspace` back (local `.md` links) |
| `c` / `Y` / `p` | copy nearest code block / whole doc / file path |
| *click* | copy a clicked code block |
| `t` | toggle theme · `l` toggle code line numbers |
| `Tab` / `Shift+Tab` | switch files |
| `h` / `?` | help · `q` quit |

Auto-reload is automatic — edit a viewed file and it refreshes, preserving scroll + search.

### CLI

```
-T, --theme <dark|light>    theme (else auto-detected via OSC 11)
-w, --width <N>             display width override (0 = auto)
-s, --slides                slide mode
-l, --line-numbers          line numbers in code blocks
-f, --follow                follow a file/stdin as it grows
    --export <html>         export instead of viewing
    --no-color              disable ANSI colors
    --pipe                  force non-interactive styled output
```

Config: `~/.config/glance/config.toml` (`theme`, `line_numbers`, `width` — same keys as
mdterm, so an existing config migrates unchanged). CLI flags override config.

---

## Features

Rendering — headings, bold/italic/strikethrough, ordered/unordered/task lists, blockquotes,
**GitHub callouts** (`> [!NOTE|TIP|IMPORTANT|WARNING|CAUTION]`), tables, thematic breaks,
inline + fenced code. Syntax highlighting is instant (a regex micro-tokenizer) and upgrades
to full **syntect** (75 languages) on a background thread.

Interactivity — search, TOC, fuzzy headings, link picker, local-file navigation, multi-file
tabs, auto-reload, theme + line-number toggles, and a copy stack that works over SSH, tmux,
Wayland, and X11 (OSC 52 first, native `pbcopy`/`wl-copy`/`xclip`/`clip` fallbacks).

Images — standalone images render as **half-blocks** (any color terminal) with a Kitty
fast-path; local and remote (`http(s)`) sources fetch and decode on a worker thread, so first
paint never blocks on the network.

Differentiators — **streaming stdin** for the `llm | glance` live demo (stable-prefix/
active-tail reparse, auto-follow with a pause pill), slide mode, and self-contained HTML
export.

Ports — inline `$…$` **LaTeX → Unicode** math, a colored **JSON** viewer, and **mermaid**
flowcharts rendered as Unicode box-art.

---

## Architecture

Layered and value-oriented: `term` (capabilities/ANSI/input/OSC) · `md` (parse → layout →
highlight) · `style` → `paint` (semantic spans → ANSI, downsampled to the terminal's color
depth) · `view` (state, event loop, overlays, background workers). Three background workers
(auto-reload, syntect, image fetch) share one event-loop spine; a `Block::Prerendered` seam
lets ports (JSON, mermaid) reuse the whole pipeline.

Design decisions are recorded in [`docs/adr/`](docs/adr/); the phase plan and status log live
in [`ROADMAP.md`](ROADMAP.md) and [`docs/STATUS.md`](docs/STATUS.md).

---

## Development

```sh
cargo test          # 225 unit + property + snapshot tests
cargo clippy        # -D warnings clean
cargo fmt --check
glance --timing f   # print parse + layout time (first-paint proxy)
```

---

## Credits & license

Clean-room Rust, with the strongest reference modules adapted under MIT (attribution in
[`vendor/NOTICE`](vendor/NOTICE)). `mdterm` is MIT © Gokul. License: MIT.

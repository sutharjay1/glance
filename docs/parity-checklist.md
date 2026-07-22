# Parity checklist — mdterm v2.0.0

Permanent gate: every reference feature must be ticked before launch. Seeded from the spec;
verify against `viewer.rs` during Phase 0. Legend: ⬜ todo · 🟨 partial · ✅ done.

## Features (25)
| # | Feature | Phase | Status |
|---|---|---|---|
| 1 | Interactive TUI (alt-screen, raw mode, keyboard + mouse, resize) | 1 | ⬜ |
| 2 | Syntax highlighting (micro-tokenizer v1 → syntect lazy) | 1/3 | ⬜ |
| 3 | Rich formatting (headings, bold/italic/strike, lists, tasks, quotes, tables, rules, inline code) | 1 | ⬜ |
| 4 | Inline images (Kitty → iTerm2 → Sixel → half-block) | 3 | ⬜ |
| 5 | Clickable links (OSC 8 + ` (url)` fallback) | 1/2 | ⬜ |
| 6 | In-document search (`/`, regex, highlight, `n`/`N`, `Esc`) | 2 | ⬜ |
| 7 | Table of contents (`o`) | 2 | ⬜ |
| 8 | Fuzzy heading search (`:`) | 2 | ⬜ |
| 9 | Heading jumps (`[` / `]`) | 1 | ⬜ |
| 10 | Local file links (follow `.md`, `Backspace` history) | 2 | ⬜ |
| 11 | Link picker (`f`) | 2 | ⬜ |
| 12 | Click-to-copy (code/heading/list; `Y` doc; `c` nearest) | 2 | ⬜ |
| 13 | Mermaid diagrams (box art) | 5 | ⬜ |
| 14 | Math (`$...$` → Unicode) | 5 | ⬜ |
| 15 | Slide mode (`--slides`) | 4 | ⬜ |
| 16 | Auto-reload (watch + debounce, preserve scroll/search) | 2 | ⬜ |
| 17 | Stdin (`cat x.md | glance`, keys via `/dev/tty`) | 1 | ⬜ |
| 18 | Multiple files (`Tab`/`Shift+Tab`, per-file scroll) | 2 | ⬜ |
| 19 | HTML export (`--export html`) | 4 | ⬜ |
| 20 | Themes (dark/light, `t`, `-T`, config; dark accent `#FF5800`) | 2 | ⬜ |
| 21 | Line numbers in code (`l`, `-l`, config) | 2 | ⬜ |
| 22 | Config file (`~/.config/glance/config.toml`) | 1 | ⬜ |
| 23 | Word wrapping (hanging indents, resize relayout) | 1 | ⬜ |
| 24 | JSON viewer (`glance data.json`) | 5 | ⬜ |
| 25 | Pipe-friendly (no TTY → styled text; `--no-color`; `--pipe`) | 1 | ⬜ |

## Beyond the reference (our differentiators)
| Feature | Phase | Status |
|---|---|---|
| Copy everywhere (OSC 52 first + wl-copy/xclip/xsel/pbcopy/PowerShell) | 2 | ⬜ |
| `p` — copy full file path | 2 | ⬜ |
| Streaming stdin + follow mode (`llm | glance`) | 4 | ⬜ |
| GitHub callouts (`> [!NOTE]` …) | 1 | ⬜ |
| Narrow-table record/card view | 1 | ⬜ |
| Escape-sequence sanitization | 1 | ⬜ |
| Color downsampling (truecolor→256→16) | 1 | ⬜ |
| Auto theme via OSC 11 | 2 | ⬜ |

## Keybindings (§5) — verify each with a PTY test
`j/k/↓/↑`/wheel · `Space/PgDn` `b/PgUp` · `d/u` · `g/Home` `G/End` · `[`/`]` · `/` `n`/`N` `Esc` · `o` · `:` · `f` · `Backspace` · `c` · `Y` · `p` · click-copy · click-link · `t` · `l` · `Tab`/`Shift+Tab` · `h`/`?`/`F1` · `q`/`Ctrl+C`
Slide mode remap: `Right`/`Space`/`j` next · `Left`/`b`/`k` prev · `g`/`G` first/last

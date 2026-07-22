# Parity notes extracted from `reference/` source

Read-only analysis of the reference (`mdterm`) event loop, CLI, config, and theme.
Cross-checked against `docs/parity-checklist.md`. All line numbers below are from
`reference/src/viewer.rs` unless another file is named.

The reference binary is `mdterm` (`main.rs:20`), config dir `~/.config/mdterm/`
(`config.rs:42`), NOT `glance`. Our checklist already renames these; kept here for
accuracy of citations.

---

## (a) Complete keybinding table (extracted from viewer.rs)

### Global (any mode) — `handle_event` (L993)
| Key | Action | Source |
|---|---|---|
| `Ctrl+C` | Quit (checked before everything, all modes) | L996-998 |
| `F1` | Toggle Help overlay from ANY mode | `is_help_toggle` L270 |
| `?` | Toggle Help from any mode except text-input modes (Search/Fuzzy), non-Ctrl | L271 |
| `h`/`H` | Toggle Help — only from Normal (open) or Help (close); yields to slide mode + JSON nav; non-Ctrl | L272-276 |
| mouse wheel down | Scroll down 3 lines (Normal); routed to Help/TOC/Fuzzy/slide per mode | L1107-1147 |
| mouse wheel up | Scroll up 3 lines; per-mode routing | L1148-1179 |
| `Resize` | Update cols/rows, recompute cell aspect, `rebuild()` | L1252-1257 |

### Normal mode — `handle_normal` (L1606)
| Key | Action | Source |
|---|---|---|
| `q` | Quit | L1620 |
| `Esc` | Clear search if results exist, else quit | L1621-1627 |
| `t` | Toggle dark/light theme | L1630-1633 |
| `l` | Toggle line numbers (+ toast) | L1636-1644 |
| `m` | **Toggle mouse capture** on/off (+ toast) | L1647-1663 |
| `/` | Enter Search mode | L1666-1671 |
| `n` | Next search match (only if results) | L1672-1675 |
| `N` | Previous search match (only if results) | L1676-1679 |
| `o` | Open TOC overlay (only if headings exist) | L1682-1701 |
| `f` | Open Link Picker (only if links exist) | L1704-1709 |
| `:` | Open Fuzzy heading search (only if headings exist) | L1712-1718 |
| `Y` | Copy full document to clipboard | L1721-1726 |
| `c` | Copy nearest code block at offset | L1729-1736 |
| `[` | Jump to previous heading | L1739-1744 |
| `]` | Jump to next heading | L1745-1750 |
| `Tab` | Next file (only if >1 file) | L1753-1756 |
| `Shift+Tab` (`BackTab`) | Previous file (only if >1 file) | L1757-1764 |
| `Backspace` | Pop nav history — go back after following a link (+ "Back" toast) | L1765-1771 |
| `j`/`↓` | Scroll down 1 line | L1774-1776 |
| `k`/`↑` | Scroll up 1 line | L1777-1779 |
| `Space`/`PgDn` | Scroll down 1 page | L1780-1782 |
| `d` / `Ctrl+d` | Scroll down half page | L1783-1785 |
| `u` / `Ctrl+u` | Scroll up half page | L1786-1788 |
| `b`/`PgUp` | Scroll up 1 page | L1789-1791 |
| `g`/`Home` | Go to top | L1792-1794 |
| `G`/`End` | Go to bottom | L1795-1797 |

### Slide mode — `handle_slide_keys` (L1803), reached via `handle_normal` early-return L1610
| Key | Action | Source |
|---|---|---|
| `q`/`Esc` | Quit | L1806 |
| `Right`/`Space`/`l`/`j`/`↓`/`PgDn` | Next slide | L1807-1816 |
| `Left`/`h`/`k`/`↑`/`PgUp`/`b` | Previous slide | L1817-1824 |
| `g`/`Home` | First slide | L1825-1827 |
| `G`/`End` | Last slide | L1828-1830 |
| `t` | Toggle theme | L1831-1834 |

Note: slide mode has NO search/TOC/link/copy — it is a reduced key set.

### Search mode — `handle_search` (L1840)
| Key | Action | Source |
|---|---|---|
| `Esc` | Cancel search, back to Normal | L1842-1846 |
| `Enter` | Execute search, jump to nearest match, back to Normal | L1847-1855 |
| `Backspace` | Delete char from query | L1856-1858 |
| any char | Append to query buffer (query typed, executed on Enter) | L1859-1861 |

### TOC overlay — `handle_toc` (L1866)
| Key | Action | Source |
|---|---|---|
| `Esc`/`o`/`q` | Close overlay | L1878-1880 |
| `k`/`↑` | Move selection up | L1881-1883 |
| `j`/`↓` | Move selection down | L1884-1886 |
| `PgUp`/`PgDn` | Page selection | L1887-1892 |
| `g`/`Home`, `G`/`End` | First / last entry | L1893-1898 |
| `Enter` | Jump to heading, close | L1899-1903 |

### Link Picker — `handle_link_picker` (L2033)
| Key | Action | Source |
|---|---|---|
| `Esc` | Close | L2046-2048 |
| `k`/`↑`, `j`/`↓` | Move selection | L2049-2054 |
| `PgUp`/`PgDn` | Page | L2055-2060 |
| `g`/`Home`, `G`/`End` | First / last | L2061-2066 |
| `Enter` | Open selected URL via `dispatch_link` (open crate) | L2067-2073 |

### Fuzzy heading — `handle_fuzzy` (L2085)
| Key | Action | Source |
|---|---|---|
| `↓`/`PgDn`/`Ctrl+n` | Move selection down | L2090-2092, 2104 |
| `↑`/`PgUp`/`Ctrl+p` | Move selection up | L2093-2095, 2097 |
| `Esc` | Cancel, back to Normal | L2114-2117 |
| any char | Append to fuzzy query | L2118-2122 |
| `Backspace` | Delete char | L2123-2127 |
| `Enter` | Jump to selected heading | L2128-2137 |

### Help overlay — inline in `handle_event` (L1016-1061)
| Key | Action | Source |
|---|---|---|
| `Esc`/`q` | Close help | L1020-1022 |
| `j`/`↓`, `k`/`↑` | Scroll help 1 line | L1023-1033 |
| `Space`/`PgDn`, `b`/`PgUp` | Page help | L1034-1045 |
| `g`/`Home`, `G`/`End` | Top / bottom of help | L1046-1054 |
| (`h`/`?`/`F1` also close it via `is_help_toggle`) | | L272-273 |

### JSON viewer — `handle_json_keys` (L1264) + `handle_json_diagram_keys` (L1421)
Reached from `handle_normal` L1615 when `json_view.is_some()`. Card-explorer mode:
| Key | Action | Source |
|---|---|---|
| `D` | **Toggle graph/diagram view** ("Graph view" / "Card explorer" toast) | L1266-1280 |
| `j`/`↓`, `k`/`↑` | Move JSON node cursor | L1319-1344 |
| `Enter`/`Space` | Toggle expand/collapse current node | L1345-1356 |
| `l`/`→` | Expand node | L1357-1374 |
| `h`/`←` | Collapse node | L1375-1385 |
| `L` | **Expand all** | L1386-1399 |
| `H` | **Collapse all** | L1400-1411 |
Diagram (graph) mode adds card-graph navigation semantics for `j/k/l/h` (jump to
sibling/child/parent cards) — L1421-1556.

### Mouse (Normal mode) — `handle_event` L1180-1249
| Action | Behavior | Source |
|---|---|---|
| Left click on link | Follow via `dispatch_link` (http/mailto → `open`; `#anchor` → jump; local `.md` → open + push nav history) | L1182-1186, 1940 |
| Left click on code block | Copy block ("Code block copied") | L1191-1197 |
| Left click on heading | Copy heading section ("Copied: <label>", label truncated at 30 chars) | L1198-1211 |
| Left click on list item | Copy list ("List copied") | L1212-1217 |
| Left click on task item | **Toggle task checkbox** (writes file) | L1218-1224 |
| Mouse moved | Set hand cursor via **OSC 22** (`\x1b]22;pointer`/`;default`) when hovering clickable | L1229-1248 |

---

## (b) CLI flags + config keys

### CLI flags (`main.rs`, clap)
| Flag | Short | Purpose | Line |
|---|---|---|---|
| `<files>...` | (positional) | Markdown/JSON file(s) to view | main.rs:24 |
| `--theme` | `-T` | `dark` or `light` | main.rs:27-28 |
| `--width` | `-w` | Display width override (0 = auto) | main.rs:31-32 |
| `--slides` | `-s` | Slide mode | main.rs:35-36 |
| `--follow` | `-f` | **Deprecated / hidden** (watching always on) | main.rs:39-40 |
| `--line-numbers` | `-l` | Line numbers in code blocks | main.rs:43-44 |
| `--export <fmt>` | (none) | Non-interactive export; only `html` supported | main.rs:47-48, 110-120 |
| `--no-color` | (none) | Disable colors (forces plain piped output) | main.rs:51-52 |

There is **NO `--pipe` flag**. Piped/plain output is chosen automatically when stdout
is not a TTY OR `--no-color` is set (`main.rs:125`). `-p` / copy-path is not a CLI flag.

`--export` short flag: none. `--theme` accepts only `light`; anything else → dark
(`main.rs:61-64`) — so effectively only two theme names.

Stdin: read when no files given and stdin is not a TTY; **100 MB cap**, error above
(`main.rs:76-97`). Usage error printed if stdin is a TTY and no files.

### Config keys (`config.toml`, `config.rs`)
| Key | Type | Default | Line |
|---|---|---|---|
| `theme` | String | `"dark"` | config.rs:7-8, 15-17 |
| `line_numbers` | bool | `false` | config.rs:9-10 |
| `width` | usize | `0` | config.rs:11-12 |

Only THREE config keys. CLI overrides config for theme/line_numbers/width
(`main.rs:60-73`). Config path: `dirs::config_dir()/mdterm/config.toml`.

---

## (c) GAPS vs our checklist (concrete additions / corrections)

1. **`m` — toggle mouse capture — MISSING from §5 and from the 25 features.**
   L1647-1663. Toggles crossterm mouse capture so the user can select text with the
   native terminal selection. Emits toasts "Mouse capture OFF — select text freely" /
   "ON — scroll with mouse". Mouse capture is **ON by default** (`viewer.rs:40`,
   `mouse_captured: true` L445). This is a first-class, user-facing binding shown in
   the help overlay ("m — Toggle mouse capture (for text select)", L3637). Add to §5.

2. **JSON viewer keybindings are entirely missing from §5.** Feature #24 exists, but the
   checklist has no keys. The reference JSON viewer is interactive with its own key
   map: `j/k` navigate, `Enter`/`Space` toggle node, `l/h` (or `→/←`) expand/collapse,
   **`L` expand-all, `H` collapse-all, `D` toggle graph/diagram view**. The graph view
   (`D`) with card-graph navigation is a substantial sub-feature not called out
   anywhere in our checklist. Add these keys and note the diagram/graph mode.

3. **Task-checkbox toggle on click — MISSING.** Left-clicking a task list item toggles
   `[ ]`/`[x]` and writes it back to the file (L1218-1224, `toggle_task`). Our
   click-copy row (§12) only mentions copy for code/heading/list; it omits interactive
   task toggling. This is a mutation, not a copy — flag it as its own behavior.

4. **OSC 22 hand-cursor on hover — MISSING.** On mouse-move over a clickable
   (link/copyable) line the reference emits OSC 22 to switch the terminal pointer to a
   hand and back (L1229-1248). Not in our checklist; worth listing under mouse behavior.

5. **`D` diagram toggle also exists for JSON, and slide mode is a *reduced* key set.**
   Our §5 "Slide mode remap" is roughly right but incomplete: slide mode also accepts
   `Right`/`Left`/`Up`/`Down`/`PgDn`/`PgUp` and keeps `t` (theme). It does NOT support
   `/ o : f c Y [ ] Tab l m` etc. Worth stating slide mode disables those.

6. **Fuzzy-search extra nav keys — `Ctrl+n` / `Ctrl+p`** move selection while typing
   (L2090-2095). Our checklist lists `:` fuzzy but not these. Also `PgUp/PgDn` page the
   fuzzy list. Minor, but include for a PTY test matrix.

7. **Help overlay is scrollable and has its own key map** (`j/k/Space/b/g/G/Home/End`,
   L1016-1061) — our checklist treats `h/?/F1` as a single toggle but doesn't note the
   overlay scrolls. Also note: **all three of `h`, `?`, `F1` toggle help**, with `?`/`F1`
   working from any non-text-input mode, and `h` only from Normal/Help (yields to slide
   + JSON which bind `h`). This context-sensitivity (`is_help_toggle`, L261-279) is a
   correctness detail worth a test.

8. **`Esc` in Normal is overloaded**: clears an active search if results exist,
   otherwise quits (L1621-1627). Our §5 lists `Esc` generically; make explicit.

9. **`--pipe` flag does NOT exist in the reference** (feature #25 mentions `--pipe`).
   The reference auto-detects non-TTY. `--pipe` is one of *our* differentiators, not a
   reference feature — fine to keep, but don't attribute it to parity. Similarly `-p`
   copy-path and OSC-52 clipboard are ours; the reference clipboard is external only:
   **macOS `pbcopy`; else `xclip` then `xsel`** (L2402-2419) — **no `wl-copy`, no
   `pbcopy`-on-Linux, no PowerShell, no OSC 52**. Our "Copy everywhere" differentiator
   row is genuinely beyond the reference — confirmed.

10. **Search is NOT live/incremental in the reference.** Query is buffered and only
    executed on `Enter` (`handle_search` L1847-1861). Feature #6 says "regex, highlight,
    n/N" — accurate — but note matches don't update per keystroke; regex is
    auto-detected on execute.

11. **`--follow` / `-f` is a deprecated hidden flag** (watching always on) — our
    checklist folds auto-reload into #16, which is correct; just don't advertise a
    `--follow` flag as needed for parity.

12. **Status-bar hint strings** (not in checklist, useful for pixel parity):
    - Normal: `" / search · o toc · f links · t theme · ? help "` (L2964)
    - Slide: `" ←/→ navigate · t theme "` (L2787)
    - JSON: card vs diagram hints (L2897-2901)
    These are the visible bottom hints; our renderer should match.

13. **Reference has no `--json`/JSON flag** — JSON mode is auto-selected purely by the
    `.json` extension (`main.rs:107`, `is_json`). No CLI toggle.

---

## (d) Confirmed theme names + accent

- **Theme names: exactly two — `dark` and `light`** (`theme.rs:97 dark()`,
  `theme.rs:383 light()`, `name()` returns `"dark"`/`"light"` L677-680). `--theme`
  matches only `"light"`, everything else falls back to dark (`main.rs:61-64`).
- **syntect themes:** dark → `"base16-ocean.dark"` (theme.rs:194); light →
  `"InspiredGitHub"` (theme.rs:480).
- **Dark accent color — DISCREPANCY WITH OUR CHECKLIST.** Our checklist (#20) claims
  the dark accent is `#FF5800`. **The reference dark theme contains NO `#FF5800`.** The
  dark palette is a Catppuccin-Mocha-style scheme: bg `rgb(30,30,46)` = `#1E1E2E`,
  fg `#CDD6F4`, links/h2/table-header `rgb(137,180,250)` = `#89B4FA` (blue), h3/json_path
  `#CBA6F7` (mauve), accents green `#A6E3A1`, yellow `#F9E2AF`, peach `#FAB387`, red
  `#F38BA8`. The closest thing to an "accent" is the blue `#89B4FA` (links, h2, table
  header, json keys). **`#FF5800` appears nowhere in `theme.rs`** — our checklist's dark
  accent value looks wrong (possibly a mdterm-vs-glance rebrand color). Verify and
  correct #20.
- Light theme is Catppuccin-Latte-style: bg `#EFF1F5`, fg `#4C4F69`, blue `#1E66F5`.

---

## Summary of the 25-feature list vs reference

All 25 tracked features map to real reference capabilities EXCEPT the `--pipe` flag,
`-p` copy-path, OSC-52/multi-backend clipboard, and the `#FF5800` accent, which are
our own additions/errors rather than reference parity items. The reference features
NOT surfaced as their own tracked items or §5 keys: **`m` mouse-capture toggle**,
**JSON interactive nav + `D` graph view + `L`/`H` expand/collapse-all**, **click-to-
toggle task checkboxes**, **OSC 22 hover cursor**, and **scrollable context-sensitive
help overlay**.

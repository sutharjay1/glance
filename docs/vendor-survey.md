# Vendor Survey — mdterm → glance

Read-only analysis of the reference project `mdterm` (cloned at `reference/`,
`github.com/bahdotsh/mdterm`, MIT, © 2026 Gokul) ahead of vendoring four modules
into `glance`. glance is a clean-room Rust terminal markdown viewer; the four
modules below are candidates for **lift + adapt** (not clean-room reimplementation).

The goal of this document is to size the adaptation cost of each module — in
particular, how tightly each is bound to mdterm's own type model (`Theme`,
`Style`, `StyledSpan`, `Line`, `LineMeta`, `DocumentInfo`) versus how much is
pure, portable logic. glance will have its own `Line`/`Span`/`Style`/cell model,
so every reference-internal type a module touches is a seam that must be
re-pointed.

Source files inspected: `src/image.rs`, `src/diagram.rs`, `src/json.rs`,
`src/markdown.rs` (math), `src/style.rs`, `src/theme.rs`, `Cargo.toml`, `LICENSE`.

## Reference type model (the seams)

For reference, these are the shared types the modules bind to (`src/style.rs`):

```rust
pub struct Style { fg: Option<Color>, bg: Option<Color>, bold, italic,
                   underline, strikethrough, dim: bool, link_url: Option<String> }
pub struct StyledSpan { text: String, style: Style }
pub struct Line { spans: Vec<StyledSpan>, meta: LineMeta }
pub enum   LineMeta { None, Heading{..}, CodeContent{..}, ListItem{..},
                      TaskItem{..}, SlideBreak, Image{..} }
pub struct DocumentInfo { code_blocks: Vec<CodeBlockContent> }
pub struct CodeBlockContent { language: String, content: String }
```

`Color` throughout is `crossterm::style::Color`. `Theme` (`src/theme.rs`) is a
flat struct of ~55 `Color` fields (`fg`, `code_bg`, `code_border`,
`json_key`, `json_string`, `json_number`, `json_bool`, `json_null`,
`json_bracket`, `json_focus_bg`, `table_border`, `table_header`, `link`,
`overlay_muted`, `heading_separator`, a `h: [Color; 6]` heading-level array,
a `colors` field, etc.).

---

## Summary table

| Module | LOC (approx) | External crates | Reference-internal coupling | Coupling level | Vendor phase |
|---|---|---|---|---|---|
| image (`image.rs`) | ~3299 (~1200 test/helper) | `image`, `base64`, `ureq`, `libc`, `crossterm::Color`, std `mpsc`/threads | **none** — no `crate::` imports, no `Theme`, no `Style`/`Line`; only `crossterm::style::Color` at the API edge | **Low** | Phase 3 |
| diagram / mermaid (`diagram.rs`) | ~1135 (no tests) | `crossterm::Color` only | `crate::style::{Style, StyledSpan}`, `crate::theme::Theme` (4 fields: `fg`, `code_bg`, `code_border`, `h`) | **Medium** | Phase 5 |
| json (`json.rs`) | ~2293 (~70 test) | `serde_json`, `unicode_width`, `crossterm::Color` | `crate::style::{CodeBlockContent, DocumentInfo, Line, LineMeta, Style, StyledSpan}`, `crate::theme::Theme` (~15 fields), **and** `crate::diagram::{Canvas, CardDrawRow}` | **High** | Phase 5 |
| math (`render_math` in `markdown.rs`) | ~215 (fn 1307–~1520) + tests | none | **none** — pure `&str -> String` | **Low** | Phase 5 |

---

## 1. Image — `src/image.rs`

**Purpose.** A complete terminal image-rendering ladder. Detects the best
available graphics protocol (Kitty ID-based upload/placement, Kitty-Unicode
placeholders, iTerm2 inline sequences, Sixel, Terminology, and a Unicode
half-block RGB fallback) and renders raster images into terminal cells. Handles
remote/local fetch on background threads via `std::sync::mpsc` (non-blocking:
`start_fetch()` spawns one thread per URL; `poll_completed()` drains results per
event-loop tick), downscaling, an LRU-ish image cache, pre-rendering of
half-block output off the hot path, and terminal cell-pixel-metric detection
(ioctl `TIOCGWINSZ` via `libc`, with tmux passthrough guards).

**Public entry points.**
- `pub enum ImageProtocol { … }` (line 11) and `pub fn detect_protocol() -> ImageProtocol` (146).
- `pub struct CellMetrics` (260) and `pub fn get_cell_metrics() -> CellMetrics` (276).
- `pub fn kitty_delete_all(&mut impl Write) -> io::Result<()>` (377); `kitty_unicode_delete_all` (473).
- `pub struct ImageCache` (850) — the central object. Key methods:
  `new()` (898), `protocol()` (928), `update_cell_aspect()` (932),
  `has_image()` (956), `has_attempted()` (963), `start_fetch(&mut self, url:&str)->bool` (972),
  `poll_completed()->bool` (997), `has_in_flight()` (1008), `in_flight_count()` (1013),
  `cancel_in_flight()` (1020), `image_dimensions(url)->Option<(u32,u32)>` (1037),
  `display_size(url,max_cols,max_rows)` (1041), `ideal_rows(url,content_width)` (1057),
  `is_ready_to_render(url)` (1072), `queue_all_pre_renders(content_width,bg)` (1161),
  `poll_pre_rendered()->bool` (1191), `render_image_row(...)` (1309),
  `transmit_pending_kitty(&mut self, stdout)` (1354), `reset_kitty_unicode_placements()` (1366),
  `transmit_pending_kitty_unicode(stdout)` (1374),
  `render_iterm2_block(...)` (1517), `render_sixel_block(...)` (1582),
  `render_terminology_block(...)` (1666), `render_block_image(...)` (1726).
- `pub const IMAGE_ROWS: usize = 8` (721).
- `pub fn color_to_rgb(c: crossterm::style::Color) -> (u8,u8,u8)` (1759).

**Internal dependencies.** External crates only: `image` (0.25),
`base64` (0.22), `ureq` (3, in `fetch_image` at line ~2180), `libc` (0.2, cell
metrics), plus `crossterm::style::Color` used purely at the API surface
(`render_*` bg params and `color_to_rgb`). Standard library: `mpsc`, `Arc`,
`Mutex`, threads. **No `use crate::…` at all** (the only `use super::*` is inside
the test module).

**Coupling to reference internals.** Effectively zero. It does not import
`Theme`, `Style`, `StyledSpan`, `Line`, or `DocumentInfo`. The only reference-
adjacent type is `crossterm::style::Color`, and glance already uses crossterm.
`render_image_row`/`render_*_block` emit escape sequences directly to a
`impl Write`; they do not return glance `Line`/`Span` structures, so there is
almost nothing to re-point on the cell model.

**Adaptation notes.** Portable essentially as-is. Drop `image.rs` in, keep the
same `Cargo.toml` deps (`image`, `base64`, `ureq`, `libc`, `unicode-width` for
half-block width). The only integration work is at the caller boundary in
glance's viewer: (a) glance must own the event-loop tick that calls
`poll_completed()`/`poll_pre_rendered()` and the `transmit_pending_*` flush; (b)
glance decides `bg: (u8,u8,u8)` from its own theme and passes it in — this is a
value hand-off, not a type coupling; (c) `detect_protocol`/`get_cell_metrics`
read env + ioctl, so keep the tmux/`TIOCGWINSZ` guards intact. Watch items:
per-URL thread spawning and the `MAX_CONCURRENT_FETCHES` cap are baked in;
`fetch_image` does synchronous `ureq` inside the spawned thread (has a
`catch_unwind` guard so a decode panic can't wedge `in_flight`). No hard seams.

**Approx LOC / dead code.** ~3299 total; the file carries a large inline test
module (`#[cfg(test)]` at 2248) plus small `#[cfg(test)]` test-only helpers
(`insert` at ~1033, `fetch_if_missing` at ~1062). Net non-test rendering logic is
~2000 LOC. No `allow(dead_code)`/`TODO`/`FIXME` markers found. Some protocols
(Sixel, Terminology) are lower-traffic paths glance may choose to gate behind
`detect_protocol` but they are not dead.

---

## 2. Diagram / Mermaid — `src/diagram.rs`

**Purpose.** A self-contained Mermaid flowchart parser + ASCII/box-art renderer.
Parses a `mermaid` code block into an internal `Graph` (nodes with shapes —
rect/rounded/diamond/circle — and labelled edges), assigns nodes to layers
(`assign_layers`, topological), orders within layers to reduce crossings
(`order_within_layers`, barycenter heuristic), and paints onto a character
`Canvas` in either top-down (`render_td`) or left-right (`render_lr`) direction.
Returns styled rows ready to splice into the rendered document.

**Public entry points.**
- `pub fn render_mermaid(code: &str, theme: &Theme) -> Option<(Vec<Vec<StyledSpan>>, usize)>`
  (line 1129) — the sole public function. Returns `(content_rows, content_width)`.

Everything else is private or `pub(crate)`: `Direction`, `Node`, `Edge`,
`Graph`, `NodeShape`, the parser helpers (`parse_mermaid` 54, `parse_node_ref`
107, `parse_line` 194, `parse_arrow` 239), layout (`assign_layers` 279,
`order_within_layers` 360, `node_box_width` 415), and the rendering surface
`Canvas` / `CanvasCell` / `CardDrawRow` (see coupling note — these are shared
with `json.rs`).

**Internal dependencies.** `crate::style::{Style, StyledSpan}`,
`crate::theme::Theme`, and `crossterm::style::Color`. No external crates beyond
crossterm.

**Coupling to reference internals.** Medium. Two seams: (1) it returns
`Vec<Vec<StyledSpan>>`, so the output type must map onto glance's span type;
(2) it reads four `Theme` fields — `theme.fg`, `theme.code_bg`,
`theme.code_border`, `theme.h` (heading-colour array, used for node fills). Both
are shallow. Note that `Canvas`, `CanvasCell`, and `CardDrawRow` are declared
`pub(crate)` here and are **reused by `json.rs`'s graph view** — so diagram.rs is
best treated as a shared "box-art canvas" primitive that json.rs depends on, not
a leaf module.

**Adaptation notes.** The parser and layout engine (~lines 54–465: `Graph`,
`assign_layers`, `order_within_layers`, `node_box_width`) are pure algorithmic
code and port as-is. The seams are the `Canvas`/rendering half: re-point
`StyledSpan`/`Style` to glance's span type and thread glance's theme in place of
the four `Theme` fields (either keep a tiny 4-field colour struct at the API
edge, or pass the four `Color`s explicitly). Because `Canvas`/`CardDrawRow` are
shared with json.rs, vendor diagram.rs and json.rs together and keep the
`Canvas` API stable between them — do not fork two copies of the canvas.

**Approx LOC / dead code.** ~1135 total; **no test module** in this file. No
`allow(dead_code)`/`TODO`/`FIXME` found. The barycenter ordering is the most
intricate part and the highest risk for subtle layout diffs, but it is not dead.

---

## 3. JSON — `src/json.rs`

**Purpose.** A JSON file viewer with three rendering modes: (1) `render` — a
flat, semantically coloured pretty-printer (keys/strings/numbers/booleans/nulls,
indented structure, value-alignment capped at `MAX_ALIGN_WIDTH = 24`); (2)
`render_interactive` — a collapsible **card**-based view with a navigation model
(expand/collapse per path, cursor movement, breadcrumbs) driven by
`JsonViewState`; (3) `render_diagram` — a graph/box-art view that lays JSON
objects out as connected cards using the `Canvas`/`CardDrawRow` primitives from
`diagram.rs` (horizontal scroll via `h_offset`).

**Public entry points.**
- `pub fn render(input:&str, width:usize, theme:&Theme) -> Result<(Vec<Line>, DocumentInfo), String>` (13).
- `pub fn render_interactive(value:&Value, width:usize, theme:&Theme, expanded:&HashSet<String>) -> (Vec<Line>, DocumentInfo, Vec<NavItem>)` (911).
- `pub fn render_diagram(value:&Value, width:usize, theme:&Theme, expanded:&HashSet<String>, cursor_path:Option<&str>, h_offset:usize) -> (Vec<Line>, DocumentInfo, Vec<NavItem>, usize)` (1789).
- `pub struct NavItem` (791) — a navigable target (line + path).
- `pub struct JsonViewState` (809) with `new` (824), `toggle_current` (837),
  `cursor_line` (847), `cursor_path` (851), `move_cursor(delta:i32)` (855),
  `restore_cursor` (868), `expand_all(root:&Value)` (881), `collapse_all` (893),
  `breadcrumb() -> Option<String>` (901).

**Internal dependencies.** External: `serde_json::Value`, `unicode_width`,
`crossterm::style::Color`. Reference-internal:
`crate::style::{CodeBlockContent, DocumentInfo, Line, LineMeta, Style, StyledSpan}`,
`crate::theme::Theme`, and (inside `render_diagram`)
`crate::diagram::{Canvas, CardDrawRow}`.

**Coupling to reference internals.** **High — the heaviest of the four.** It
consumes six `crate::style` types, ~15 `Theme` fields
(`json_key`, `json_string`, `json_number`, `json_bool`, `json_null`,
`json_bracket`, `json_focus_bg`, `code_border`, `table_border`, `table_header`,
`heading_separator`, `overlay_muted`, `link`, `h`, `colors`), and — critically —
reaches into `diagram.rs`'s `pub(crate)` `Canvas`/`CardDrawRow`. It builds glance-
shaped `Line`/`StyledSpan`/`LineMeta` values directly, so every construction site
is a seam. `JsonViewState`/`NavItem` also imply glance owning interactive
navigation state.

**Adaptation notes.** Vendor **after** diagram.rs and in the same pass, because
`render_diagram` depends on the shared canvas. Work required: (a) re-point the
six `crate::style` types to glance equivalents at every span/line construction —
mechanical but pervasive (this is the bulk of the effort); (b) supply the ~15
theme colours from glance's theme (add the `json_*` colour slots to glance's
theme, or pass a small JSON-colour struct); (c) `LineMeta` is used to tag lines
(headings/code) — map to glance's line-meta enum; if glance's meta variants
differ, some tagging may need to be dropped or approximated. The flat `render`
mode is the cheapest to lift; `render_interactive` and `render_diagram` carry the
state model and the canvas dependency and are the expensive parts. `NavItem` /
`JsonViewState` are portable structs once the span types are swapped.

**Approx LOC / dead code.** ~2293 total; small test module (`#[cfg(test)]` at
2224, ~70 LOC). No `allow(dead_code)`/`TODO`/`FIXME` in the module body.

---

## 4. Math (`$...$`) — `render_math` in `src/markdown.rs`

**Purpose.** Converts basic LaTeX math (inline `$…$` via
`Event::InlineMath` and block `$$…$$` via `Event::DisplayMath`) into a Unicode
approximation — a best-effort, table-driven substitution, **not** a layout
engine. Covers Greek lower/upper case, operators (`\sum`, `\int`, `\nabla`, …),
relations, set theory, logic, arrows, misc symbols (`\sqrt`, `\langle`, spacing
macros), superscript/subscript digits and a few letters (`^{2}`→`²`, `_1`→`₁`,
`^n`→`ⁿ`), and a simple `\frac{a}{b}` → `a/b` rewrite.

**Public entry points.**
- `pub fn render_math(latex: &str) -> String` (`markdown.rs` line 1307).

The call sites are in the markdown renderer (`markdown.rs` ~1131 for
`Event::InlineMath`, ~1143 for `Event::DisplayMath`), where the returned string
is wrapped in a `StyledSpan` coloured with `theme.math_fg`. That colouring lives
in the caller, **not** in `render_math`.

**Internal dependencies.** None. Pure `&str -> String` over a static
`replacements` table (`markdown.rs` 1310–1429) plus chained `str::replace`
passes and a small `\frac` loop. No external crates, no `crate::` imports.

**Coupling to reference internals.** None. `render_math` itself touches no
`Theme`/`Style`/`Line`. The only reference-side detail is that its *callers* wrap
the output in a span and pick `theme.math_fg` — glance supplies its own colour at
its own call site.

**Adaptation notes.** Copy `render_math` (and its accompanying inline tests —
`render_math_basic_symbols`, `render_math_fractions`, etc. at ~1760) verbatim
into a glance module (e.g. `math.rs`). glance wires it into its own
`pulldown-cmark` `InlineMath`/`DisplayMath` handling and applies its own math
colour. Zero hard parts. One caveat: the naive `str::replace` ordering means
longer macros must precede prefixes (e.g. `\varepsilon` before `\epsilon`,
`\subseteq` before `\subset`) — the reference table is already ordered for this;
preserve the ordering when copying.

**Approx LOC / dead code.** ~215 LOC for the function (1307–~1520) plus its
tests. No dead code.

---

## Recommended vendor order

1. **Phase 3 — image** (`image.rs`): lift as-is; wire poll/transmit into
   glance's event loop. Lowest risk.
2. **Phase 5 — math** (`render_math`): copy verbatim into `math.rs`. Trivial.
3. **Phase 5 — diagram + json together**: diagram.rs first (it owns the shared
   `Canvas`/`CardDrawRow` primitive that json.rs's `render_diagram` reuses), then
   json.rs on top. Re-point `Style`/`StyledSpan`/`Line`/`LineMeta`/`DocumentInfo`
   and the `json_*`/`h`/`code_*` theme fields to glance's model. Highest
   adaptation cost, driven by json.rs.

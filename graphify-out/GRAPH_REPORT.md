# Graph Report - .  (2026-07-22)

## Corpus Check
- 40 files · ~83,608 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 391 nodes · 852 edges · 16 communities (15 shown, 1 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 13 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Community 0
- Community 1
- Community 2
- Community 3
- Community 4
- Community 5
- Community 6
- Community 7
- Community 8
- Community 9
- Community 10
- Community 13

## God Nodes (most connected - your core abstractions)
1. `Block` - 25 edges
2. `Span` - 21 edges
3. `Theme` - 17 edges
4. `layout_doc()` - 16 edges
5. `parse()` - 16 edges
6. `Line` - 16 edges
7. `ColorDepth` - 16 edges
8. `ViewerState` - 16 edges
9. `tokenize()` - 15 edges
10. `DocLayout` - 14 edges

## Surprising Connections (you probably didn't know these)
- `Parity checklist` --references--> `mdterm (bahdotsh/mdterm)`  [EXTRACTED]
  docs/parity-checklist.md → ROADMAP.md
- `Decision: clean-room + vendor mdterm modules` --rationale_for--> `Images module (src/term/images/)`  [EXTRACTED]
  docs/adr/0002-clean-room-vendor-modules.md → ROADMAP.md
- `Decision: clean-room + vendor mdterm modules` --rationale_for--> `Ports module (src/ports/)`  [EXTRACTED]
  docs/adr/0002-clean-room-vendor-modules.md → ROADMAP.md
- `Decision: Rust over TypeScript` --rationale_for--> `glance (project)`  [EXTRACTED]
  docs/adr/0001-language-rust-over-typescript.md → ROADMAP.md
- `STATUS.md dev log` --references--> `glance (project)`  [EXTRACTED]
  docs/STATUS.md → ROADMAP.md

## Import Cycles
- 1-file cycle: `src/term/input.rs -> src/term/input.rs`

## Hyperedges (group relationships)
- **The render pipeline modules** — roadmap_module_term_layer, roadmap_module_parse, roadmap_module_layout, roadmap_module_paint, roadmap_module_render [EXTRACTED 1.00]
- **The four mdterm weaknesses glance fixes** — roadmap_weakness_slow_first_paint, roadmap_weakness_blocking_stdin, roadmap_weakness_broken_copy, roadmap_weakness_no_callouts [EXTRACTED 1.00]
- **Stack-choice architectural decisions** — docs_adr_0001_language_rust_over_typescript_rust, docs_adr_0002_clean_room_vendor_modules_clean_room, docs_adr_0003_crossterm_input_layer_crossterm, docs_adr_0004_perf_architecture_upfront_perf, docs_adr_0005_tracking_linear_plus_repo_docs_tracking [EXTRACTED 1.00]

## Communities (16 total, 1 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.09
Nodes (51): FnOnce, assert_within(), blockquote_bar_on_every_line(), callout_has_header_and_barred_body(), callout_label(), code_block_is_nowrap(), CodeRef, collect_links() (+43 more)

### Community 1 - "Community 1"
Cohesion: 0.10
Nodes (39): CodeBlockKind, HeadingLevel, Block, Builder, callout_kind(), CalloutKind, code_lang(), detect_callout() (+31 more)

### Community 2 - "Community 2"
Cohesion: 0.11
Nodes (34): Into, all_spans_marked_code(), at(), code_span(), collect(), find(), function_calls_detected(), highlight() (+26 more)

### Community 3 - "Community 3"
Cohesion: 0.07
Nodes (18): String, run(), bg(), dist_sq(), fg(), move_to(), Rgb, Self (+10 more)

### Community 4 - "Community 4"
Cohesion: 0.08
Nodes (38): Decision: Rust over TypeScript, Decision: clean-room + vendor mdterm modules, vendor/NOTICE (MIT attribution), Decision: crossterm for input, hand-write paint/damage, Decision: full perf architecture from Phase 1, graphify knowledge graph, Linear project glance, Decision: track work in Linear + repo markdown (+30 more)

### Community 5 - "Community 5"
Cohesion: 0.13
Nodes (31): dark(), highlighted_code_colors_reach_output(), line(), link_run_uses_single_osc8(), no_hyperlinks_emits_no_osc8(), no_wrap_suppresses_link_suffix_and_osc(), paint(), paint_span() (+23 more)

### Community 6 - "Community 6"
Cohesion: 0.09
Nodes (26): CtEvent, Drop, KeyCode, KeyEvent, KeyModifiers, MouseEvent, Event, Key (+18 more)

### Community 7 - "Community 7"
Cohesion: 0.18
Nodes (16): Action, g_and_shift_g(), heading_jumps(), page_and_half_page(), quit_and_ignore(), resize_relayouts_and_keeps_anchor(), Self, String (+8 more)

### Community 8 - "Community 8"
Cohesion: 0.18
Nodes (12): clipboard(), clipboard_within(), hyperlink(), is_dark(), link_open(), osc52(), parse_bg_four_digit_channels(), parse_bg_response() (+4 more)

### Community 9 - "Community 9"
Cohesion: 0.18
Nodes (12): Error, Args, empty_args(), equals_form_and_booleans(), files_collected(), parse(), parse_ok(), Option (+4 more)

### Community 10 - "Community 10"
Cohesion: 0.20
Nodes (11): PathBuf, Config, config_path(), load(), parse_str(), parses_all_keys(), partial_config_keeps_other_defaults(), Default (+3 more)

## Knowledge Gaps
- **10 isolated node(s):** `Streaming stdin + follow`, `pulldown-cmark`, `syntect`, `insta (snapshot tests)`, `expectrl (PTY tests)` (+5 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **1 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `ColorDepth` connect `Community 3` to `Community 5`, `Community 6`?**
  _High betweenness centrality (0.116) - this node is a cross-community bridge._
- **Why does `Block` connect `Community 1` to `Community 0`, `Community 6`, `Community 7`?**
  _High betweenness centrality (0.065) - this node is a cross-community bridge._
- **Why does `Span` connect `Community 2` to `Community 0`, `Community 5`?**
  _High betweenness centrality (0.063) - this node is a cross-community bridge._
- **What connects `Streaming stdin + follow`, `pulldown-cmark`, `syntect` to the rest of the system?**
  _10 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Community 0` be split into smaller, more focused modules?**
  _Cohesion score 0.09285714285714286 - nodes in this community are weakly interconnected._
- **Should `Community 1` be split into smaller, more focused modules?**
  _Cohesion score 0.10196078431372549 - nodes in this community are weakly interconnected._
- **Should `Community 2` be split into smaller, more focused modules?**
  _Cohesion score 0.11074197120708748 - nodes in this community are weakly interconnected._
# Graph Report - .  (2026-07-22)

## Corpus Check
- Corpus is ~2,319 words - fits in a single context window. You may not need a graph.

## Summary
- 40 nodes · 56 edges · 7 communities (6 shown, 1 thin omitted)
- Extraction: 91% EXTRACTED · 9% INFERRED · 0% AMBIGUOUS · INFERRED: 5 edges (avg confidence: 0.79)
- Token cost: 0 input · 54,922 output

## Community Hubs (Navigation)
- Viewer Core & Performance
- Stack Decisions & Ports
- Project Tracking & Scaffold
- Reference Parity & Differentiators
- Highlighting & Images
- Terminal Input Layer
- PTY Testing

## God Nodes (most connected - your core abstractions)
1. `Phase 1 — viewer core` - 9 edges
2. `mdterm (bahdotsh/mdterm)` - 7 edges
3. `Decision: clean-room + vendor mdterm modules` - 6 edges
4. `Architecture Decision Records (ADRs)` - 5 edges
5. `Phase 3 — highlight + images` - 4 edges
6. `Paint module` - 4 edges
7. `Decision: full perf architecture from Phase 1` - 4 edges
8. `Decision: track work in Linear + repo markdown` - 4 edges
9. `glance (project)` - 3 edges
10. `Phase 4 — differentiators` - 3 edges

## Surprising Connections (you probably didn't know these)
- `Decision: clean-room + vendor mdterm modules` --rationale_for--> `Images module (src/term/images/)`  [EXTRACTED]
  docs/adr/0002-clean-room-vendor-modules.md → ROADMAP.md
- `Decision: clean-room + vendor mdterm modules` --references--> `mdterm (bahdotsh/mdterm)`  [EXTRACTED]
  docs/adr/0002-clean-room-vendor-modules.md → ROADMAP.md
- `Parity checklist` --references--> `mdterm (bahdotsh/mdterm)`  [EXTRACTED]
  docs/parity-checklist.md → ROADMAP.md
- `Decision: clean-room + vendor mdterm modules` --rationale_for--> `Ports module (src/ports/)`  [EXTRACTED]
  docs/adr/0002-clean-room-vendor-modules.md → ROADMAP.md
- `Decision: Rust over TypeScript` --rationale_for--> `glance (project)`  [EXTRACTED]
  docs/adr/0001-language-rust-over-typescript.md → ROADMAP.md

## Hyperedges (group relationships)
- **The render pipeline modules** — roadmap_module_term_layer, roadmap_module_parse, roadmap_module_layout, roadmap_module_paint, roadmap_module_render [EXTRACTED 1.00]
- **The four mdterm weaknesses glance fixes** — roadmap_weakness_slow_first_paint, roadmap_weakness_blocking_stdin, roadmap_weakness_broken_copy, roadmap_weakness_no_callouts [EXTRACTED 1.00]
- **Stack-choice architectural decisions** — docs_adr_0001_language_rust_over_typescript_rust, docs_adr_0002_clean_room_vendor_modules_clean_room, docs_adr_0003_crossterm_input_layer_crossterm, docs_adr_0004_perf_architecture_upfront_perf, docs_adr_0005_tracking_linear_plus_repo_docs_tracking [EXTRACTED 1.00]

## Communities (7 total, 1 thin omitted)

### Community 0 - "Viewer Core & Performance"
Cohesion: 0.33
Nodes (9): Decision: full perf architecture from Phase 1, insta (snapshot tests), pulldown-cmark, First-paint < 80 ms budget, Layout module, Paint module, Parse module, Render module (damage-diff) (+1 more)

### Community 1 - "Stack Decisions & Ports"
Cohesion: 0.29
Nodes (8): Decision: Rust over TypeScript, Decision: clean-room + vendor mdterm modules, vendor/NOTICE (MIT attribution), Architecture Decision Records (ADRs), STATUS.md dev log, glance (project), Ports module (src/ports/), Phase 5 — ports + launch

### Community 2 - "Project Tracking & Scaffold"
Cohesion: 0.29
Nodes (7): graphify knowledge graph, Linear project glance, Decision: track work in Linear + repo markdown, cargo-dist, Phase 0.5 — scaffold, Phase 2 — interactivity, Weakness: broken copy off macOS/X11

### Community 3 - "Reference Parity & Differentiators"
Cohesion: 0.29
Nodes (7): Parity checklist, mdterm (bahdotsh/mdterm), Streaming stdin + follow, Phase 0 — benchmark + vendor survey, Phase 4 — differentiators, Weakness: blocking stdin, Weakness: no GitHub callouts

### Community 4 - "Highlighting & Images"
Cohesion: 0.50
Nodes (4): syntect, Images module (src/term/images/), Phase 3 — highlight + images, Weakness: slow first paint

### Community 5 - "Terminal Input Layer"
Cohesion: 1.00
Nodes (3): Decision: crossterm for input, hand-write paint/damage, crossterm, Term layer module

## Knowledge Gaps
- **10 isolated node(s):** `Streaming stdin + follow`, `pulldown-cmark`, `syntect`, `insta (snapshot tests)`, `expectrl (PTY tests)` (+5 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **1 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Phase 1 — viewer core` connect `Viewer Core & Performance` to `Project Tracking & Scaffold`, `Reference Parity & Differentiators`, `Highlighting & Images`, `Terminal Input Layer`?**
  _High betweenness centrality (0.376) - this node is a cross-community bridge._
- **Why does `mdterm (bahdotsh/mdterm)` connect `Reference Parity & Differentiators` to `Stack Decisions & Ports`, `Project Tracking & Scaffold`, `Highlighting & Images`?**
  _High betweenness centrality (0.188) - this node is a cross-community bridge._
- **Why does `Architecture Decision Records (ADRs)` connect `Stack Decisions & Ports` to `Viewer Core & Performance`, `Project Tracking & Scaffold`, `Terminal Input Layer`?**
  _High betweenness centrality (0.138) - this node is a cross-community bridge._
- **What connects `Streaming stdin + follow`, `pulldown-cmark`, `syntect` to the rest of the system?**
  _10 weakly-connected nodes found - possible documentation gaps or missing edges._
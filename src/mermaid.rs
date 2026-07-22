//! Mermaid flowchart port (Phase 5): render fenced ```` ```mermaid ```` blocks as Unicode box-art.
//!
//! A pragmatic subset — `graph`/`flowchart` with `TD`/`LR` direction, `A[Label]`/`A(Label)`/bare
//! node definitions, and `A --> B` / `A -->|label| B` / `A --- B` edges. Nodes render as bordered
//! boxes stacked vertically with `▼` connectors for consecutive edges; any other edge is listed
//! below. Anything outside the subset (or non-flowchart input) falls back to the raw source, so a
//! diagram never crashes or garbles the view. Pure and unit-tested.

use std::collections::HashMap;

use crate::style::{Line, Role, Span, Style};
use crate::text::width as text_width;

/// A parsed flowchart: node labels in first-seen order + directed edges (with optional labels).
pub struct Graph {
    pub nodes: Vec<String>, // labels, indexed
    pub edges: Vec<(usize, usize, Option<String>)>,
}

/// Parse the simple flowchart subset. Returns `None` for anything unrecognized (→ raw fallback).
pub fn parse(src: &str) -> Option<Graph> {
    let src = src.trim();
    let mut head = src.split_whitespace();
    let kind = head.next()?;
    if kind != "graph" && kind != "flowchart" {
        return None;
    }
    let dir = head.next().unwrap_or("TD");
    // Everything after `graph <dir>` is statements, separated by `;` or newlines.
    let rest = src.strip_prefix(kind).unwrap_or(src).trim_start();
    let rest = rest.strip_prefix(dir).unwrap_or(rest);

    let mut labels: Vec<String> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut edges: Vec<(usize, usize, Option<String>)> = Vec::new();
    let intern = |id: &str,
                  label: &str,
                  labels: &mut Vec<String>,
                  index: &mut HashMap<String, usize>|
     -> usize {
        if let Some(&i) = index.get(id) {
            // Upgrade a bare id to a real label if we later see one.
            if labels[i] == id && label != id {
                labels[i] = label.to_string();
            }
            i
        } else {
            let i = labels.len();
            labels.push(label.to_string());
            index.insert(id.to_string(), i);
            i
        }
    };

    for stmt in rest.split([';', '\n']) {
        let stmt = stmt.trim();
        if stmt.is_empty() {
            continue;
        }
        match parse_edge(stmt) {
            Some(steps) => {
                // A chain `A --> B --> C` yields edges A→B, B→C.
                let mut prev: Option<usize> = None;
                for (id, label, edge_label) in steps {
                    let i = intern(&id, &label, &mut labels, &mut index);
                    if let Some(p) = prev {
                        edges.push((p, i, edge_label));
                    }
                    prev = Some(i);
                }
            }
            None => {
                // A lone node definition.
                if let Some((id, label)) = parse_node(stmt) {
                    intern(&id, &label, &mut labels, &mut index);
                }
            }
        }
    }
    if labels.is_empty() {
        return None;
    }
    Some(Graph {
        nodes: labels,
        edges,
    })
}

/// Parse an edge statement into an ordered chain of `(id, label, incoming_edge_label)`. The first
/// node's edge label is always `None`. Returns `None` if the statement has no `-->`/`---` operator.
fn parse_edge(stmt: &str) -> Option<Vec<(String, String, Option<String>)>> {
    if !stmt.contains("-->") && !stmt.contains("---") {
        return None;
    }
    // Split on the arrow operators, keeping it simple: normalize `---` and `-->` to a marker.
    let normalized = stmt.replace("-->", "\u{1}").replace("---", "\u{1}");
    let mut steps: Vec<(String, String, Option<String>)> = Vec::new();
    for (i, part) in normalized.split('\u{1}').enumerate() {
        let mut part = part.trim();
        let mut edge_label = None;
        // A leading `|label|` (from `A -->|label| B`) belongs to this incoming edge.
        if let Some(rest) = part.strip_prefix('|') {
            if let Some(end) = rest.find('|') {
                edge_label = Some(rest[..end].trim().to_string());
                part = rest[end + 1..].trim();
            }
        }
        // A trailing `-- text` label form (`A -- text --> B`) — strip a leading `text` word run is
        // ambiguous; we ignore that rarer form and just take the node token.
        let (id, label) = parse_node(part)?;
        steps.push((id, label, if i == 0 { None } else { edge_label }));
    }
    (steps.len() >= 2).then_some(steps)
}

/// Parse a node token `A`, `A[Label]`, `A(Label)`, `A{Label}` into `(id, label)`.
fn parse_node(tok: &str) -> Option<(String, String)> {
    let tok = tok.trim();
    if tok.is_empty() {
        return None;
    }
    if let Some(p) = tok.find(['[', '(', '{']) {
        let id = tok[..p].trim();
        if id.is_empty() {
            return None;
        }
        let label = tok[p..]
            .trim_matches(|c| matches!(c, '[' | ']' | '(' | ')' | '{' | '}' | '"' | ' '))
            .to_string();
        Some((
            id.to_string(),
            if label.is_empty() {
                id.to_string()
            } else {
                label
            },
        ))
    } else {
        Some((tok.to_string(), tok.to_string()))
    }
}

/// Render `code` (a mermaid block) to display lines: box-art for the simple subset, else the raw
/// source shown as plain code.
pub fn render(code: &str, width: usize) -> Vec<Line> {
    match parse(code) {
        Some(g) => render_graph(&g, width),
        None => raw_fallback(code),
    }
}

fn raw_fallback(code: &str) -> Vec<Line> {
    code.lines()
        .map(|l| Line {
            spans: vec![Span::new(
                l.to_string(),
                Style::role(Role::Body).with_code_flag(),
            )],
            no_wrap: true,
        })
        .collect()
}

fn render_graph(g: &Graph, _width: usize) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::new();
    for (i, label) in g.nodes.iter().enumerate() {
        out.extend(node_box(label));
        if i + 1 < g.nodes.len() {
            // Draw a connector only when an edge links this node to the next (a linear chain).
            if let Some((_, _, lbl)) = g.edges.iter().find(|(f, t, _)| *f == i && *t == i + 1) {
                out.push(accent_line("  │"));
                let arrow = match lbl {
                    Some(l) if !l.is_empty() => format!("  ▼ {l}"),
                    _ => "  ▼".to_string(),
                };
                out.push(accent_line(&arrow));
            }
        }
    }
    // Any edge not represented by the linear stack (branches / back-edges) is listed explicitly.
    let extra: Vec<&(usize, usize, Option<String>)> =
        g.edges.iter().filter(|(f, t, _)| *t != f + 1).collect();
    if !extra.is_empty() {
        out.push(Line::default());
        for (f, t, lbl) in extra {
            let mid = match lbl {
                Some(l) if !l.is_empty() => format!(" ──[{l}]▶ "),
                _ => " ──▶ ".to_string(),
            };
            out.push(Line {
                spans: vec![
                    Span::new(g.nodes[*f].clone(), Style::role(Role::Body)),
                    Span::new(mid, Style::role(Role::Accent)),
                    Span::new(g.nodes[*t].clone(), Style::role(Role::Body)),
                ],
                no_wrap: true,
            });
        }
    }
    out
}

/// A three-line bordered box for `label`.
fn node_box(label: &str) -> Vec<Line> {
    let w = text_width(label) + 2; // one space of padding each side
    let border = |left: &str, fill: char, right: &str| Line {
        spans: vec![Span::new(
            format!("{left}{}{right}", fill.to_string().repeat(w)),
            Style::role(Role::Accent),
        )],
        no_wrap: true,
    };
    let mid = Line {
        spans: vec![
            Span::new("│ ", Style::role(Role::Accent)),
            Span::new(label.to_string(), Style::role(Role::Body)),
            Span::new(" │", Style::role(Role::Accent)),
        ],
        no_wrap: true,
    };
    vec![border("┌", '─', "┐"), mid, border("└", '─', "┘")]
}

fn accent_line(s: &str) -> Line {
    Line {
        spans: vec![Span::new(s.to_string(), Style::role(Role::Accent))],
        no_wrap: true,
    }
}

/// Small helper to mark a span as code-styled without importing layout internals.
trait CodeFlag {
    fn with_code_flag(self) -> Self;
}
impl CodeFlag for Style {
    fn with_code_flag(mut self) -> Self {
        self.code = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nodes_and_edges() {
        let g = parse("graph TD; A[Start] --> B[End]").unwrap();
        assert_eq!(g.nodes, vec!["Start", "End"]);
        assert_eq!(g.edges, vec![(0, 1, None)]);
    }

    #[test]
    fn parses_chain_and_edge_labels() {
        let g = parse("flowchart LR\n A --> B\n B -->|yes| C").unwrap();
        assert_eq!(g.nodes, vec!["A", "B", "C"]);
        assert_eq!(g.edges[0], (0, 1, None));
        assert_eq!(g.edges[1], (1, 2, Some("yes".to_string())));
    }

    #[test]
    fn non_flowchart_is_none() {
        assert!(parse("sequenceDiagram\n A->>B: hi").is_none());
        assert!(parse("just some text").is_none());
    }

    #[test]
    fn render_draws_boxes_and_arrow() {
        let lines = render("graph TD; A[Start] --> B[End]", 80);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()))
            .collect::<Vec<_>>()
            .join("");
        assert!(text.contains("Start"));
        assert!(text.contains("End"));
        assert!(text.contains('┌')); // a box border
        assert!(text.contains('▼')); // a connector arrow
    }

    #[test]
    fn unknown_syntax_falls_back_to_raw() {
        let src = "sequenceDiagram\n Alice->>Bob: Hi";
        let lines = render(src, 80);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("sequenceDiagram"));
        assert!(text.contains("Alice->>Bob: Hi"));
    }

    #[test]
    fn branching_edges_are_listed() {
        // A→B and A→C: A→B is the linear connector, A→C is listed below.
        let lines = render("graph TD\n A[A] --> B[B]\n A --> C[C]", 80);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()))
            .collect::<Vec<_>>()
            .join("");
        assert!(text.contains("──▶")); // the non-consecutive edge rendered as an inline arrow
    }
}

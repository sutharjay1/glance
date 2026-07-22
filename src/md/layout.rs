//! Layout: a [`Block`] tree × a width → wrapped [`Line`]s.
//!
//! This is the core of the perf architecture (ADR 0004). Word-wrapping uses the `text::width`
//! ASCII fast path with incremental width accumulation (never re-measuring joined strings).
//! Wrapped lines carry a `first` prefix and an equal-width `cont` prefix so list markers and
//! quote bars never repeat on continuation lines (plan §8). Layout produces semantic styles
//! only; color is applied later by `paint`.
//!
//! Not yet handled: tables (parsed-but-skipped upstream), the `(block,width)` cache, and
//! viewport-first slicing — those are the next layout iteration.

use crate::md::parse::{Block, CalloutKind, Inline, Item};
use crate::style::{Line, Role, Span, Style};
use crate::text::width;

/// Lay out a slice of blocks at content width `w`, separated by blank lines.
pub fn layout_blocks(blocks: &[Block], w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    for (i, b) in blocks.iter().enumerate() {
        if i > 0 {
            out.push(Line::default());
        }
        out.extend(layout_block(b, w));
    }
    out
}

/// Lay out one block at content width `w`.
pub fn layout_block(block: &Block, w: usize) -> Vec<Line> {
    match block {
        Block::Heading { level: _, inlines } => {
            let style = Style {
                bold: true,
                role: Role::Heading,
                ..Default::default()
            };
            wrap_inlines(inlines, style, w, &empty(), &empty())
        }
        Block::Paragraph(inlines) => wrap_inlines(inlines, Style::default(), w, &empty(), &empty()),
        Block::Code { code, .. } => code
            .lines()
            .map(|l| Line {
                spans: vec![Span::new(
                    truncate(l, w),
                    Style::role(Role::Body).with_code(),
                )],
                no_wrap: true,
            })
            .collect(),
        Block::ThematicBreak => vec![Line {
            spans: vec![Span::new("─".repeat(w.max(1)), Style::role(Role::Rule))],
            no_wrap: true,
        }],
        Block::List {
            ordered,
            start,
            items,
        } => layout_list(*ordered, *start, items, w),
        Block::BlockQuote(blocks) => {
            let bar = Span::new("▏ ", Style::role(Role::Accent));
            prefixed(
                layout_blocks(blocks, w.saturating_sub(width(&bar.text))),
                &bar,
                &bar,
            )
        }
        Block::Callout { kind, blocks } => layout_callout(*kind, blocks, w),
    }
}

impl Style {
    fn with_code(mut self) -> Self {
        self.code = true;
        self
    }
}

fn empty() -> Span {
    Span::plain("")
}

fn layout_list(ordered: bool, start: u64, items: &[Item], w: usize) -> Vec<Line> {
    let mut out = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        let marker = item_marker(ordered, start, idx, item.task);
        let mw = width(&marker);
        let inner = layout_blocks(&item.blocks, w.saturating_sub(mw));
        let first = Span::new(marker, Style::role(Role::Marker));
        let cont = Span::plain(" ".repeat(mw));
        out.extend(prefixed(inner, &first, &cont));
    }
    out
}

fn item_marker(ordered: bool, start: u64, idx: usize, task: Option<bool>) -> String {
    match task {
        Some(true) => "✓ ".to_string(),
        Some(false) => "○ ".to_string(),
        None if ordered => format!("{}. ", start + idx as u64),
        None => "• ".to_string(),
    }
}

fn layout_callout(kind: CalloutKind, blocks: &[Block], w: usize) -> Vec<Line> {
    let bar = Span::new("▎ ", Style::role(Role::Accent));
    let bw = width(&bar.text);
    let (icon, name) = callout_label(kind);
    let header = Line {
        spans: vec![
            bar.clone(),
            Span::new(
                format!("{icon} {name}"),
                Style {
                    bold: true,
                    role: Role::Accent,
                    ..Default::default()
                },
            ),
        ],
        no_wrap: false,
    };
    let body = prefixed(layout_blocks(blocks, w.saturating_sub(bw)), &bar, &bar);
    let mut out = vec![header];
    out.extend(body);
    out
}

fn callout_label(kind: CalloutKind) -> (&'static str, &'static str) {
    match kind {
        CalloutKind::Note => ("ⓘ", "NOTE"),
        CalloutKind::Tip => ("✎", "TIP"),
        CalloutKind::Important => ("‼", "IMPORTANT"),
        CalloutKind::Warning => ("⚠", "WARNING"),
        CalloutKind::Caution => ("⛔", "CAUTION"),
    }
}

/// Prepend `first` to the first line and `cont` to the rest (skipping empty prefixes).
fn prefixed(lines: Vec<Line>, first: &Span, cont: &Span) -> Vec<Line> {
    lines
        .into_iter()
        .enumerate()
        .map(|(i, mut line)| {
            let p = if i == 0 { first } else { cont };
            if !p.text.is_empty() {
                line.spans.insert(0, p.clone());
            }
            line
        })
        .collect()
}

// --- Inline wrapping ------------------------------------------------------

enum Tok {
    Word(Span),
    HardBreak,
}

/// Flatten inline content to a styled token stream, then greedy-wrap it to `w` cells, applying
/// `first`/`cont` prefixes. Continuation lines reserve the prefix width so nothing overflows.
fn wrap_inlines(inlines: &[Inline], base: Style, w: usize, first: &Span, cont: &Span) -> Vec<Line> {
    let mut toks = Vec::new();
    flatten(inlines, base, None, &mut toks);

    let avail = w.saturating_sub(width(&first.text)).max(1);
    let mut lines: Vec<Vec<Span>> = Vec::new();
    let mut cur: Vec<Span> = Vec::new();
    let mut cur_w = 0usize;

    for tok in toks {
        match tok {
            Tok::HardBreak => {
                lines.push(std::mem::take(&mut cur));
                cur_w = 0;
            }
            Tok::Word(sp) => {
                let ww = width(&sp.text);
                if cur_w == 0 {
                    cur.push(sp);
                    cur_w = ww;
                } else if cur_w + 1 + ww <= avail {
                    cur.push(Span::plain(" "));
                    cur.push(sp);
                    cur_w += 1 + ww;
                } else {
                    lines.push(std::mem::take(&mut cur));
                    cur.push(sp);
                    cur_w = ww;
                }
            }
        }
    }
    if !cur.is_empty() || lines.is_empty() {
        lines.push(cur);
    }

    lines
        .into_iter()
        .enumerate()
        .map(|(i, mut spans)| {
            let p = if i == 0 { first } else { cont };
            if !p.text.is_empty() {
                spans.insert(0, p.clone());
            }
            Line {
                spans,
                no_wrap: false,
            }
        })
        .collect()
}

fn flatten(inlines: &[Inline], style: Style, href: Option<&str>, out: &mut Vec<Tok>) {
    for inl in inlines {
        match inl {
            Inline::Text(s) => push_words(s, style, href, out),
            Inline::Code(s) => push_words(s, style.with_code(), href, out),
            Inline::Emph(v) => flatten(v, with(style, |s| s.italic = true), href, out),
            Inline::Strong(v) => flatten(v, with(style, |s| s.bold = true), href, out),
            Inline::Strike(v) => flatten(v, with(style, |s| s.strike = true), href, out),
            Inline::Link { url, inlines } => flatten(inlines, style, Some(url), out),
            Inline::Image { alt, .. } => {
                push_words(&format!("[{alt}]"), Style::role(Role::Dim), href, out)
            }
            Inline::SoftBreak => {} // words are already separated; nothing to emit
            Inline::HardBreak => out.push(Tok::HardBreak),
        }
    }
}

fn with(mut style: Style, f: impl FnOnce(&mut Style)) -> Style {
    f(&mut style);
    style
}

fn push_words(s: &str, style: Style, href: Option<&str>, out: &mut Vec<Tok>) {
    for word in s.split_whitespace() {
        out.push(Tok::Word(Span {
            text: word.to_string(),
            style,
            href: href.map(str::to_string),
        }));
    }
}

/// Truncate `s` to at most `w` display cells (for no-wrap code lines).
fn truncate(s: &str, w: usize) -> String {
    if width(s) <= w {
        return s.to_string();
    }
    let mut out = String::new();
    let mut acc = 0;
    for ch in s.chars() {
        let cw = width(ch.encode_utf8(&mut [0u8; 4]));
        if acc + cw > w {
            break;
        }
        out.push(ch);
        acc += cw;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::md::parse::parse;

    fn layout_doc(md: &str, w: usize) -> Vec<Line> {
        layout_blocks(&parse(md).blocks, w)
    }

    /// No line may exceed the width (the core wrap invariant).
    fn assert_within(lines: &[Line], w: usize) {
        for l in lines {
            assert!(
                l.width() <= w,
                "line {:?} width {} > {}",
                l.plain_text(),
                l.width(),
                w
            );
        }
    }

    #[test]
    fn paragraph_wraps_within_width() {
        let md = "the quick brown fox jumps over the lazy dog again and again and again";
        for w in [10, 20, 40, 80] {
            let lines = layout_doc(md, w);
            assert_within(&lines, w);
            assert!(lines.len() > 1 || w >= 80);
        }
    }

    #[test]
    fn wrapped_text_roundtrips() {
        let md = "alpha beta gamma delta epsilon zeta eta theta";
        let lines = layout_doc(md, 12);
        let joined: String = lines
            .iter()
            .map(|l| l.plain_text())
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(
            joined.split_whitespace().collect::<Vec<_>>(),
            md.split_whitespace().collect::<Vec<_>>()
        );
    }

    #[test]
    fn emphasis_style_is_carried() {
        let lines = layout_doc("plain **bold** and *italic*", 80);
        let spans = &lines[0].spans;
        assert!(spans.iter().any(|s| s.text == "bold" && s.style.bold));
        assert!(spans.iter().any(|s| s.text == "italic" && s.style.italic));
    }

    #[test]
    fn link_href_carried_on_words() {
        let lines = layout_doc("see [the docs](https://x.io) now", 80);
        assert!(lines[0]
            .spans
            .iter()
            .any(|s| s.text == "docs" && s.href.as_deref() == Some("https://x.io")));
    }

    #[test]
    fn list_marker_hanging_indent() {
        // A long item wraps; the marker is on line 0, continuation lines are indented, not re-marked.
        let lines = layout_doc("- one two three four five six seven eight nine ten", 16);
        assert!(lines[0].plain_text().starts_with("• "));
        assert!(lines.len() > 1);
        // continuation line begins with marker-width spaces, no bullet
        assert!(lines[1].plain_text().starts_with("  "));
        assert!(!lines[1].plain_text().trim_start().starts_with('•'));
        assert_within(&lines, 16);
    }

    #[test]
    fn ordered_list_numbers() {
        let lines = layout_doc("3. c\n4. d", 40);
        assert!(lines.iter().any(|l| l.plain_text().starts_with("3. ")));
        assert!(lines.iter().any(|l| l.plain_text().starts_with("4. ")));
    }

    #[test]
    fn task_list_markers() {
        let lines = layout_doc("- [x] done\n- [ ] todo", 40);
        assert!(lines.iter().any(|l| l.plain_text().starts_with("✓ ")));
        assert!(lines.iter().any(|l| l.plain_text().starts_with("○ ")));
    }

    #[test]
    fn code_block_is_nowrap() {
        let lines = layout_doc("```\nfn main() {}\n```", 80);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].no_wrap);
        assert!(lines[0].spans[0].style.code);
        assert_eq!(lines[0].plain_text(), "fn main() {}");
    }

    #[test]
    fn blockquote_bar_on_every_line() {
        let lines = layout_doc("> one two three four five six seven eight", 20);
        assert!(lines.iter().all(|l| l.plain_text().starts_with("▏")));
        assert_within(&lines, 20);
    }

    #[test]
    fn callout_has_header_and_barred_body() {
        let lines = layout_doc("> [!WARNING]\n> be careful here", 30);
        assert!(lines[0].plain_text().contains("WARNING"));
        assert!(lines[1].plain_text().starts_with("▎"));
        assert_within(&lines, 30);
    }

    #[test]
    fn thematic_break_fills_width() {
        let lines = layout_doc("---", 10);
        assert_eq!(lines[0].plain_text(), "──────────");
        assert!(lines[0].no_wrap);
    }

    #[test]
    fn reference_fixture_lays_out_within_widths() {
        let doc = parse(include_str!("../../tests/fixtures/mdterm-test.md"));
        for w in [44, 80, 120] {
            let lines = layout_blocks(&doc.blocks, w);
            assert!(!lines.is_empty());
            assert_within(&lines, w);
        }
    }
}

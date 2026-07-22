//! Layout: a [`Block`] tree × a width → wrapped [`Line`]s.
//!
//! This is the core of the perf architecture (ADR 0004). Word-wrapping uses the `text::width`
//! ASCII fast path with incremental width accumulation (never re-measuring joined strings).
//! Wrapped lines carry a `first` prefix and an equal-width `cont` prefix so list markers and
//! quote bars never repeat on continuation lines (plan §8). Layout produces semantic styles
//! only; color is applied later by `paint`.
//!
//! [`layout_document`] produces a [`DocLayout`] — the lines plus heading/code/link indices the
//! event loop needs. Deferred until a perf gate demands them: tables (parsed-but-skipped
//! upstream), and the `(block,width)` cache + viewport-first background layout (a full-doc
//! layout is already cheap; scrolling just slices `lines`, so no cache is needed to scroll).

use std::collections::HashMap;

use crate::md::highlight;
use crate::md::parse::{Block, CalloutKind, Inline, Item};
use crate::style::{Line, Role, Span, Style};
use crate::text::width;

/// Images the background worker has already fetched + rendered, keyed by image ordinal (the index
/// into [`DocLayout::images`]). Passed back into layout so a resolved placeholder expands to the
/// image's full rendered rows.
pub type ResolvedImages = HashMap<usize, Vec<Line>>;

/// Lay out a slice of blocks at content width `w`, separated by blank lines.
pub fn layout_blocks(blocks: &[Block], w: usize, line_numbers: bool) -> Vec<Line> {
    let mut out = Vec::new();
    for (i, b) in blocks.iter().enumerate() {
        if i > 0 {
            out.push(Line::default());
        }
        out.extend(layout_block(b, w, line_numbers));
    }
    out
}

/// A heading entry: depth, plain text, and the line it lands on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub depth: u8,
    pub text: String,
    pub line: usize,
}

/// A code block's span in the laid-out document, with its source and language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeRef {
    pub start: usize,
    pub end: usize,
    pub content: String,
    pub lang: String,
}

/// A link occurrence: display text, target, and the line it appears on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkRef {
    pub text: String,
    pub url: String,
    pub line: usize,
}

/// A standalone image (a paragraph that is just `![alt](url)`). Rendered as a placeholder line
/// range `[start, end)` until the background worker fetches, decodes, and patches it in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRef {
    pub start: usize,
    pub end: usize,
    pub url: String,
    pub alt: String,
}

/// A fully laid-out document: the lines to render plus indices for search, navigation, and
/// click hit-testing. Built once per `(document, width)`; the event loop slices `lines` for the
/// viewport and consults the indices for `[`/`]`, `/`, `o`, `f`, and click handling.
#[derive(Debug, Clone, Default)]
pub struct DocLayout {
    pub lines: Vec<Line>,
    /// Plain text per line (parallel to `lines`), for search and hit-testing.
    pub text: Vec<String>,
    pub headings: Vec<Heading>,
    pub code_blocks: Vec<CodeRef>,
    pub links: Vec<LinkRef>,
    /// Standalone images, in document order — the background image worker fetches and patches these.
    pub images: Vec<ImageRef>,
}

impl DocLayout {
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

/// Lay out a whole document, recording indices alongside the lines. Top-level headings and code
/// blocks are indexed with their line positions; links are collected from every line's spans (so
/// nested links are found too).
pub fn layout_document(blocks: &[Block], w: usize, line_numbers: bool) -> DocLayout {
    layout_document_with(blocks, w, line_numbers, &ResolvedImages::new())
}

/// Like [`layout_document`], but any standalone image whose ordinal is present in `resolved`
/// expands to its already-rendered rows instead of a one-line placeholder — so the background image
/// worker's result is applied by re-laying-out (the image's height is unknown until decoded, so it
/// can't be patched in place like a code block).
pub fn layout_document_with(
    blocks: &[Block],
    w: usize,
    line_numbers: bool,
    resolved: &ResolvedImages,
) -> DocLayout {
    let mut lines: Vec<Line> = Vec::new();
    let mut headings = Vec::new();
    let mut code_blocks = Vec::new();
    let mut images = Vec::new();
    for (i, b) in blocks.iter().enumerate() {
        if i > 0 {
            lines.push(Line::default());
        }
        let start = lines.len();
        // A paragraph that is just `![alt](url)` becomes an image: its already-rendered rows if the
        // worker has resolved it, else a one-line placeholder + an ImageRef to resolve later.
        let bl = match b {
            Block::Paragraph(inls) if standalone_image(inls).is_some() => {
                let (url, alt) = standalone_image(inls).unwrap();
                let ord = images.len();
                let rendered = match resolved.get(&ord) {
                    Some(r) if !r.is_empty() => r.clone(),
                    _ => image_placeholder(&alt, &url, w),
                };
                images.push(ImageRef {
                    start,
                    end: start + rendered.len(),
                    url,
                    alt,
                });
                rendered
            }
            _ => layout_block(b, w, line_numbers),
        };
        match b {
            Block::Heading { level, inlines } => headings.push(Heading {
                depth: *level,
                text: inline_plain(inlines),
                line: start,
            }),
            Block::Code { code, lang } => code_blocks.push(CodeRef {
                start,
                end: start + bl.len(),
                content: code.clone(),
                lang: lang.clone().unwrap_or_default(),
            }),
            _ => {}
        }
        lines.extend(bl);
    }
    let links = collect_links(&lines);
    let text = lines.iter().map(Line::plain_text).collect();
    DocLayout {
        lines,
        text,
        headings,
        code_blocks,
        links,
        images,
    }
}

/// If `inlines` is a single image with only surrounding whitespace/breaks, return `(url, alt)`.
/// This is the "standalone image" case worth rendering as a real picture (vs. an inline icon).
fn standalone_image(inlines: &[Inline]) -> Option<(String, String)> {
    let mut found: Option<(String, String)> = None;
    for inl in inlines {
        match inl {
            Inline::Image { url, alt } => {
                if found.is_some() {
                    return None; // more than one image → treat as ordinary text flow
                }
                found = Some((url.clone(), alt.clone()));
            }
            Inline::Text(t) if t.trim().is_empty() => {}
            Inline::SoftBreak | Inline::HardBreak => {}
            _ => return None, // any real text/other inline → not standalone
        }
    }
    found
}

/// One-line dim placeholder shown until an image is fetched and rendered, truncated to width.
fn image_placeholder(alt: &str, url: &str, w: usize) -> Vec<Line> {
    let label = if alt.trim().is_empty() {
        format!("⌛ image: {url}")
    } else {
        format!("⌛ image: {alt} ({url})")
    };
    let spans = truncate_spans(vec![Span::new(label, Style::role(Role::Dim))], w);
    vec![Line {
        spans,
        no_wrap: true,
    }]
}

/// Concatenate the plain text of an inline sequence.
fn inline_plain(inlines: &[Inline]) -> String {
    let mut out = String::new();
    fn go(out: &mut String, inlines: &[Inline]) {
        for i in inlines {
            match i {
                Inline::Text(s) | Inline::Code(s) => out.push_str(s),
                Inline::Emph(v) | Inline::Strong(v) | Inline::Strike(v) => go(out, v),
                Inline::Link { inlines, .. } => go(out, inlines),
                Inline::Image { alt, .. } => out.push_str(alt),
                Inline::SoftBreak | Inline::HardBreak => out.push(' '),
            }
        }
    }
    go(&mut out, inlines);
    out
}

/// Collect link occurrences from laid-out lines, merging consecutive same-href spans into one.
fn collect_links(lines: &[Line]) -> Vec<LinkRef> {
    let mut links = Vec::new();
    for (ln, line) in lines.iter().enumerate() {
        let mut i = 0;
        while i < line.spans.len() {
            let Some(url) = line.spans[i].href.clone() else {
                i += 1;
                continue;
            };
            let mut text = String::new();
            while i < line.spans.len() && line.spans[i].href.as_deref() == Some(url.as_str()) {
                text.push_str(&line.spans[i].text);
                i += 1;
            }
            links.push(LinkRef {
                text: text.trim().to_string(),
                url,
                line: ln,
            });
        }
    }
    links
}

/// Lay out one block at content width `w`. `line_numbers` adds a gutter to code blocks.
pub fn layout_block(block: &Block, w: usize, line_numbers: bool) -> Vec<Line> {
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
        Block::Code { code, lang } => {
            layout_code(code, lang.as_deref().unwrap_or(""), w, line_numbers)
        }
        // Ports (JSON, …) supply their own colored lines; show them verbatim.
        Block::Prerendered(lines) => lines.clone(),
        Block::ThematicBreak => vec![Line {
            spans: vec![Span::new("─".repeat(w.max(1)), Style::role(Role::Rule))],
            no_wrap: true,
        }],
        Block::List {
            ordered,
            start,
            items,
        } => layout_list(*ordered, *start, items, w, line_numbers),
        Block::BlockQuote(blocks) => {
            let bar = Span::new("▏ ", Style::role(Role::Accent));
            prefixed(
                layout_blocks(blocks, w.saturating_sub(width(&bar.text)), line_numbers),
                &bar,
                &bar,
            )
        }
        Block::Callout { kind, blocks } => layout_callout(*kind, blocks, w, line_numbers),
    }
}

/// Lay out a code block: highlighted (or plain) lines, optionally with a right-aligned line-number
/// gutter (`  1 │ `) whose width is reserved from the code area.
fn layout_code(code: &str, lang: &str, w: usize, line_numbers: bool) -> Vec<Line> {
    let rows: Vec<Vec<Span>> = if highlight::supported(lang) {
        highlight::highlight(code, lang)
    } else {
        code.lines()
            .map(|l| {
                vec![Span::new(
                    l.to_string(),
                    Style::role(Role::Body).with_code(),
                )]
            })
            .collect()
    };
    render_code_rows(rows, w, line_numbers)
}

/// Re-render a code block from precomputed per-line spans (e.g. the background syntect
/// highlighter's output) with the same gutter + width handling as [`layout_code`]. Because the
/// row count equals the source-line count either way, the resulting line count matches
/// `layout_code`'s — so a highlighted block can be patched in over the micro-tokenizer's version
/// without shifting any downstream line indices.
pub fn layout_code_with(rows: Vec<Vec<Span>>, w: usize, line_numbers: bool) -> Vec<Line> {
    render_code_rows(rows, w, line_numbers)
}

/// Shared code-block renderer: one display line per source line (truncated, `no_wrap`), with an
/// optional right-aligned line-number gutter. Every span is marked `code` so it paints against
/// the code palette regardless of which highlighter produced it.
fn render_code_rows(rows: Vec<Vec<Span>>, w: usize, line_numbers: bool) -> Vec<Line> {
    let gutter_w = if line_numbers {
        gutter_width(rows.len())
    } else {
        0
    };
    let content_w = w.saturating_sub(gutter_w);
    rows.into_iter()
        .enumerate()
        .map(|(i, spans)| {
            let spans: Vec<Span> = spans
                .into_iter()
                .map(|mut s| {
                    s.style.code = true;
                    s
                })
                .collect();
            let mut out = truncate_spans(spans, content_w);
            if line_numbers {
                let digits = gutter_w.saturating_sub(3);
                out.insert(
                    0,
                    Span::new(format!("{:>digits$} │ ", i + 1), Style::role(Role::Dim)),
                );
            }
            Line {
                spans: out,
                no_wrap: true,
            }
        })
        .collect()
}

/// Width of the line-number gutter for a code block of `rows` lines (digits + `" │ "`).
fn gutter_width(rows: usize) -> usize {
    rows.max(1).to_string().len() + 3
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

fn layout_list(
    ordered: bool,
    start: u64,
    items: &[Item],
    w: usize,
    line_numbers: bool,
) -> Vec<Line> {
    let mut out = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        let marker = item_marker(ordered, start, idx, item.task);
        let mw = width(&marker);
        let inner = layout_blocks(&item.blocks, w.saturating_sub(mw), line_numbers);
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

fn layout_callout(kind: CalloutKind, blocks: &[Block], w: usize, line_numbers: bool) -> Vec<Line> {
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
    let body = prefixed(
        layout_blocks(blocks, w.saturating_sub(bw), line_numbers),
        &bar,
        &bar,
    );
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
    /// A whitespace boundary between words. Consecutive spaces collapse; a space at a line
    /// start or wrap point is dropped. Preserving these (vs. re-joining split words) keeps
    /// punctuation glued to inline elements — `code`, not `code ,`.
    Space,
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

    let mut pending_space = false;
    for tok in toks {
        match tok {
            Tok::Space => pending_space = true,
            Tok::HardBreak => {
                lines.push(std::mem::take(&mut cur));
                cur_w = 0;
                pending_space = false;
            }
            Tok::Word(sp) => {
                let ww = width(&sp.text);
                let need_space = pending_space && cur_w > 0;
                pending_space = false;
                let add = if need_space { 1 + ww } else { ww };
                if cur_w == 0 {
                    cur.push(sp);
                    cur_w = ww;
                } else if cur_w + add <= avail {
                    if need_space {
                        // A space *between two words of the same link* carries that link's href,
                        // so paint keeps the whole link as one OSC 8 run; boundary spaces don't.
                        let prev_href = cur.last().and_then(|s| s.href.clone());
                        let sp_href = if prev_href == sp.href {
                            sp.href.clone()
                        } else {
                            None
                        };
                        cur.push(Span {
                            text: " ".to_string(),
                            style: Style::default(),
                            href: sp_href,
                        });
                    }
                    cur.push(sp);
                    cur_w += add;
                } else {
                    lines.push(std::mem::take(&mut cur)); // the space becomes the line break
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
            Inline::Text(s) => emit_text(s, style, href, out),
            Inline::Code(s) => emit_text(s, style.with_code(), href, out),
            Inline::Emph(v) => flatten(v, with(style, |s| s.italic = true), href, out),
            Inline::Strong(v) => flatten(v, with(style, |s| s.bold = true), href, out),
            Inline::Strike(v) => flatten(v, with(style, |s| s.strike = true), href, out),
            Inline::Link { url, inlines } => flatten(inlines, style, Some(url), out),
            Inline::Image { alt, .. } => out.push(Tok::Word(Span {
                text: format!("[{alt}]"),
                style: Style::role(Role::Dim),
                href: href.map(str::to_string),
            })),
            Inline::SoftBreak => out.push(Tok::Space),
            Inline::HardBreak => out.push(Tok::HardBreak),
        }
    }
}

fn with(mut style: Style, f: impl FnOnce(&mut Style)) -> Style {
    f(&mut style);
    style
}

/// Emit a styled text run as `Word`/`Space` tokens, preserving leading/trailing/inter-word
/// whitespace boundaries so words glue or separate exactly as the source intended.
fn emit_text(s: &str, style: Style, href: Option<&str>, out: &mut Vec<Tok>) {
    let words: Vec<&str> = s.split_whitespace().collect();
    if words.is_empty() {
        if !s.is_empty() {
            out.push(Tok::Space); // all-whitespace run is a single boundary
        }
        return;
    }
    if starts_ws(s) {
        out.push(Tok::Space);
    }
    for (k, word) in words.iter().enumerate() {
        if k > 0 {
            out.push(Tok::Space);
        }
        out.push(Tok::Word(Span {
            text: (*word).to_string(),
            style,
            href: href.map(str::to_string),
        }));
    }
    if ends_ws(s) {
        out.push(Tok::Space);
    }
}

fn starts_ws(s: &str) -> bool {
    s.starts_with(|c: char| c.is_whitespace())
}

fn ends_ws(s: &str) -> bool {
    s.ends_with(|c: char| c.is_whitespace())
}

/// Truncate a styled line to at most `w` display cells (for highlighted no-wrap code).
fn truncate_spans(spans: Vec<Span>, w: usize) -> Vec<Span> {
    let mut acc = 0;
    let mut out = Vec::new();
    for mut sp in spans {
        let sw = width(&sp.text);
        if acc + sw <= w {
            acc += sw;
            out.push(sp);
        } else {
            sp.text = truncate(&sp.text, w - acc);
            if !sp.text.is_empty() {
                out.push(sp);
            }
            break;
        }
    }
    out
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
        layout_blocks(&parse(md).blocks, w, false)
    }

    #[test]
    fn standalone_image_becomes_placeholder_and_imageref() {
        let doc = layout_document(&parse("# H\n\n![a cat](cat.png)\n\ntext").blocks, 80, false);
        assert_eq!(doc.images.len(), 1);
        let img = &doc.images[0];
        assert_eq!(img.url, "cat.png");
        assert_eq!(img.alt, "a cat");
        // The placeholder occupies its recorded line range and reads as a dim image line.
        assert_eq!(img.end - img.start, 1);
        assert!(doc.lines[img.start].plain_text().contains("image: a cat"));
        assert!(doc.lines[img.start].plain_text().contains("cat.png"));
    }

    #[test]
    fn inline_image_in_text_is_not_a_standalone_image() {
        // An image mixed with real text stays in the paragraph flow (alt text), not an ImageRef.
        let doc = layout_document(&parse("see ![x](y.png) here").blocks, 80, false);
        assert!(doc.images.is_empty());
    }

    #[test]
    fn resolved_image_expands_placeholder_and_shifts_indices() {
        let blocks = parse("![a](a.png)\n\n## After").blocks;
        // Baseline: 1-line placeholder, heading somewhere below it.
        let base = layout_document(&blocks, 80, false);
        assert_eq!(base.images[0].end - base.images[0].start, 1);
        let heading_line_before = base.headings[0].line;

        // Resolve image 0 to a fake 4-row render.
        let mut resolved = ResolvedImages::new();
        resolved.insert(0, vec![Line::default(); 4]);
        let doc = layout_document_with(&blocks, 80, false, &resolved);
        assert_eq!(doc.images[0].end - doc.images[0].start, 4); // placeholder → 4 rows
        assert_eq!(doc.headings[0].line, heading_line_before + 3); // everything after shifts down
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
    fn punctuation_glues_to_inline_elements() {
        // Regression: word-splitting must not fabricate a space before punctuation that
        // directly follows an inline element (`code`, not `code ,`).
        let lines = layout_doc("use `code`, then **stop**.", 80);
        assert_eq!(lines[0].plain_text(), "use code, then stop.");
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
    fn code_line_numbers_gutter() {
        let with = layout_document(&parse("```\nfoo\nbar\n```").blocks, 80, true).lines;
        assert!(with[0].plain_text().starts_with("1 │ foo"));
        assert!(with[1].plain_text().starts_with("2 │ bar"));
        // without the flag, no gutter
        let plain = layout_document(&parse("```\nfoo\nbar\n```").blocks, 80, false).lines;
        assert!(!plain[0].plain_text().contains('│'));
        assert_eq!(plain[0].plain_text(), "foo");
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
    fn doclayout_indexes_headings_with_line_positions() {
        let doc = layout_document(&parse("# One\n\nbody text\n\n## Two").blocks, 80, false);
        assert_eq!(doc.headings.len(), 2);
        assert_eq!(
            doc.headings[0],
            Heading {
                depth: 1,
                text: "One".into(),
                line: 0
            }
        );
        assert_eq!(doc.headings[1].depth, 2);
        assert_eq!(doc.headings[1].text, "Two");
        // "Two" heading lands on the line the index claims.
        let l = doc.headings[1].line;
        assert!(doc.text[l].contains("Two"));
    }

    #[test]
    fn doclayout_indexes_code_blocks() {
        let doc = layout_document(
            &parse("intro\n\n```rust\nlet x = 1;\nlet y = 2;\n```").blocks,
            80,
            false,
        );
        assert_eq!(doc.code_blocks.len(), 1);
        let c = &doc.code_blocks[0];
        assert_eq!(c.lang, "rust");
        assert!(c.content.contains("let x = 1;"));
        assert_eq!(c.end - c.start, 2); // two code lines
    }

    #[test]
    fn doclayout_indexes_links_with_lines() {
        let doc = layout_document(
            &parse("see [the docs](https://x.io) here").blocks,
            80,
            false,
        );
        assert_eq!(doc.links.len(), 1);
        assert_eq!(doc.links[0].url, "https://x.io");
        assert_eq!(doc.links[0].text, "the docs");
        assert_eq!(doc.links[0].line, 0);
    }

    #[test]
    fn doclayout_text_parallels_lines() {
        let doc = layout_document(&parse("# H\n\nsome text").blocks, 80, false);
        assert_eq!(doc.text.len(), doc.lines.len());
        for (t, l) in doc.text.iter().zip(&doc.lines) {
            assert_eq!(*t, l.plain_text());
        }
        assert!(!doc.is_empty());
    }

    #[test]
    fn reference_fixture_lays_out_within_widths() {
        let doc = parse(include_str!("../../tests/fixtures/mdterm-test.md"));
        for w in [44, 80, 120] {
            let lines = layout_blocks(&doc.blocks, w, false);
            assert!(!lines.is_empty());
            assert_within(&lines, w);
        }
    }
}

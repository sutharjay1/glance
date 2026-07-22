//! Markdown → typed block tree (CommonMark + GFM).
//!
//! Built on pulldown-cmark's flat event stream, folded into a tree with an explicit frame
//! stack. Two things happen at this boundary that matter beyond plain parsing:
//! - **Sanitization**: every piece of text/URL is stripped of `ESC` and C0/C1 control chars
//!   (except `\n`/`\t`) so hostile markdown cannot inject terminal escape sequences (§4.5).
//! - **GitHub callouts**: a blockquote whose first line is `[!NOTE]` (TIP/IMPORTANT/WARNING/
//!   CAUTION) becomes a [`Block::Callout`], fixing reference weakness #4.
//!
//! Unhandled containers (tables, footnotes, raw HTML) are skipped safely via a `Skip` frame;
//! table layout is a follow-up module.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag};

/// A parsed document: a flat sequence of top-level blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
}

/// A GitHub callout kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalloutKind {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

/// A block-level element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    Code {
        lang: Option<String>,
        code: String,
    },
    BlockQuote(Vec<Block>),
    Callout {
        kind: CalloutKind,
        blocks: Vec<Block>,
    },
    List {
        ordered: bool,
        start: u64,
        items: Vec<Item>,
    },
    ThematicBreak,
}

/// A list item. `task` is `Some(checked)` for GFM task-list items, `None` otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}

/// An inline element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Code(String),
    Emph(Vec<Inline>),
    Strong(Vec<Inline>),
    Strike(Vec<Inline>),
    Link { url: String, inlines: Vec<Inline> },
    Image { url: String, alt: String },
    SoftBreak,
    HardBreak,
}

/// Strip `ESC` and C0/C1 control characters (keeping `\n` and `\t`) from document text, so
/// markdown content can never inject terminal control sequences.
pub fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|&c| c == '\n' || c == '\t' || !c.is_control())
        .collect()
}

/// Parse `input` into a [`Document`].
pub fn parse(input: &str) -> Document {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_FOOTNOTES);

    let mut b = Builder::default();
    for ev in Parser::new_ext(input, opts) {
        b.event(ev);
    }
    Document {
        blocks: b.into_blocks(),
    }
}

/// Return a copy of the block tree with each link followed by a ` (url)` text run. Used for
/// terminals without OSC 8 hyperlink support (and pipe output): baking the suffix into the tree
/// *before* layout means it wraps as real content instead of overflowing the line (paint would
/// otherwise append it post-wrap). The link itself is preserved so its text stays styled.
pub fn with_url_suffixes(blocks: &[Block]) -> Vec<Block> {
    blocks.iter().map(suffix_block).collect()
}

fn suffix_block(b: &Block) -> Block {
    match b {
        Block::Heading { level, inlines } => Block::Heading {
            level: *level,
            inlines: suffix_inlines(inlines),
        },
        Block::Paragraph(v) => Block::Paragraph(suffix_inlines(v)),
        Block::Code { .. } | Block::ThematicBreak => b.clone(),
        Block::BlockQuote(bs) => Block::BlockQuote(with_url_suffixes(bs)),
        Block::Callout { kind, blocks } => Block::Callout {
            kind: *kind,
            blocks: with_url_suffixes(blocks),
        },
        Block::List {
            ordered,
            start,
            items,
        } => Block::List {
            ordered: *ordered,
            start: *start,
            items: items
                .iter()
                .map(|it| Item {
                    task: it.task,
                    blocks: with_url_suffixes(&it.blocks),
                })
                .collect(),
        },
    }
}

fn suffix_inlines(v: &[Inline]) -> Vec<Inline> {
    let mut out = Vec::new();
    for i in v {
        match i {
            Inline::Link { url, inlines } => {
                out.push(Inline::Link {
                    url: url.clone(),
                    inlines: suffix_inlines(inlines),
                });
                out.push(Inline::Text(format!(" ({url})")));
            }
            Inline::Emph(x) => out.push(Inline::Emph(suffix_inlines(x))),
            Inline::Strong(x) => out.push(Inline::Strong(suffix_inlines(x))),
            Inline::Strike(x) => out.push(Inline::Strike(suffix_inlines(x))),
            other => out.push(other.clone()),
        }
    }
    out
}

// --- Tree builder ---------------------------------------------------------

enum Frame {
    Root(Vec<Block>),
    Quote(Vec<Block>),
    List {
        ordered: bool,
        start: u64,
        items: Vec<Item>,
    },
    Item {
        task: Option<bool>,
        blocks: Vec<Block>,
    },
    /// A paragraph. `implicit` paragraphs wrap bare inline text that pulldown emits directly
    /// inside a container (tight-list items) with no `Start(Paragraph)` — we open and close
    /// them ourselves.
    Para {
        inlines: Vec<Inline>,
        implicit: bool,
    },
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    Emph(Vec<Inline>),
    Strong(Vec<Inline>),
    Strike(Vec<Inline>),
    Link {
        url: String,
        inlines: Vec<Inline>,
    },
    Code {
        lang: Option<String>,
        code: String,
    },
    Image {
        url: String,
        alt: String,
    },
    /// A container we don't model (table, footnote, HTML block) — swallowed without corrupting
    /// the stack.
    Skip,
}

struct Builder {
    stack: Vec<Frame>,
}

impl Default for Builder {
    fn default() -> Self {
        Builder {
            stack: vec![Frame::Root(Vec::new())],
        }
    }
}

impl Builder {
    fn event(&mut self, ev: Event<'_>) {
        match ev {
            Event::Start(tag) => self.start(tag),
            Event::End(_) => {
                self.flush_implicit();
                if let Some(frame) = self.stack.pop() {
                    self.finish(frame);
                }
            }
            Event::Text(t) => {
                let s = sanitize(&t);
                let handled = match self.stack.last_mut() {
                    Some(Frame::Code { code, .. }) => {
                        code.push_str(&s);
                        true
                    }
                    Some(Frame::Image { alt, .. }) => {
                        alt.push_str(&s);
                        true
                    }
                    _ => false,
                };
                if !handled {
                    self.attach_inline(Inline::Text(s));
                }
            }
            Event::Code(c) => self.attach_inline(Inline::Code(sanitize(&c))),
            Event::SoftBreak => self.attach_inline(Inline::SoftBreak),
            Event::HardBreak => self.attach_inline(Inline::HardBreak),
            Event::Rule => {
                self.flush_implicit();
                self.attach_block(Block::ThematicBreak);
            }
            Event::TaskListMarker(checked) => {
                for f in self.stack.iter_mut().rev() {
                    if let Frame::Item { task, .. } = f {
                        *task = Some(checked);
                        break;
                    }
                }
            }
            // Raw HTML, footnote refs, math: not renderable in the terminal — drop.
            _ => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        // Inline tags nest inside the current paragraph; block-level tags first close any
        // implicit paragraph so the new block is a sibling, not a child.
        let is_inline = matches!(
            tag,
            Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link { .. } | Tag::Image { .. }
        );
        if !is_inline {
            self.flush_implicit();
        }
        let frame = match tag {
            Tag::Paragraph => Frame::Para {
                inlines: Vec::new(),
                implicit: false,
            },
            Tag::Heading { level, .. } => Frame::Heading {
                level: heading_level(level),
                inlines: Vec::new(),
            },
            Tag::BlockQuote(_) => Frame::Quote(Vec::new()),
            Tag::CodeBlock(kind) => Frame::Code {
                lang: code_lang(&kind),
                code: String::new(),
            },
            Tag::List(start) => Frame::List {
                ordered: start.is_some(),
                start: start.unwrap_or(1),
                items: Vec::new(),
            },
            Tag::Item => Frame::Item {
                task: None,
                blocks: Vec::new(),
            },
            Tag::Emphasis => Frame::Emph(Vec::new()),
            Tag::Strong => Frame::Strong(Vec::new()),
            Tag::Strikethrough => Frame::Strike(Vec::new()),
            Tag::Link { dest_url, .. } => Frame::Link {
                url: sanitize(&dest_url),
                inlines: Vec::new(),
            },
            Tag::Image { dest_url, .. } => Frame::Image {
                url: sanitize(&dest_url),
                alt: String::new(),
            },
            // Tables, footnote defs, HTML blocks, metadata: skipped safely.
            _ => Frame::Skip,
        };
        self.stack.push(frame);
    }

    fn finish(&mut self, frame: Frame) {
        match frame {
            Frame::Para { inlines, .. } => self.attach_block(Block::Paragraph(inlines)),
            Frame::Heading { level, inlines } => {
                self.attach_block(Block::Heading { level, inlines })
            }
            Frame::Quote(blocks) => self.attach_block(make_quote(blocks)),
            Frame::Code { lang, code } => {
                let code = code.strip_suffix('\n').map(str::to_string).unwrap_or(code);
                self.attach_block(Block::Code { lang, code });
            }
            Frame::List {
                ordered,
                start,
                items,
            } => self.attach_block(Block::List {
                ordered,
                start,
                items,
            }),
            Frame::Item { task, blocks } => {
                if let Some(Frame::List { items, .. }) = self.stack.last_mut() {
                    items.push(Item { task, blocks });
                }
            }
            Frame::Emph(v) => self.attach_inline(Inline::Emph(v)),
            Frame::Strong(v) => self.attach_inline(Inline::Strong(v)),
            Frame::Strike(v) => self.attach_inline(Inline::Strike(v)),
            Frame::Link { url, inlines } => self.attach_inline(Inline::Link { url, inlines }),
            Frame::Image { url, alt } => self.attach_inline(Inline::Image { url, alt }),
            Frame::Skip | Frame::Root(_) => {}
        }
    }

    fn attach_block(&mut self, b: Block) {
        if let Some(Frame::Root(v) | Frame::Quote(v) | Frame::Item { blocks: v, .. }) =
            self.stack.last_mut()
        {
            v.push(b);
        }
    }

    fn attach_inline(&mut self, i: Inline) {
        // Tight-list items feed inline text straight into a block container; open an implicit
        // paragraph to hold it.
        if !self.top_is_inline_sink()
            && matches!(
                self.stack.last(),
                Some(Frame::Root(_) | Frame::Quote(_) | Frame::Item { .. })
            )
        {
            self.stack.push(Frame::Para {
                inlines: Vec::new(),
                implicit: true,
            });
        }
        if let Some(
            Frame::Para { inlines: v, .. }
            | Frame::Heading { inlines: v, .. }
            | Frame::Emph(v)
            | Frame::Strong(v)
            | Frame::Strike(v)
            | Frame::Link { inlines: v, .. },
        ) = self.stack.last_mut()
        {
            v.push(i);
        }
    }

    fn top_is_inline_sink(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                Frame::Para { .. }
                    | Frame::Heading { .. }
                    | Frame::Emph(_)
                    | Frame::Strong(_)
                    | Frame::Strike(_)
                    | Frame::Link { .. }
            )
        )
    }

    /// Close an implicit paragraph if one is on top of the stack.
    fn flush_implicit(&mut self) {
        if matches!(self.stack.last(), Some(Frame::Para { implicit: true, .. })) {
            if let Some(frame) = self.stack.pop() {
                self.finish(frame);
            }
        }
    }

    fn into_blocks(mut self) -> Vec<Block> {
        self.flush_implicit();
        match self.stack.drain(..).next() {
            Some(Frame::Root(v)) => v,
            _ => Vec::new(),
        }
    }
}

fn heading_level(l: HeadingLevel) -> u8 {
    match l {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn code_lang(kind: &CodeBlockKind<'_>) -> Option<String> {
    match kind {
        CodeBlockKind::Fenced(l) => {
            let l = sanitize(l);
            l.split_whitespace()
                .next()
                .map(str::to_string)
                .filter(|s| !s.is_empty())
        }
        CodeBlockKind::Indented => None,
    }
}

/// Turn a finished blockquote into a [`Block::Callout`] if its first line is a GitHub alert
/// marker, otherwise a plain [`Block::BlockQuote`].
fn make_quote(blocks: Vec<Block>) -> Block {
    match detect_callout(blocks) {
        Ok((kind, body)) => Block::Callout { kind, blocks: body },
        Err(blocks) => Block::BlockQuote(blocks),
    }
}

fn detect_callout(mut blocks: Vec<Block>) -> Result<(CalloutKind, Vec<Block>), Vec<Block>> {
    // The marker `[!NOTE]` can be split across several Text inlines (pulldown emits `[`,
    // `!NOTE`, `]`), so join the leading run of Text inlines before matching.
    let Some(Block::Paragraph(inls)) = blocks.first() else {
        return Err(blocks);
    };
    let lead_count = inls
        .iter()
        .take_while(|i| matches!(i, Inline::Text(_)))
        .count();
    let lead: String = inls[..lead_count]
        .iter()
        .map(|i| match i {
            Inline::Text(s) => s.as_str(),
            _ => "",
        })
        .collect();
    let Some((kind, rest)) = parse_marker(&lead) else {
        return Err(blocks);
    };
    if let Some(Block::Paragraph(inls)) = blocks.first_mut() {
        inls.drain(..lead_count); // drop the consumed marker text inlines
        if !rest.is_empty() {
            inls.insert(0, Inline::Text(rest)); // keep trailing same-line text as body
        } else if matches!(inls.first(), Some(Inline::SoftBreak | Inline::HardBreak)) {
            inls.remove(0); // drop the break that followed the marker line
        }
        if inls.is_empty() {
            blocks.remove(0);
        }
    }
    Ok((kind, blocks))
}

/// Parse a `[!NOTE]`-style marker from the start of a text run, returning the kind and any
/// trailing text on the same line.
fn parse_marker(t: &str) -> Option<(CalloutKind, String)> {
    let t = t.trim_start();
    let rest = t.strip_prefix("[!")?;
    let close = rest.find(']')?;
    let kind = callout_kind(&rest[..close])?;
    Some((kind, rest[close + 1..].trim_start().to_string()))
}

fn callout_kind(name: &str) -> Option<CalloutKind> {
    match name.to_ascii_uppercase().as_str() {
        "NOTE" => Some(CalloutKind::Note),
        "TIP" => Some(CalloutKind::Tip),
        "IMPORTANT" => Some(CalloutKind::Important),
        "WARNING" => Some(CalloutKind::Warning),
        "CAUTION" => Some(CalloutKind::Caution),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Flatten all text (incl. inline code) in a block tree — for loose assertions.
    fn text_of(blocks: &[Block]) -> String {
        fn inl(out: &mut String, i: &Inline) {
            match i {
                Inline::Text(s) | Inline::Code(s) => out.push_str(s),
                Inline::Emph(v) | Inline::Strong(v) | Inline::Strike(v) => {
                    v.iter().for_each(|x| inl(out, x))
                }
                Inline::Link { inlines, .. } => inlines.iter().for_each(|x| inl(out, x)),
                Inline::Image { alt, .. } => out.push_str(alt),
                Inline::SoftBreak | Inline::HardBreak => out.push(' '),
            }
        }
        fn blk(out: &mut String, b: &Block) {
            match b {
                Block::Heading { inlines, .. } | Block::Paragraph(inlines) => {
                    inlines.iter().for_each(|i| inl(out, i))
                }
                Block::Code { code, .. } => out.push_str(code),
                Block::BlockQuote(bs) | Block::Callout { blocks: bs, .. } => {
                    bs.iter().for_each(|b| blk(out, b))
                }
                Block::List { items, .. } => items
                    .iter()
                    .for_each(|it| it.blocks.iter().for_each(|b| blk(out, b))),
                Block::ThematicBreak => {}
            }
        }
        let mut out = String::new();
        blocks.iter().for_each(|b| blk(&mut out, b));
        out
    }

    #[test]
    fn sanitize_strips_control_chars_keeps_tab_newline() {
        assert_eq!(sanitize("a\x1b[31mb"), "a[31mb");
        assert_eq!(sanitize("keep\tthis\nand\x07drop"), "keep\tthis\nanddrop");
    }

    #[test]
    fn heading_with_emphasis() {
        let doc = parse("# Hello *world*");
        assert_eq!(
            doc.blocks,
            vec![Block::Heading {
                level: 1,
                inlines: vec![
                    Inline::Text("Hello ".into()),
                    Inline::Emph(vec![Inline::Text("world".into())]),
                ],
            }]
        );
    }

    #[test]
    fn paragraph_strong_code_link() {
        let doc = parse("go **fast** with `code` and [a](https://x.io)");
        let Block::Paragraph(inls) = &doc.blocks[0] else {
            panic!("expected paragraph, got {:?}", doc.blocks);
        };
        assert!(inls.contains(&Inline::Strong(vec![Inline::Text("fast".into())])));
        assert!(inls.contains(&Inline::Code("code".into())));
        assert!(inls
            .iter()
            .any(|i| matches!(i, Inline::Link { url, .. } if url == "https://x.io")));
    }

    #[test]
    fn fenced_code_captures_lang_and_body() {
        let doc = parse("```rust\nfn main() {}\n```");
        assert_eq!(
            doc.blocks,
            vec![Block::Code {
                lang: Some("rust".into()),
                code: "fn main() {}".into(),
            }]
        );
    }

    #[test]
    fn unordered_list_items() {
        let doc = parse("- one\n- two");
        let Block::List { ordered, items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(!ordered);
        assert_eq!(items.len(), 2);
        assert_eq!(text_of(&items[0].blocks), "one");
    }

    #[test]
    fn ordered_list_preserves_start() {
        let doc = parse("3. c\n4. d");
        let Block::List {
            ordered,
            start,
            items,
        } = &doc.blocks[0]
        else {
            panic!("expected list");
        };
        assert!(ordered);
        assert_eq!(*start, 3);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn task_list_markers() {
        let doc = parse("- [x] done\n- [ ] todo");
        let Block::List { items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert_eq!(items[0].task, Some(true));
        assert_eq!(items[1].task, Some(false));
    }

    #[test]
    fn plain_blockquote() {
        let doc = parse("> just a quote");
        assert!(matches!(doc.blocks[0], Block::BlockQuote(_)));
    }

    #[test]
    fn github_callout_note() {
        let doc = parse("> [!NOTE]\n> the body text");
        let Block::Callout { kind, blocks } = &doc.blocks[0] else {
            panic!("expected callout, got {:?}", doc.blocks);
        };
        assert_eq!(*kind, CalloutKind::Note);
        assert_eq!(text_of(blocks), "the body text");
    }

    #[test]
    fn callout_kinds_case_insensitive() {
        assert_eq!(
            parse_marker("[!warning] x").map(|(k, _)| k),
            Some(CalloutKind::Warning)
        );
        assert_eq!(
            parse_marker("[!TIP]").map(|(k, _)| k),
            Some(CalloutKind::Tip)
        );
        assert_eq!(parse_marker("[!nope]"), None);
        assert_eq!(parse_marker("not a marker"), None);
    }

    #[test]
    fn thematic_break() {
        let doc = parse("a\n\n---\n\nb");
        assert!(doc.blocks.iter().any(|b| matches!(b, Block::ThematicBreak)));
    }

    #[test]
    fn parses_reference_fixture_without_panicking() {
        // Real-world smoke test against the mdterm reference doc (headings, code, tables,
        // lists, links, images, callouts). Must not panic and must yield blocks.
        let md = include_str!("../../tests/fixtures/mdterm-test.md");
        let doc = parse(md);
        assert!(!doc.blocks.is_empty());
        assert!(doc
            .blocks
            .iter()
            .any(|b| matches!(b, Block::Heading { .. })));
    }

    #[test]
    fn hostile_escape_in_text_is_neutralized() {
        let doc = parse("normal \x1b]0;pwned\x07 text");
        let flat = text_of(&doc.blocks);
        assert!(!flat.contains('\x1b'));
        assert!(!flat.contains('\x07'));
    }
}

//! Paint: a laid-out [`Line`] → an ANSI string, against a [`Theme`] and the terminal's
//! [`ColorDepth`].
//!
//! Painting is separate from layout so a theme toggle or color-depth change re-paints without
//! re-wrapping. Link handling:
//! - **OSC 8 link runs**: consecutive spans sharing an href become one hyperlink (not one per
//!   word), suppressed on `no_wrap` lines (tables/code) to keep alignment (plan §8).
//! - **No OSC 8**: the ` (url)` suffix is baked into the block tree *before* layout by
//!   `parse::with_url_suffixes` (so it wraps as content), not appended here post-wrap.
//!
//! `ColorDepth::None` yields clean plain text (no SGR, no OSC) for `--no-color`/pipe output.

use crate::md::{layout::layout_blocks, parse::parse};
use crate::style::{Line, Role, Span};
use crate::term::ansi::{self, Rgb};
use crate::term::caps::ColorDepth;
use crate::term::osc;
use crate::theme::Theme;

/// Render `line` to an ANSI string.
///
/// `hyperlinks` should reflect terminal OSC 8 support; when false, link runs get a dim
/// ` (url)` suffix instead (except on `no_wrap` lines).
pub fn paint(line: &Line, theme: &Theme, depth: ColorDepth, hyperlinks: bool) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < line.spans.len() {
        let href = line.spans[i].href.as_deref();
        // Collect the maximal run of spans sharing this href.
        let mut j = i;
        while j < line.spans.len() && line.spans[j].href.as_deref() == href {
            j += 1;
        }
        let run = &line.spans[i..j];
        let linkable = href.is_some() && !line.no_wrap;
        let use_osc = linkable && hyperlinks && depth != ColorDepth::None;

        if use_osc {
            out.push_str(&osc::link_open(href.unwrap()));
        }
        for span in run {
            paint_span(&mut out, span, theme, depth);
        }
        if use_osc {
            out.push_str(osc::LINK_CLOSE);
        }
        i = j;
    }
    out
}

/// Parse → layout → paint a whole document to a single string (provisional pipe/demo path;
/// the interactive viewport render is a later module).
pub fn render_document(
    input: &str,
    width: usize,
    theme: &Theme,
    depth: ColorDepth,
    hyperlinks: bool,
) -> String {
    let doc = parse(input);
    // Without OSC 8, bake ` (url)` suffixes into the tree so they wrap as content.
    let blocks = if hyperlinks {
        doc.blocks
    } else {
        crate::md::parse::with_url_suffixes(&doc.blocks)
    };
    let mut out = String::new();
    for line in layout_blocks(&blocks, width) {
        out.push_str(&paint(&line, theme, depth, hyperlinks));
        out.push('\n');
    }
    out
}

fn paint_span(out: &mut String, span: &Span, theme: &Theme, depth: ColorDepth) {
    if depth == ColorDepth::None {
        out.push_str(&span.text);
        return;
    }
    let params = sgr_params(span, theme, depth);
    if !params.is_empty() {
        out.push_str(&ansi::sgr(&params));
        out.push_str(&span.text);
        out.push_str(ansi::RESET);
    } else {
        out.push_str(&span.text);
    }
}

fn sgr_params(span: &Span, theme: &Theme, depth: ColorDepth) -> String {
    let mut parts: Vec<String> = Vec::new();
    if span.style.bold {
        parts.push("1".into());
    }
    if span.style.italic {
        parts.push("3".into());
    }
    if span.style.strike {
        parts.push("9".into());
    }
    if span.href.is_some() {
        parts.push("4".into()); // underline links
    }
    if span.style.highlight {
        parts.push("7".into()); // reverse video for search matches
    }
    let fgp = ansi::fg(pick_color(span, theme), depth);
    if !fgp.is_empty() {
        parts.push(fgp);
    }
    parts.join(";")
}

fn pick_color(span: &Span, theme: &Theme) -> Rgb {
    if span.href.is_some() {
        return theme.link;
    }
    match span.style.role {
        Role::Heading => theme.heading,
        Role::Marker | Role::Accent => theme.accent,
        Role::Dim => theme.dim,
        Role::Rule => theme.rule,
        Role::Keyword => theme.kw,
        Role::Str => theme.string,
        Role::Comment => theme.comment,
        Role::Number => theme.number,
        Role::Function => theme.function,
        // Plain code text (highlighter's default) vs. ordinary body text.
        Role::Body if span.style.code => theme.code,
        Role::Body => theme.body,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{Line, Span, Style};
    use crate::theme;

    fn dark() -> Theme {
        theme::dark()
    }

    fn line(spans: Vec<Span>, no_wrap: bool) -> Line {
        Line { spans, no_wrap }
    }

    #[test]
    fn plain_depth_emits_no_escapes() {
        let l = line(
            vec![Span::new(
                "bold",
                Style {
                    bold: true,
                    ..Default::default()
                },
            )],
            false,
        );
        let s = paint(&l, &dark(), ColorDepth::None, true);
        assert_eq!(s, "bold");
        assert!(!s.contains('\x1b'));
    }

    #[test]
    fn truecolor_bold_wraps_with_sgr_and_reset() {
        let l = line(
            vec![Span::new(
                "x",
                Style {
                    bold: true,
                    ..Default::default()
                },
            )],
            false,
        );
        let s = paint(&l, &dark(), ColorDepth::TrueColor, true);
        assert!(s.starts_with("\x1b[1;38;2;"));
        assert!(s.ends_with("\x1b[0m"));
        assert!(s.contains('x'));
    }

    #[test]
    fn link_run_uses_single_osc8() {
        // Real pipeline: a two-word link must paint as ONE OSC 8 run (internal space included).
        use crate::md::layout::layout_blocks;
        use crate::md::parse::parse;
        let lines = layout_blocks(&parse("see [the docs](https://x.io) now").blocks, 80);
        let s = paint(&lines[0], &dark(), ColorDepth::TrueColor, true);
        assert_eq!(s.matches("\x1b]8;;https://x.io\x1b\\").count(), 1);
        assert_eq!(s.matches(osc::LINK_CLOSE).count(), 1);
        // "now" is outside the link.
        assert!(s.contains("now"));
    }

    #[test]
    fn no_hyperlinks_emits_no_osc8() {
        // paint itself never adds a suffix now — it only decides OSC 8. The ` (url)` suffix is
        // baked into the tree by render_document (see url_suffix_baked_when_no_hyperlinks).
        let l = line(
            vec![Span {
                text: "docs".into(),
                style: Style::default(),
                href: Some("https://x.io".into()),
            }],
            false,
        );
        let s = paint(&l, &dark(), ColorDepth::TrueColor, false);
        assert!(!s.contains("\x1b]8;;")); // no OSC 8 when hyperlinks off
    }

    #[test]
    fn url_suffix_baked_when_no_hyperlinks() {
        // render_document without hyperlinks shows the URL inline (wrapped as content).
        let out = render_document(
            "see [docs](https://x.io) now",
            80,
            &dark(),
            ColorDepth::None,
            false,
        );
        assert!(out.contains("(https://x.io)"));
        // ...and it does not appear when hyperlinks are on (OSC 8 carries it instead).
        let osc = render_document(
            "see [docs](https://x.io) now",
            80,
            &dark(),
            ColorDepth::TrueColor,
            true,
        );
        assert!(!osc.contains("(https://x.io)"));
        assert!(osc.contains("\x1b]8;;https://x.io"));
    }

    #[test]
    fn no_wrap_suppresses_link_suffix_and_osc() {
        let l = line(
            vec![Span {
                text: "code".into(),
                style: Style::default(),
                href: Some("https://x.io".into()),
            }],
            true, // no_wrap
        );
        let with_osc = paint(&l, &dark(), ColorDepth::TrueColor, true);
        let without = paint(&l, &dark(), ColorDepth::TrueColor, false);
        assert!(!with_osc.contains("\x1b]8;;"));
        assert!(!without.contains("(https://x.io)"));
    }

    #[test]
    fn highlighted_code_colors_reach_output() {
        // Full chain: layout invokes highlight → paint colors the tokens.
        let out = render_document(
            "```rust\nlet x = 42;\n```",
            80,
            &dark(),
            ColorDepth::TrueColor,
            false,
        );
        assert!(out.contains("203;166;247"), "keyword mauve missing"); // `let`
        assert!(out.contains("250;179;135"), "number peach missing"); // `42`
    }

    #[test]
    fn render_document_smoke() {
        let out = render_document(
            "# Title\n\nsome *text*",
            80,
            &dark(),
            ColorDepth::None,
            true,
        );
        assert!(out.contains("Title"));
        assert!(out.contains("some text"));
        assert!(!out.contains('\x1b')); // plain in no-color
    }
}

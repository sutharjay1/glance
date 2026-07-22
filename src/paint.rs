//! Paint: a laid-out [`Line`] → an ANSI string, against a [`Theme`] and the terminal's
//! [`ColorDepth`].
//!
//! Painting is separate from layout so a theme toggle or color-depth change re-paints without
//! re-wrapping. Two behaviors from the plan live here:
//! - **Link runs**: consecutive spans sharing an href become one OSC 8 hyperlink (not one per
//!   word). When hyperlinks are unavailable, a single dim ` (url)` suffix is appended per run.
//! - **`no_wrap` suppression**: code/table/rule lines get neither OSC 8 nor ` (url)` suffixes,
//!   so column alignment is preserved (plan §8).
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
        } else if linkable && !hyperlinks {
            // One dim suffix per link run.
            paint_suffix(&mut out, &format!(" ({})", href.unwrap()), theme, depth);
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
    let mut out = String::new();
    for line in layout_blocks(&doc.blocks, width) {
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

fn paint_suffix(out: &mut String, text: &str, theme: &Theme, depth: ColorDepth) {
    if depth == ColorDepth::None {
        out.push_str(text);
        return;
    }
    let p = ansi::fg(theme.dim, depth);
    if p.is_empty() {
        out.push_str(text);
    } else {
        out.push_str(&ansi::sgr(&p));
        out.push_str(text);
        out.push_str(ansi::RESET);
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
    if span.style.code {
        return theme.code;
    }
    match span.style.role {
        Role::Heading => theme.heading,
        Role::Marker | Role::Accent => theme.accent,
        Role::Dim => theme.dim,
        Role::Rule => theme.rule,
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
    fn link_suffix_fallback_when_no_hyperlinks() {
        let l = line(
            vec![Span {
                text: "docs".into(),
                style: Style::default(),
                href: Some("https://x.io".into()),
            }],
            false,
        );
        let s = paint(&l, &dark(), ColorDepth::TrueColor, false);
        assert!(!s.contains("\x1b]8;;")); // no OSC 8
        assert!(s.contains("(https://x.io)")); // dim suffix instead
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

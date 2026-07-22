//! Frame building + damage-conscious rendering.
//!
//! A [`Frame`] is the painted content of each terminal row. [`build_frame`] slices the viewport
//! out of a [`DocLayout`] and paints it; [`render`] diffs the previous frame against the next and
//! emits terminal writes for **only the changed rows**, each preceded by a cursor move +
//! clear-to-EOL, the whole update wrapped in synchronized output so it appears without tearing
//! (ADR 0004). `render(None, next)` forces a full repaint — the `--full-repaint` debug fallback.

use crate::md::layout::DocLayout;
use crate::paint::paint;
use crate::term::ansi;
use crate::term::caps::ColorDepth;
use crate::theme::Theme;

/// The painted content of each visible row (no cursor moves baked in).
pub type Frame = Vec<String>;

/// Build a `height`-row frame showing `doc` scrolled to `top`. Rows past the end of the document
/// are blank.
pub fn build_frame(
    doc: &DocLayout,
    top: usize,
    height: usize,
    theme: &Theme,
    depth: ColorDepth,
    hyperlinks: bool,
) -> Frame {
    (0..height)
        .map(|row| match doc.lines.get(top + row) {
            Some(line) => paint(line, theme, depth, hyperlinks),
            None => String::new(),
        })
        .collect()
}

/// Emit the terminal writes to turn `prev` into `next`, rewriting only changed rows. Pass
/// `prev = None` (or a differently sized frame) to repaint everything.
pub fn render(prev: Option<&Frame>, next: &Frame) -> String {
    let mut out = String::with_capacity(next.len() * 8);
    out.push_str(ansi::SYNC_BEGIN);
    for (row, line) in next.iter().enumerate() {
        let unchanged = prev.and_then(|p| p.get(row)) == Some(line);
        if !unchanged {
            out.push_str(&ansi::move_to(row, 0));
            out.push_str(ansi::CLEAR_LINE_EOL);
            out.push_str(line);
        }
    }
    out.push_str(ansi::SYNC_END);
    out
}

/// Clamp a scroll offset so the last screenful of a document stays on screen.
pub fn max_top(total_lines: usize, height: usize) -> usize {
    total_lines.saturating_sub(height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::md::{layout::layout_document, parse::parse};
    use crate::theme;

    fn doc(md: &str, w: usize) -> DocLayout {
        layout_document(&parse(md).blocks, w)
    }

    #[test]
    fn build_frame_slices_and_pads() {
        let d = doc("# A\n\nb\n\nc", 80);
        let f = build_frame(&d, 0, 10, &theme::dark(), ColorDepth::None, true);
        assert_eq!(f.len(), 10); // padded to height
        assert!(f[0].contains('A'));
        // rows past the document are blank
        assert!(f.last().unwrap().is_empty());
    }

    #[test]
    fn build_frame_respects_top() {
        let d = doc("l0\n\nl1\n\nl2\n\nl3", 80);
        let f = build_frame(&d, 2, 2, &theme::dark(), ColorDepth::None, true);
        assert_eq!(f.len(), 2);
        // starts at line index 2 of the doc
        assert_eq!(f[0], d.text[2]);
    }

    #[test]
    fn full_repaint_writes_every_row() {
        let next = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let out = render(None, &next);
        assert!(out.starts_with(ansi::SYNC_BEGIN));
        assert!(out.ends_with(ansi::SYNC_END));
        // at least one cursor move per row
        assert!(out.matches("\x1b[").count() >= 3);
        for s in ["a", "b", "c"] {
            assert!(out.contains(s));
        }
    }

    #[test]
    fn damage_diff_writes_only_changed_rows() {
        let prev = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let next = vec!["a".to_string(), "B!".to_string(), "c".to_string()];
        let out = render(Some(&prev), &next);
        assert!(out.contains("B!")); // changed row written
        assert!(!out.contains("\x1b[1;1H")); // row 0 (unchanged) not moved-to
        assert!(out.contains(&ansi::move_to(1, 0))); // row 1 moved-to
                                                     // exactly one row rewritten → exactly one clear-to-EOL
        assert_eq!(out.matches(ansi::CLEAR_LINE_EOL).count(), 1);
    }

    #[test]
    fn identical_frames_write_nothing_but_sync() {
        let f = vec!["x".to_string(), "y".to_string()];
        let out = render(Some(&f), &f);
        assert_eq!(out, format!("{}{}", ansi::SYNC_BEGIN, ansi::SYNC_END));
    }

    #[test]
    fn max_top_keeps_last_screen() {
        assert_eq!(max_top(100, 30), 70);
        assert_eq!(max_top(10, 30), 0); // doc shorter than screen
    }
}

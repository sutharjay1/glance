//! Viewer state + navigation.
//!
//! `ViewerState` owns the parsed document, its current layout, and the scroll offset. All
//! navigation is expressed as pure methods that mutate `top` (always clamped to a valid range),
//! and `on_key`/`on_mouse` map input events to those methods returning an [`Action`]. Keeping
//! this free of terminal I/O makes the whole navigation surface unit-testable; `view::app`
//! supplies the real event source and calls `view::render`.

use crate::md::layout::{layout_document, DocLayout};
use crate::md::parse::Block;
use crate::term::input::{Key, Mouse};

/// What the event loop should do after handling an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// State changed; repaint.
    Redraw,
    /// Quit the viewer.
    Quit,
    /// Nothing to do.
    Ignore,
}

/// Rows scrolled per mouse-wheel notch.
const WHEEL_LINES: isize = 3;

pub struct ViewerState {
    blocks: Vec<Block>,
    pub doc: DocLayout,
    pub top: usize,
    /// Content width used for layout.
    pub width: usize,
    /// Viewport height in rows.
    pub height: usize,
}

impl ViewerState {
    pub fn new(blocks: Vec<Block>, width: usize, height: usize) -> Self {
        let doc = layout_document(&blocks, width);
        ViewerState {
            blocks,
            doc,
            top: 0,
            width,
            height,
        }
    }

    /// The largest valid `top` — keeps the last screenful on screen.
    pub fn max_top(&self) -> usize {
        self.doc.len().saturating_sub(self.height)
    }

    /// Scroll by `delta` lines (negative = up), clamped.
    pub fn scroll(&mut self, delta: isize) {
        let t = (self.top as isize + delta).max(0) as usize;
        self.top = t.min(self.max_top());
    }

    pub fn to_top(&mut self) {
        self.top = 0;
    }

    pub fn to_bottom(&mut self) {
        self.top = self.max_top();
    }

    /// Jump so the next heading below the viewport top is the first visible line.
    pub fn next_heading(&mut self) {
        if let Some(h) = self.doc.headings.iter().find(|h| h.line > self.top) {
            self.top = h.line.min(self.max_top());
        }
    }

    /// Jump to the previous heading above the viewport top.
    pub fn prev_heading(&mut self) {
        if let Some(h) = self.doc.headings.iter().rev().find(|h| h.line < self.top) {
            self.top = h.line;
        }
    }

    /// Re-layout at a new terminal size, anchoring roughly to the current scroll line.
    pub fn on_resize(&mut self, width: usize, height: usize) {
        let anchor = self.top;
        self.width = width;
        self.height = height;
        self.doc = layout_document(&self.blocks, width);
        self.top = anchor.min(self.max_top());
    }

    /// Map a key to a navigation action (§5 movement subset).
    pub fn on_key(&mut self, key: Key) -> Action {
        let page = self.height.max(1) as isize;
        let half = (self.height / 2).max(1) as isize;
        match key {
            Key::Char('j') | Key::Down => self.scroll(1),
            Key::Char('k') | Key::Up => self.scroll(-1),
            Key::Char(' ') | Key::PageDown => self.scroll(page),
            Key::Char('b') | Key::PageUp => self.scroll(-page),
            Key::Char('d') | Key::Ctrl('d') => self.scroll(half),
            Key::Char('u') | Key::Ctrl('u') => self.scroll(-half),
            Key::Char('g') | Key::Home => self.to_top(),
            Key::Char('G') | Key::End => self.to_bottom(),
            Key::Char(']') => self.next_heading(),
            Key::Char('[') => self.prev_heading(),
            Key::Char('q') | Key::Ctrl('c') => return Action::Quit,
            _ => return Action::Ignore,
        }
        Action::Redraw
    }

    /// Map a mouse event to a navigation action (wheel scroll; clicks handled later).
    pub fn on_mouse(&mut self, m: Mouse) -> Action {
        match m {
            Mouse::ScrollDown => self.scroll(WHEEL_LINES),
            Mouse::ScrollUp => self.scroll(-WHEEL_LINES),
            Mouse::Click { .. } => return Action::Ignore,
        }
        Action::Redraw
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::md::parse::parse;

    fn state(md: &str, width: usize, height: usize) -> ViewerState {
        ViewerState::new(parse(md).blocks, width, height)
    }

    /// A document with `n` short paragraphs → at least `n` lines.
    fn tall(n: usize) -> String {
        (0..n)
            .map(|i| format!("line{i}"))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    #[test]
    fn starts_at_top_and_lays_out() {
        let s = state("# H\n\nbody", 80, 24);
        assert_eq!(s.top, 0);
        assert!(!s.doc.is_empty());
    }

    #[test]
    fn scroll_clamps_both_ends() {
        let mut s = state(&tall(50), 80, 10);
        s.scroll(-5);
        assert_eq!(s.top, 0); // can't scroll above the top
        s.scroll(10_000);
        assert_eq!(s.top, s.max_top()); // can't scroll past the bottom
        assert_eq!(s.top, s.doc.len() - 10);
    }

    #[test]
    fn short_doc_cannot_scroll() {
        let mut s = state("just a bit", 80, 40);
        assert_eq!(s.max_top(), 0);
        s.scroll(5);
        assert_eq!(s.top, 0);
    }

    #[test]
    fn page_and_half_page() {
        let mut s = state(&tall(100), 80, 20);
        assert_eq!(s.on_key(Key::Char(' ')), Action::Redraw);
        assert_eq!(s.top, 20); // page = height
        s.on_key(Key::Char('u'));
        assert_eq!(s.top, 10); // half page up
    }

    #[test]
    fn g_and_shift_g() {
        let mut s = state(&tall(60), 80, 10);
        s.on_key(Key::Char('G'));
        assert_eq!(s.top, s.max_top());
        s.on_key(Key::Char('g'));
        assert_eq!(s.top, 0);
    }

    #[test]
    fn heading_jumps() {
        // headings at doc lines; jump forward then back.
        let mut s = state("# A\n\nx\n\n## B\n\ny\n\n### C", 80, 3);
        let hs: Vec<usize> = s.doc.headings.iter().map(|h| h.line).collect();
        assert!(hs.len() >= 3);
        s.next_heading();
        assert_eq!(s.top, hs[1].min(s.max_top()));
        let after_first = s.top;
        s.next_heading();
        assert!(s.top >= after_first);
        s.prev_heading();
        assert!(s.top < hs[2]);
    }

    #[test]
    fn resize_relayouts_and_keeps_anchor() {
        let mut s = state(&tall(100), 80, 20);
        s.scroll(30);
        let before = s.top;
        s.on_resize(40, 10); // narrower + shorter
        assert_eq!(s.width, 40);
        assert_eq!(s.height, 10);
        assert!(s.top <= s.max_top());
        assert!(s.top <= before || s.top == s.max_top());
    }

    #[test]
    fn quit_and_ignore() {
        let mut s = state("hi", 80, 24);
        assert_eq!(s.on_key(Key::Char('q')), Action::Quit);
        assert_eq!(s.on_key(Key::Ctrl('c')), Action::Quit);
        assert_eq!(s.on_key(Key::Char('z')), Action::Ignore);
    }

    #[test]
    fn wheel_scrolls() {
        let mut s = state(&tall(50), 80, 10);
        assert_eq!(s.on_mouse(Mouse::ScrollDown), Action::Redraw);
        assert_eq!(s.top, WHEEL_LINES as usize);
        s.on_mouse(Mouse::ScrollUp);
        assert_eq!(s.top, 0);
    }
}

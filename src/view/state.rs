//! Viewer state + navigation.
//!
//! `ViewerState` owns the parsed document, its current layout, and the scroll offset. All
//! navigation is expressed as pure methods that mutate `top` (always clamped to a valid range),
//! and `on_key`/`on_mouse` map input events to those methods returning an [`Action`]. Keeping
//! this free of terminal I/O makes the whole navigation surface unit-testable; `view::app`
//! supplies the real event source and calls `view::render`.

use std::path::PathBuf;

use crate::md::layout::{layout_document, DocLayout};
use crate::md::parse::{parse, Block};
use crate::term::input::{Key, Mouse};
use crate::view::search::Search;

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
    /// Active in-document search, if any (drives `n`/`N`, highlighting, and the status readout).
    pub search: Option<Search>,
    /// Path of the current file (for resolving relative links and `p` copy-path); `None` for stdin.
    path: Option<PathBuf>,
    /// Back-navigation stack of `(path, scroll)` for following and returning from local links.
    history: Vec<(PathBuf, usize)>,
}

impl ViewerState {
    pub fn new(blocks: Vec<Block>, width: usize, height: usize, path: Option<PathBuf>) -> Self {
        let doc = layout_document(&blocks, width);
        ViewerState {
            blocks,
            doc,
            top: 0,
            width,
            height,
            search: None,
            path,
            history: Vec::new(),
        }
    }

    /// The directory of the current file, for resolving relative links.
    pub fn current_dir(&self) -> Option<PathBuf> {
        self.path
            .as_ref()
            .and_then(|p| p.parent())
            .map(std::path::Path::to_path_buf)
    }

    /// Load `path` as the new document, pushing the current file+scroll onto the back stack.
    pub fn load(&mut self, path: PathBuf) -> std::io::Result<()> {
        let input = std::fs::read_to_string(&path)?;
        if let Some(cur) = self.path.take() {
            self.history.push((cur, self.top));
        }
        self.blocks = parse(&input).blocks;
        self.doc = layout_document(&self.blocks, self.width);
        self.top = 0;
        self.search = None;
        self.path = Some(path);
        Ok(())
    }

    /// Return to the previously viewed file (restoring its scroll). Returns false at the root.
    pub fn back(&mut self) -> bool {
        let Some((path, top)) = self.history.pop() else {
            return false;
        };
        let Ok(input) = std::fs::read_to_string(&path) else {
            return false;
        };
        self.blocks = parse(&input).blocks;
        self.doc = layout_document(&self.blocks, self.width);
        self.path = Some(path);
        self.search = None;
        self.top = top.min(self.max_top());
        true
    }

    pub fn can_go_back(&self) -> bool {
        !self.history.is_empty()
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

    /// Scroll so `line` sits in the middle of the viewport (used to reveal a search match).
    pub fn center_on(&mut self, line: usize) {
        self.top = line.saturating_sub(self.height / 2).min(self.max_top());
    }

    /// Run a search, store it, and jump to the first match. Returns the match count. An empty
    /// query clears any active search.
    pub fn run_search(&mut self, query: &str) -> usize {
        if query.is_empty() {
            self.search = None;
            return 0;
        }
        let s = Search::new(query, &self.doc.text);
        let n = s.len();
        let first = s.current().map(|m| m.line);
        self.search = Some(s);
        if let Some(line) = first {
            self.center_on(line);
        }
        n
    }

    /// Cycle to the next / previous match and re-center on it.
    pub fn search_next(&mut self) {
        let line = self.search.as_mut().and_then(|s| {
            s.next();
            s.current().map(|m| m.line)
        });
        if let Some(line) = line {
            self.center_on(line);
        }
    }

    pub fn search_prev(&mut self) {
        let line = self.search.as_mut().and_then(|s| {
            s.prev();
            s.current().map(|m| m.line)
        });
        if let Some(line) = line {
            self.center_on(line);
        }
    }

    pub fn clear_search(&mut self) {
        self.search = None;
    }

    /// Re-layout at a new terminal size, anchoring roughly to the current scroll line. A live
    /// search is re-run against the new layout so its match positions stay valid.
    pub fn on_resize(&mut self, width: usize, height: usize) {
        let anchor = self.top;
        self.width = width;
        self.height = height;
        self.doc = layout_document(&self.blocks, width);
        self.top = anchor.min(self.max_top());
        if let Some(s) = &self.search {
            self.search = Some(Search::new(&s.query, &self.doc.text));
        }
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
            Key::Char('n') if self.search.is_some() => self.search_next(),
            Key::Char('N') if self.search.is_some() => self.search_prev(),
            Key::Esc if self.search.is_some() => self.clear_search(),
            Key::Backspace if self.can_go_back() => {
                self.back();
            }
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
        ViewerState::new(parse(md).blocks, width, height, None)
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

    #[test]
    fn search_finds_counts_and_activates() {
        let mut s = state(
            "alpha\n\nbeta target\n\ngamma\n\ndelta target\n\nend",
            80,
            4,
        );
        let n = s.run_search("target");
        assert_eq!(n, 2);
        assert_eq!(s.search.as_ref().unwrap().position(), 1);
    }

    #[test]
    fn n_and_shift_n_cycle_via_key() {
        let mut s = state("x\n\ntarget\n\ny\n\ntarget", 80, 2);
        s.run_search("target");
        let p1 = s.search.as_ref().unwrap().position();
        assert_eq!(s.on_key(Key::Char('n')), Action::Redraw);
        let p2 = s.search.as_ref().unwrap().position();
        assert_ne!(p1, p2);
        s.on_key(Key::Char('N'));
        assert_eq!(s.search.as_ref().unwrap().position(), p1);
    }

    #[test]
    fn esc_clears_search() {
        let mut s = state("has target here", 80, 4);
        s.run_search("target");
        assert!(s.search.is_some());
        assert_eq!(s.on_key(Key::Esc), Action::Redraw);
        assert!(s.search.is_none());
    }

    #[test]
    fn n_ignored_without_active_search() {
        let mut s = state("hello", 80, 4);
        assert_eq!(s.on_key(Key::Char('n')), Action::Ignore);
        assert_eq!(s.on_key(Key::Esc), Action::Ignore);
    }

    #[test]
    fn empty_query_clears_search() {
        let mut s = state("has target", 80, 4);
        s.run_search("target");
        assert_eq!(s.run_search(""), 0);
        assert!(s.search.is_none());
    }
}

//! Slide mode (`-s`): present a document one "slide" at a time (Phase 4).
//!
//! Slides are split on top-level thematic breaks (`---`). The splitter and the navigation state are
//! pure and unit-tested; the event loop that paints one slide at a time lives in `view::app`.

use crate::md::parse::Block;

/// Split a block list into slides on [`Block::ThematicBreak`] (the `---` separators are dropped).
/// Empty slides (e.g. a leading `---`) are removed; the result always has at least one slide.
pub fn split_slides(blocks: &[Block]) -> Vec<Vec<Block>> {
    let mut slides: Vec<Vec<Block>> = Vec::new();
    let mut current: Vec<Block> = Vec::new();
    for b in blocks {
        if matches!(b, Block::ThematicBreak) {
            slides.push(std::mem::take(&mut current));
        } else {
            current.push(b.clone());
        }
    }
    slides.push(current);
    slides.retain(|s| !s.is_empty());
    if slides.is_empty() {
        slides.push(Vec::new());
    }
    slides
}

/// Slide-deck navigation: a non-empty list of slides + the active index (always in range).
pub struct Slides {
    slides: Vec<Vec<Block>>,
    index: usize,
}

impl Slides {
    pub fn new(slides: Vec<Vec<Block>>) -> Self {
        debug_assert!(!slides.is_empty(), "a deck must have at least one slide");
        Slides { slides, index: 0 }
    }

    pub fn len(&self) -> usize {
        self.slides.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slides.is_empty()
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn current(&self) -> &[Block] {
        &self.slides[self.index]
    }

    /// Advance / retreat one slide, clamped at the ends (no wrap — a deck has a first and last).
    pub fn next(&mut self) {
        if self.index + 1 < self.slides.len() {
            self.index += 1;
        }
    }

    pub fn prev(&mut self) {
        self.index = self.index.saturating_sub(1);
    }

    pub fn first(&mut self) {
        self.index = 0;
    }

    pub fn last(&mut self) {
        self.index = self.slides.len() - 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::md::parse::parse;

    fn deck(md: &str) -> Vec<Vec<Block>> {
        split_slides(&parse(md).blocks)
    }

    #[test]
    fn splits_on_thematic_breaks() {
        let d = deck("# One\n\nintro\n\n---\n\n# Two\n\n---\n\n# Three");
        assert_eq!(d.len(), 3);
        // Breaks are dropped; each slide keeps its own blocks.
        assert!(matches!(d[0][0], Block::Heading { .. }));
        assert_eq!(d[0].len(), 2); // heading + paragraph
        assert_eq!(d[1].len(), 1);
        assert_eq!(d[2].len(), 1);
    }

    #[test]
    fn no_breaks_is_one_slide() {
        let d = deck("# Only\n\nbody");
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn leading_and_trailing_breaks_dont_make_empty_slides() {
        let d = deck("---\n\n# A\n\n---");
        assert_eq!(d.len(), 1);
        assert!(matches!(d[0][0], Block::Heading { .. }));
    }

    #[test]
    fn navigation_is_clamped() {
        let mut s = Slides::new(deck("# A\n\n---\n\n# B\n\n---\n\n# C"));
        assert_eq!(s.index(), 0);
        s.prev(); // clamped at start
        assert_eq!(s.index(), 0);
        s.next();
        assert_eq!(s.index(), 1);
        s.next();
        s.next(); // clamped at end (only 3 slides)
        assert_eq!(s.index(), 2);
        s.first();
        assert_eq!(s.index(), 0);
        s.last();
        assert_eq!(s.index(), 2);
    }
}

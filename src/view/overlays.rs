//! Overlays — full-screen pickers layered over the document. `Toc` is the table-of-contents
//! (`o`): a selectable, depth-indented list of the document's headings; Enter jumps to one.
//!
//! Overlays are pure: they own a list + a selection cursor and render themselves to [`Line`]s.
//! The event loop routes keys to them and, on confirm, reads `selected_line` to scroll the
//! document. This keeps selection/rendering unit-testable without a terminal.

use crate::fuzzy;
use crate::md::layout::Heading;
use crate::style::{Line, Span, Style};

/// Render `headings[order]` as depth-indented styled lines, the row at `selected` reverse-
/// highlighted and padded to `width`. Shared by `Toc` and `Fuzzy`.
fn heading_lines(
    headings: &[Heading],
    order: &[usize],
    selected: usize,
    width: usize,
) -> Vec<Line> {
    order
        .iter()
        .enumerate()
        .map(|(row, &i)| {
            let h = &headings[i];
            let indent = "  ".repeat(usize::from(h.depth.saturating_sub(1)));
            let mut text = format!("{indent}{}", h.text);
            if row == selected {
                let w = crate::text::width(&text);
                if w < width {
                    text.push_str(&" ".repeat(width - w));
                }
            }
            let style = Style {
                highlight: row == selected,
                ..Default::default()
            };
            Line {
                spans: vec![Span::new(text, style)],
                no_wrap: true,
            }
        })
        .collect()
}

/// A `height`-row window of `all` that keeps `selected` visible.
fn window(all: Vec<Line>, selected: usize, height: usize) -> Vec<Line> {
    if all.len() <= height {
        return all;
    }
    let off = selected.saturating_sub(height / 2).min(all.len() - height);
    all[off..off + height].to_vec()
}

/// The table-of-contents picker.
pub struct Toc {
    headings: Vec<Heading>,
    selected: usize,
}

impl Toc {
    pub fn new(headings: &[Heading]) -> Self {
        Toc {
            headings: headings.to_vec(),
            selected: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.headings.is_empty()
    }

    pub fn len(&self) -> usize {
        self.headings.len()
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// Move the selection up / down, clamped.
    pub fn up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn down(&mut self) {
        if self.selected + 1 < self.headings.len() {
            self.selected += 1;
        }
    }

    /// The document line of the currently selected heading (to scroll to on Enter).
    pub fn selected_line(&self) -> Option<usize> {
        self.headings.get(self.selected).map(|h| h.line)
    }

    /// All entries as styled lines: depth-indented, the selected row highlighted.
    pub fn lines(&self, width: usize) -> Vec<Line> {
        let order: Vec<usize> = (0..self.headings.len()).collect();
        heading_lines(&self.headings, &order, self.selected, width)
    }

    /// A `height`-row window of the list that keeps the selection visible.
    pub fn view(&self, width: usize, height: usize) -> Vec<Line> {
        window(self.lines(width), self.selected, height)
    }
}

/// The fuzzy heading filter (`:`): a `Toc` with a live query that filters + ranks headings by
/// [`fuzzy::score`]. Typed characters build the query; arrows move the selection among results.
pub struct Fuzzy {
    headings: Vec<Heading>,
    pub query: String,
    /// Indices into `headings`, best match first (all, in order, when the query is empty).
    filtered: Vec<usize>,
    selected: usize,
}

impl Fuzzy {
    pub fn new(headings: &[Heading]) -> Self {
        Fuzzy {
            headings: headings.to_vec(),
            query: String::new(),
            filtered: (0..headings.len()).collect(),
            selected: 0,
        }
    }

    pub fn push(&mut self, c: char) {
        self.query.push(c);
        self.refilter();
    }

    pub fn pop(&mut self) {
        self.query.pop();
        self.refilter();
    }

    fn refilter(&mut self) {
        if self.query.is_empty() {
            self.filtered = (0..self.headings.len()).collect();
        } else {
            let mut scored: Vec<(i32, usize)> = self
                .headings
                .iter()
                .enumerate()
                .filter_map(|(i, h)| fuzzy::score(&self.query, &h.text).map(|s| (s, i)))
                .collect();
            // Best score first; ties keep document order.
            scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            self.filtered = scored.into_iter().map(|(_, i)| i).collect();
        }
        self.selected = 0;
    }

    pub fn count(&self) -> usize {
        self.filtered.len()
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn down(&mut self) {
        if self.selected + 1 < self.filtered.len() {
            self.selected += 1;
        }
    }

    pub fn selected_line(&self) -> Option<usize> {
        self.filtered
            .get(self.selected)
            .map(|&i| self.headings[i].line)
    }

    pub fn view(&self, width: usize, height: usize) -> Vec<Line> {
        window(
            heading_lines(&self.headings, &self.filtered, self.selected, width),
            self.selected,
            height,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::md::layout::layout_document;
    use crate::md::parse::parse;

    fn toc(md: &str) -> Toc {
        Toc::new(&layout_document(&parse(md).blocks, 80).headings)
    }

    #[test]
    fn lists_headings_in_order() {
        let t = toc("# One\n\nx\n\n## Two\n\n### Three");
        assert_eq!(t.len(), 3);
        assert_eq!(t.selected_index(), 0);
    }

    #[test]
    fn selection_moves_and_clamps() {
        let mut t = toc("# a\n\n## b\n\n## c");
        t.up(); // already at top
        assert_eq!(t.selected_index(), 0);
        t.down();
        t.down();
        assert_eq!(t.selected_index(), 2);
        t.down(); // clamp at bottom
        assert_eq!(t.selected_index(), 2);
    }

    #[test]
    fn selected_line_matches_heading() {
        let doc = layout_document(&parse("# a\n\nbody\n\n## b").blocks, 80);
        let mut t = Toc::new(&doc.headings);
        assert_eq!(t.selected_line(), Some(doc.headings[0].line));
        t.down();
        assert_eq!(t.selected_line(), Some(doc.headings[1].line));
    }

    #[test]
    fn empty_when_no_headings() {
        let t = toc("just a paragraph, no headings");
        assert!(t.is_empty());
        assert_eq!(t.selected_line(), None);
    }

    #[test]
    fn renders_indented_with_selected_highlighted() {
        let t = toc("# top\n\n## nested");
        let lines = t.lines(40);
        assert_eq!(lines.len(), 2);
        // depth-1 heading: no indent; depth-2: two-space indent
        assert!(lines[0].plain_text().starts_with("top"));
        assert!(lines[1].plain_text().starts_with("  nested"));
        // selected (row 0) is highlighted, others not
        assert!(lines[0].spans[0].style.highlight);
        assert!(!lines[1].spans[0].style.highlight);
    }

    #[test]
    fn view_windows_long_lists_keeping_selection() {
        let md: String = (0..40).map(|i| format!("# h{i}\n\n")).collect();
        let mut t = toc(&md);
        for _ in 0..30 {
            t.down();
        }
        let v = t.view(40, 10);
        assert_eq!(v.len(), 10);
        // the selected heading (h30) is within the window
        assert!(v.iter().any(|l| l.plain_text().trim() == "h30"));
    }

    fn fz(md: &str) -> Fuzzy {
        Fuzzy::new(&layout_document(&parse(md).blocks, 80).headings)
    }

    #[test]
    fn fuzzy_empty_query_shows_all() {
        let f = fz("# Installation\n\n## Config\n\n## Contributing");
        assert_eq!(f.count(), 3);
    }

    #[test]
    fn fuzzy_filters_by_subsequence() {
        let mut f = fz("# Installation\n\n## Config\n\n## Contributing");
        for c in "con".chars() {
            f.push(c);
        }
        // "con": Config + Contributing match; Installation has no 'c'
        assert_eq!(f.count(), 2);
        assert!(f.selected_line().is_some());
    }

    #[test]
    fn fuzzy_no_match_is_empty_and_pop_restores() {
        let mut f = fz("# Alpha\n\n## Beta");
        f.push('z');
        f.push('z');
        assert_eq!(f.count(), 0);
        assert_eq!(f.selected_line(), None);
        f.pop();
        f.pop();
        assert_eq!(f.count(), 2); // empty query → all again
    }
}

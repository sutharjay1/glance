//! In-document search over the plain text of a laid-out document.
//!
//! `Search` compiles the query as a regex (falling back to a literal match if it isn't valid
//! regex), collects every match across the document's lines, and cycles through them with
//! `next`/`prev`. It is pure — it holds match positions (line + byte range), and the viewer
//! decides how to scroll to and highlight them. Keeping it free of state makes it fully testable.

use regex::Regex;

/// A single match: the line index and the byte range within that line's plain text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

/// A completed search: the query and all its matches, with a cursor into them.
#[derive(Debug, Clone)]
pub struct Search {
    pub query: String,
    matches: Vec<Match>,
    current: usize,
}

impl Search {
    /// Run `query` against `lines` (plain text per line). Empty and zero-width matches are
    /// skipped so cycling always advances.
    pub fn new(query: &str, lines: &[String]) -> Self {
        let mut matches = Vec::new();
        if let Some(re) = compile(query) {
            for (i, line) in lines.iter().enumerate() {
                for m in re.find_iter(line) {
                    if m.start() != m.end() {
                        matches.push(Match {
                            line: i,
                            start: m.start(),
                            end: m.end(),
                        });
                    }
                }
            }
        }
        Search {
            query: query.to_string(),
            matches,
            current: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    pub fn len(&self) -> usize {
        self.matches.len()
    }

    /// 1-based position of the current match, for a `3/12` status readout (0 if none).
    pub fn position(&self) -> usize {
        if self.matches.is_empty() {
            0
        } else {
            self.current + 1
        }
    }

    /// Advance to the next match (wrapping).
    pub fn next(&mut self) {
        if !self.matches.is_empty() {
            self.current = (self.current + 1) % self.matches.len();
        }
    }

    /// Step to the previous match (wrapping).
    pub fn prev(&mut self) {
        if !self.matches.is_empty() {
            self.current = (self.current + self.matches.len() - 1) % self.matches.len();
        }
    }

    pub fn current(&self) -> Option<Match> {
        self.matches.get(self.current).copied()
    }

    /// All matches that fall on `line` (for highlighting a rendered row).
    pub fn on_line(&self, line: usize) -> impl Iterator<Item = &Match> {
        self.matches.iter().filter(move |m| m.line == line)
    }
}

/// Compile `query` as a regex, falling back to a literal (escaped) match if it isn't valid regex.
/// An empty query matches nothing.
fn compile(query: &str) -> Option<Regex> {
    if query.is_empty() {
        return None;
    }
    Regex::new(query)
        .ok()
        .or_else(|| Regex::new(&regex::escape(query)).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn literal_matches_across_lines() {
        let ls = lines(&["the cat sat", "on the mat", "no match here"]);
        let s = Search::new("the", &ls);
        assert_eq!(s.len(), 2); // line 0 and line 1
        assert_eq!(s.current().unwrap().line, 0);
    }

    #[test]
    fn match_positions_are_correct() {
        let ls = lines(&["hello world hello"]);
        let s = Search::new("hello", &ls);
        assert_eq!(s.len(), 2);
        assert_eq!(
            s.current(),
            Some(Match {
                line: 0,
                start: 0,
                end: 5
            })
        );
        assert_eq!(s.on_line(0).count(), 2);
    }

    #[test]
    fn regex_supported() {
        let ls = lines(&["item 1", "item 22", "nope"]);
        let s = Search::new(r"item \d+", &ls);
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn invalid_regex_falls_back_to_literal() {
        // "a(" is invalid regex; must match the literal text "a(".
        let ls = lines(&["call a(b)", "x"]);
        let s = Search::new("a(", &ls);
        assert_eq!(s.len(), 1);
        assert_eq!(
            s.current(),
            Some(Match {
                line: 0,
                start: 5,
                end: 7
            })
        );
    }

    #[test]
    fn cycle_wraps_both_ways() {
        let ls = lines(&["a", "a", "a"]);
        let mut s = Search::new("a", &ls);
        assert_eq!(s.position(), 1);
        s.next();
        assert_eq!(s.position(), 2);
        s.next();
        s.next(); // wrap 3 → 1
        assert_eq!(s.position(), 1);
        s.prev(); // wrap 1 → 3
        assert_eq!(s.position(), 3);
    }

    #[test]
    fn no_matches_is_empty() {
        let s = Search::new("zzz", &lines(&["abc", "def"]));
        assert!(s.is_empty());
        assert_eq!(s.position(), 0);
        assert_eq!(s.current(), None);
    }

    #[test]
    fn empty_query_matches_nothing() {
        assert!(Search::new("", &lines(&["anything"])).is_empty());
    }
}

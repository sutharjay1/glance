//! A tiny fuzzy matcher for the heading filter (`:`) and, later, the command palette.
//!
//! [`score`] returns `Some(score)` when `query` is a case-insensitive subsequence of `text`,
//! `None` otherwise. The score rewards matches that start a word, that are contiguous, and that
//! occur early — so "the good stuff" ranks a tight prefix match above a scattered one. It's
//! deliberately simple (not a full Smith-Waterman), which is plenty for heading lists.

/// Fuzzy-score `text` against `query`. Higher is better; `None` means no subsequence match.
/// An empty query matches everything with score 0.
pub fn score(query: &str, text: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    let q: Vec<char> = query.to_lowercase().chars().collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();

    let mut qi = 0;
    let mut total = 0i32;
    let mut last: Option<usize> = None;
    for (ti, &tc) in t.iter().enumerate() {
        if qi >= q.len() {
            break;
        }
        if tc == q[qi] {
            total += 1;
            if last == Some(ti.wrapping_sub(1)) {
                total += 5; // contiguous run
            } else if ti == 0 {
                total += 4; // matches the very start
            } else if !t[ti - 1].is_alphanumeric() {
                total += 3; // start of a word
            }
            last = Some(ti);
            qi += 1;
        }
    }
    (qi == q.len()).then_some(total)
}

/// Does `text` fuzzy-match `query` at all?
pub fn matches(query: &str, text: &str) -> bool {
    score(query, text).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_matches_all() {
        assert_eq!(score("", "anything"), Some(0));
    }

    #[test]
    fn subsequence_matches_non_contiguous() {
        assert!(matches("gst", "good stuff there")); // g..s..t
        assert!(score("gst", "good stuff there").is_some());
    }

    #[test]
    fn non_subsequence_is_none() {
        assert_eq!(score("xyz", "good stuff"), None);
        assert_eq!(score("tsg", "good stuff"), None); // wrong order
    }

    #[test]
    fn case_insensitive() {
        assert!(matches("INSTALL", "installation guide"));
        assert!(matches("api", "The API Reference"));
    }

    #[test]
    fn contiguous_beats_scattered() {
        // "arch" as a solid substring should outscore the same chars scattered.
        let tight = score("arch", "architecture").unwrap();
        let loose = score("arch", "a really cool huge").unwrap();
        assert!(tight > loose, "tight {tight} !> loose {loose}");
    }

    #[test]
    fn word_start_bonus() {
        // matching at a word boundary scores higher than mid-word.
        let boundary = score("core", "the core module").unwrap();
        let midword = score("ore", "the core module").unwrap();
        assert!(boundary >= midword);
    }
}

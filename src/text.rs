//! Text measurement primitives.
//!
//! `width` is the layout hotspot (plan §8): called once per token during wrapping, a
//! naive Unicode-width lookup dominates layout cost (~98% in profiling of the reference's
//! approach). The ASCII fast path skips the Unicode table for the common case — English
//! prose and source code — where display width equals byte length.

use unicode_width::UnicodeWidthStr;

/// Display width of `s` in terminal cells.
///
/// Pure-ASCII strings return byte length directly (each printable ASCII byte is one cell),
/// avoiding a Unicode-width table lookup. Non-ASCII falls back to `unicode-width`, which
/// accounts for wide (CJK), zero-width (combining), and emoji characters.
///
/// Document text is sanitized of C0 control bytes (except `\n`/`\t`) at parse time, so the
/// fast path never sees control characters at width-measurement time.
#[inline]
pub fn width(s: &str) -> usize {
    if s.is_ascii() {
        s.len()
    } else {
        UnicodeWidthStr::width(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_is_byte_length() {
        assert_eq!(width(""), 0);
        assert_eq!(width("hello"), 5);
        assert_eq!(width("fn main() {}"), 12);
    }

    #[test]
    fn cjk_is_double_width() {
        assert_eq!(width("中文"), 4); // two wide chars, 2 cells each
        assert_eq!(width("a中b"), 4); // 1 + 2 + 1
    }

    #[test]
    fn combining_marks_are_zero_width() {
        // "e" + combining acute accent renders as one visible cell.
        assert_eq!(width("e\u{0301}"), 1);
    }

    #[test]
    fn fast_path_agrees_with_unicode_path_on_ascii() {
        for s in ["", "x", "The quick brown fox", "| a | b |", "```rust"] {
            assert_eq!(width(s), UnicodeWidthStr::width(s), "mismatch on {s:?}");
        }
    }
}

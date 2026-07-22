//! The styled-line model: the output of `md::layout` and the input to `paint`.
//!
//! Layout produces `Line`s of `Span`s carrying *semantic* style (bold, a role, an optional
//! link href) — never raw ANSI. `paint` turns that into escape sequences against the active
//! theme + terminal color depth, so a theme or depth change re-paints without re-laying-out.

/// Semantic role of a span. `paint` maps role + emphasis flags to concrete colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Role {
    #[default]
    Body,
    Heading,
    /// List bullet / number / task box.
    Marker,
    /// Blockquote bar and callout furniture.
    Accent,
    /// Thematic break rule.
    Rule,
    /// De-emphasized text (image alt, ` (url)` suffixes).
    Dim,
    // Code-token roles (set by `md::highlight`).
    Keyword,
    Str,
    Comment,
    Number,
    Function,
}

/// Semantic style of a span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
    pub role: Role,
}

impl Style {
    pub fn role(role: Role) -> Self {
        Style {
            role,
            ..Default::default()
        }
    }
}

/// A run of text sharing one style (and optional link target).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub style: Style,
    pub href: Option<String>,
}

impl Span {
    pub fn new(text: impl Into<String>, style: Style) -> Self {
        Span {
            text: text.into(),
            style,
            href: None,
        }
    }

    pub fn plain(text: impl Into<String>) -> Self {
        Span::new(text, Style::default())
    }
}

/// One laid-out display line. `no_wrap` lines (code, tables, rules) are never re-wrapped and
/// suppress ` (url)` link suffixes so column alignment is preserved.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Line {
    pub spans: Vec<Span>,
    pub no_wrap: bool,
}

impl Line {
    /// Total display width in cells.
    pub fn width(&self) -> usize {
        self.spans.iter().map(|s| crate::text::width(&s.text)).sum()
    }

    /// Concatenated plain text (for search and hit-testing).
    pub fn plain_text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }
}

//! Terminal capability detection.
//!
//! glance authors every theme in truecolor and downsamples to whatever the terminal supports
//! (plan §4.5). This module answers "how much color?" by reading the standard environment
//! signals. Detection logic is a pure function (`detect_color_depth`) so it is exhaustively
//! unit-testable; `Capabilities::from_env` is the thin wrapper that reads the real process
//! environment.

/// How much color the terminal can render. Ordered least → most capable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorDepth {
    /// No color: `NO_COLOR` set, a `dumb`/absent terminal, or `--no-color`.
    None,
    /// The 16 basic ANSI colors.
    Ansi16,
    /// The 256-color palette.
    Ansi256,
    /// 24-bit truecolor.
    TrueColor,
}

/// Decide the color depth from the relevant environment signals.
///
/// Precedence, matching common terminal convention:
/// 1. `no_color` (from `NO_COLOR` or `--no-color`) forces [`ColorDepth::None`].
/// 2. `COLORTERM` = `truecolor`/`24bit` ⇒ [`ColorDepth::TrueColor`] (set by modern emulators).
/// 3. `TERM`: `dumb` ⇒ `None`; contains `truecolor` ⇒ `TrueColor`; contains `256color` ⇒
///    `Ansi256`; any other non-empty value ⇒ `Ansi16`.
/// 4. No `TERM` at all ⇒ `None` (not an interactive terminal).
///
/// Matching is case-insensitive. Taking the signals as parameters keeps this pure and testable.
pub fn detect_color_depth(
    no_color: bool,
    colorterm: Option<&str>,
    term: Option<&str>,
) -> ColorDepth {
    if no_color {
        return ColorDepth::None;
    }
    if let Some(ct) = colorterm {
        let ct = ct.to_ascii_lowercase();
        if ct.contains("truecolor") || ct.contains("24bit") {
            return ColorDepth::TrueColor;
        }
    }
    match term {
        Some(t) if t.eq_ignore_ascii_case("dumb") => ColorDepth::None,
        Some(t) => {
            let t = t.to_ascii_lowercase();
            if t.contains("truecolor") {
                ColorDepth::TrueColor
            } else if t.contains("256color") {
                ColorDepth::Ansi256
            } else {
                ColorDepth::Ansi16
            }
        }
        None => ColorDepth::None,
    }
}

/// Detected terminal capabilities. Grows over Phase 1 (OSC 8/52 support, cell pixel size,
/// synchronized-output support); for now it carries the color depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    pub color: ColorDepth,
}

impl Capabilities {
    /// Read capabilities from the real process environment. `force_no_color` lets the caller
    /// fold in a `--no-color` CLI flag on top of the `NO_COLOR` env convention.
    pub fn from_env(force_no_color: bool) -> Self {
        let no_color = force_no_color || std::env::var_os("NO_COLOR").is_some();
        let colorterm = std::env::var("COLORTERM").ok();
        let term = std::env::var("TERM").ok();
        Capabilities {
            color: detect_color_depth(no_color, colorterm.as_deref(), term.as_deref()),
        }
    }

    /// True when the terminal can render 24-bit color.
    pub fn is_truecolor(&self) -> bool {
        self.color == ColorDepth::TrueColor
    }
}

#[cfg(test)]
mod tests {
    use super::ColorDepth::{Ansi16, Ansi256, TrueColor};
    use super::*;

    #[test]
    fn no_color_always_wins() {
        assert_eq!(
            detect_color_depth(true, Some("truecolor"), Some("xterm-256color")),
            ColorDepth::None
        );
    }

    #[test]
    fn colorterm_signals_truecolor() {
        assert_eq!(
            detect_color_depth(false, Some("truecolor"), Some("xterm")),
            TrueColor
        );
        assert_eq!(detect_color_depth(false, Some("24bit"), None), TrueColor);
        assert_eq!(
            detect_color_depth(false, Some("TrueColor"), None),
            TrueColor
        ); // case-insensitive
    }

    #[test]
    fn colorterm_truecolor_overrides_dumb_term() {
        assert_eq!(
            detect_color_depth(false, Some("truecolor"), Some("dumb")),
            TrueColor
        );
    }

    #[test]
    fn term_256color_variants() {
        assert_eq!(
            detect_color_depth(false, None, Some("xterm-256color")),
            Ansi256
        );
        assert_eq!(
            detect_color_depth(false, None, Some("screen-256color")),
            Ansi256
        );
    }

    #[test]
    fn term_truecolor_variant() {
        assert_eq!(
            detect_color_depth(false, None, Some("xterm-truecolor")),
            TrueColor
        );
    }

    #[test]
    fn plain_term_is_ansi16() {
        assert_eq!(detect_color_depth(false, None, Some("xterm")), Ansi16);
        assert_eq!(detect_color_depth(false, None, Some("screen")), Ansi16);
    }

    #[test]
    fn dumb_or_absent_term_has_no_color() {
        assert_eq!(
            detect_color_depth(false, None, Some("dumb")),
            ColorDepth::None
        );
        assert_eq!(detect_color_depth(false, None, None), ColorDepth::None);
    }

    #[test]
    fn is_truecolor_helper() {
        assert!(Capabilities { color: TrueColor }.is_truecolor());
        assert!(!Capabilities { color: Ansi256 }.is_truecolor());
    }
}

//! Themes: semantic role → truecolor. Authored once in 24-bit; `term::ansi` downsamples to the
//! terminal's depth at paint time, so a theme is just a color table (ADR 0004).

use crate::term::ansi::Rgb;

/// A color for each semantic slot `paint` needs.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub body: Rgb,
    pub heading: Rgb,
    /// List markers, quote bars, callout furniture. glance's brand accent.
    pub accent: Rgb,
    pub dim: Rgb,
    pub code: Rgb,
    pub link: Rgb,
    pub rule: Rgb,
}

/// The dark theme. Accent is glance's brand orange `#FF5800`; text/support colors are
/// Catppuccin-Mocha-adjacent for a calm base (see ADR 0001 / parity notes).
pub fn dark() -> Theme {
    Theme {
        body: Rgb::new(0xCD, 0xD6, 0xF4),
        heading: Rgb::new(0xFF, 0x58, 0x00),
        accent: Rgb::new(0xFF, 0x58, 0x00),
        dim: Rgb::new(0x6C, 0x70, 0x86),
        code: Rgb::new(0xA6, 0xE3, 0xA1),
        link: Rgb::new(0x89, 0xB4, 0xFA),
        rule: Rgb::new(0x45, 0x47, 0x5A),
    }
}

/// The light theme. Same brand accent on a light base.
pub fn light() -> Theme {
    Theme {
        body: Rgb::new(0x24, 0x29, 0x2E),
        heading: Rgb::new(0xFF, 0x58, 0x00),
        accent: Rgb::new(0xFF, 0x58, 0x00),
        dim: Rgb::new(0x6A, 0x73, 0x7D),
        code: Rgb::new(0x22, 0x86, 0x3A),
        link: Rgb::new(0x03, 0x66, 0xD6),
        rule: Rgb::new(0xD1, 0xD5, 0xDA),
    }
}

/// Look up a theme by name (`dark`/`light`), defaulting to dark.
pub fn by_name(name: &str) -> Theme {
    match name {
        "light" => light(),
        _ => dark(),
    }
}

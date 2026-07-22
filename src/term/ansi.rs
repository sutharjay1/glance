//! ANSI color + SGR building with color downsampling.
//!
//! glance authors themes in 24-bit truecolor (ADR 0004). Terminals that can't render
//! truecolor get the color *downsampled* to their [`ColorDepth`] here — to the 256-color
//! palette (xterm 6×6×6 cube + grayscale ramp) or the 16 basic ANSI colors — so a single
//! theme definition renders everywhere. The downsampling is pure and exhaustively tested;
//! `fg`/`bg` turn a color + depth into SGR parameters.

use crate::term::caps::ColorDepth;

/// A 24-bit RGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb { r, g, b }
    }
}

/// The SGR reset sequence (`ESC[0m`).
pub const RESET: &str = "\x1b[0m";

// --- Screen / cursor operations -------------------------------------------

/// Begin synchronized output (CSI ?2026h): the terminal buffers until [`SYNC_END`], so a frame
/// appears atomically with no tearing.
pub const SYNC_BEGIN: &str = "\x1b[?2026h";
/// End synchronized output.
pub const SYNC_END: &str = "\x1b[?2026l";
/// Clear from the cursor to the end of the line.
pub const CLEAR_LINE_EOL: &str = "\x1b[K";
/// Clear the whole screen.
pub const CLEAR_SCREEN: &str = "\x1b[2J";
/// Enter / leave the alternate screen buffer.
pub const ENTER_ALT: &str = "\x1b[?1049h";
pub const LEAVE_ALT: &str = "\x1b[?1049l";
/// Hide / show the cursor.
pub const HIDE_CURSOR: &str = "\x1b[?25l";
pub const SHOW_CURSOR: &str = "\x1b[?25h";

/// Move the cursor to `(row, col)`, both 0-based (emitted 1-based per the CSI convention).
pub fn move_to(row: usize, col: usize) -> String {
    format!("\x1b[{};{}H", row + 1, col + 1)
}

/// Wrap SGR parameters in an escape sequence, or return empty for empty params.
pub fn sgr(params: &str) -> String {
    if params.is_empty() {
        String::new()
    } else {
        format!("\x1b[{params}m")
    }
}

/// SGR parameters that set `color` as the **foreground**, downsampled to `depth`.
/// Returns an empty string for [`ColorDepth::None`].
pub fn fg(color: Rgb, depth: ColorDepth) -> String {
    match depth {
        ColorDepth::None => String::new(),
        ColorDepth::TrueColor => format!("38;2;{};{};{}", color.r, color.g, color.b),
        ColorDepth::Ansi256 => format!("38;5;{}", to_ansi256(color)),
        ColorDepth::Ansi16 => {
            let i = to_ansi16(color);
            // 0–7 → 30–37, 8–15 → 90–97.
            let code = if i < 8 { 30 + i } else { 90 + (i - 8) };
            code.to_string()
        }
    }
}

/// SGR parameters that set `color` as the **background**, downsampled to `depth`.
/// Returns an empty string for [`ColorDepth::None`].
pub fn bg(color: Rgb, depth: ColorDepth) -> String {
    match depth {
        ColorDepth::None => String::new(),
        ColorDepth::TrueColor => format!("48;2;{};{};{}", color.r, color.g, color.b),
        ColorDepth::Ansi256 => format!("48;5;{}", to_ansi256(color)),
        ColorDepth::Ansi16 => {
            let i = to_ansi16(color);
            // 0–7 → 40–47, 8–15 → 100–107.
            let code = if i < 8 { 40 + i } else { 100 + (i - 8) };
            code.to_string()
        }
    }
}

// --- Downsampling ---------------------------------------------------------

/// The six intensity levels of the xterm 6×6×6 color cube.
const CUBE_LEVELS: [u8; 6] = [0x00, 0x5f, 0x87, 0xaf, 0xd7, 0xff];

fn dist_sq(a: Rgb, r: u8, g: u8, b: u8) -> i32 {
    let dr = a.r as i32 - r as i32;
    let dg = a.g as i32 - g as i32;
    let db = a.b as i32 - b as i32;
    dr * dr + dg * dg + db * db
}

/// Map one channel value to its 6-cube index (0–5).
fn to_6cube(v: u8) -> u8 {
    if v < 48 {
        0
    } else if v < 114 {
        1
    } else {
        (v - 35) / 40
    }
}

/// Downsample to the closest xterm-256 palette index (the canonical tmux algorithm):
/// compare the nearest 6×6×6 cube color against the nearest gray-ramp entry and pick whichever
/// is closer to the original.
pub fn to_ansi256(c: Rgb) -> u8 {
    let (qr, qg, qb) = (to_6cube(c.r), to_6cube(c.g), to_6cube(c.b));
    let (cr, cg, cb) = (
        CUBE_LEVELS[qr as usize],
        CUBE_LEVELS[qg as usize],
        CUBE_LEVELS[qb as usize],
    );
    let cube_idx = 16 + 36 * qr + 6 * qg + qb;
    if cr == c.r && cg == c.g && cb == c.b {
        return cube_idx; // exact cube match
    }
    // Nearest gray-ramp entry (indices 232–255 = gray 8,18,…,238).
    let grey_avg = (c.r as i32 + c.g as i32 + c.b as i32) / 3;
    let grey_idx = if grey_avg > 238 {
        23
    } else {
        (grey_avg - 3) / 10
    };
    let grey = (8 + 10 * grey_idx) as u8;
    if dist_sq(Rgb::new(grey, grey, grey), c.r, c.g, c.b)
        < dist_sq(Rgb::new(cr, cg, cb), c.r, c.g, c.b)
    {
        (232 + grey_idx) as u8
    } else {
        cube_idx
    }
}

/// Standard xterm RGB values for the 16 basic ANSI colors.
const ANSI16: [Rgb; 16] = [
    Rgb::new(0, 0, 0),       // 0 black
    Rgb::new(205, 0, 0),     // 1 red
    Rgb::new(0, 205, 0),     // 2 green
    Rgb::new(205, 205, 0),   // 3 yellow
    Rgb::new(0, 0, 238),     // 4 blue
    Rgb::new(205, 0, 205),   // 5 magenta
    Rgb::new(0, 205, 205),   // 6 cyan
    Rgb::new(229, 229, 229), // 7 white
    Rgb::new(127, 127, 127), // 8 bright black
    Rgb::new(255, 0, 0),     // 9 bright red
    Rgb::new(0, 255, 0),     // 10 bright green
    Rgb::new(255, 255, 0),   // 11 bright yellow
    Rgb::new(92, 92, 255),   // 12 bright blue
    Rgb::new(255, 0, 255),   // 13 bright magenta
    Rgb::new(0, 255, 255),   // 14 bright cyan
    Rgb::new(255, 255, 255), // 15 bright white
];

/// Downsample to the nearest of the 16 basic ANSI colors (returns the palette index 0–15).
pub fn to_ansi16(c: Rgb) -> u8 {
    let mut best = 0u8;
    let mut best_d = i32::MAX;
    for (i, p) in ANSI16.iter().enumerate() {
        let d = dist_sq(*p, c.r, c.g, c.b);
        if d < best_d {
            best_d = d;
            best = i as u8;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::caps::ColorDepth::*;

    const RED: Rgb = Rgb::new(255, 0, 0);
    const GREEN: Rgb = Rgb::new(0, 255, 0);
    const BLUE: Rgb = Rgb::new(0, 0, 255);
    const BLACK: Rgb = Rgb::new(0, 0, 0);
    const WHITE: Rgb = Rgb::new(255, 255, 255);

    #[test]
    fn cube_corners_map_exactly() {
        assert_eq!(to_ansi256(BLACK), 16); // cube origin
        assert_eq!(to_ansi256(WHITE), 231); // cube corner 5,5,5
        assert_eq!(to_ansi256(RED), 196); // 16 + 36*5
        assert_eq!(to_ansi256(GREEN), 46); // 16 + 6*5
        assert_eq!(to_ansi256(BLUE), 21); // 16 + 5
    }

    #[test]
    fn exact_cube_level_is_not_grayed() {
        // 0x5f0000 is an exact cube color → index 52, never the gray ramp.
        assert_eq!(to_ansi256(Rgb::new(0x5f, 0, 0)), 52);
    }

    #[test]
    fn mid_gray_prefers_gray_ramp() {
        // 128,128,128 is closer to gray entry 244 than to the nearest cube color.
        assert_eq!(to_ansi256(Rgb::new(128, 128, 128)), 244);
    }

    #[test]
    fn ansi16_primaries() {
        assert_eq!(to_ansi16(BLACK), 0);
        assert_eq!(to_ansi16(WHITE), 15);
        assert_eq!(to_ansi16(RED), 9); // bright red
        assert_eq!(to_ansi16(GREEN), 10);
        assert_eq!(to_ansi16(Rgb::new(255, 255, 0)), 11); // exact bright yellow
    }

    #[test]
    fn fg_by_depth() {
        assert_eq!(fg(RED, TrueColor), "38;2;255;0;0");
        assert_eq!(fg(RED, Ansi256), "38;5;196");
        assert_eq!(fg(RED, Ansi16), "91"); // bright red → 90 + 1
        assert_eq!(fg(Rgb::new(205, 0, 0), Ansi16), "31"); // dark red → 30 + 1
        assert_eq!(fg(RED, None), "");
    }

    #[test]
    fn bg_by_depth() {
        assert_eq!(bg(BLUE, TrueColor), "48;2;0;0;255");
        assert_eq!(bg(WHITE, Ansi16), "107"); // bright white → 100 + 7
        assert_eq!(bg(BLACK, Ansi16), "40"); // black → 40 + 0
        assert_eq!(bg(RED, None), "");
    }

    #[test]
    fn sgr_wraps_and_skips_empty() {
        assert_eq!(sgr("38;2;255;0;0"), "\x1b[38;2;255;0;0m");
        assert_eq!(sgr(""), "");
        assert_eq!(RESET, "\x1b[0m");
    }
}

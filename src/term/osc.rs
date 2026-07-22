//! OSC (Operating System Command) escape sequences.
//!
//! Three OSCs glance relies on, all fixing reference weaknesses or enabling differentiators:
//! - **OSC 8** — real clickable hyperlinks (feature #5).
//! - **OSC 52** — clipboard write that works over SSH and inside tmux, with no external tool
//!   (fixes weakness #3). Payload is base64; terminals cap it (~100 KB), so `clipboard_within`
//!   lets the caller fall back to a native command when too large.
//! - **OSC 11** — query the terminal background color to auto-pick dark/light (§4.5).
//!
//! Every sequence is framed `ESC ] <code> ; <payload> ST`, with `ST` = `ESC \`.

use crate::term::ansi::Rgb;
use base64::Engine;

/// OSC 11 query for the terminal background color. The terminal replies with an OSC 11
/// response parseable by [`parse_bg_response`].
pub const QUERY_BG: &str = "\x1b]11;?\x1b\\";

/// Conservative cap on the base64 payload of an OSC 52 clipboard write; above this, some
/// terminals silently drop it, so callers should fall back to a native clipboard command.
pub const OSC52_MAX_ENCODED: usize = 100_000;

/// OSC 8 hyperlink: render `text` as a clickable link to `url`.
pub fn hyperlink(url: &str, text: &str) -> String {
    format!("\x1b]8;;{url}\x1b\\{text}\x1b]8;;\x1b\\")
}

fn osc52(encoded: &str) -> String {
    format!("\x1b]52;c;{encoded}\x1b\\")
}

/// OSC 52 clipboard write of `text` (base64-encoded). Prefer [`clipboard_within`] when the
/// text may be large.
pub fn clipboard(text: &str) -> String {
    osc52(&base64::engine::general_purpose::STANDARD.encode(text.as_bytes()))
}

/// OSC 52 clipboard write, or `None` if the base64 payload would exceed `max_encoded`
/// (signal to the caller to use a native clipboard command instead).
pub fn clipboard_within(text: &str, max_encoded: usize) -> Option<String> {
    let encoded = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
    (encoded.len() <= max_encoded).then(|| osc52(&encoded))
}

/// Parse an OSC 11 background-color response, e.g. `ESC]11;rgb:1e1e/1e1e/2e2e ST`.
/// Channels may be 1–4 hex digits and are scaled to 8-bit. Returns `None` on any malformation.
pub fn parse_bg_response(s: &str) -> Option<Rgb> {
    let start = s.find("rgb:")? + 4;
    let rest = &s[start..];
    let end = rest.find(['\x1b', '\x07']).unwrap_or(rest.len());
    let mut parts = rest[..end].split('/');
    let r = parse_channel(parts.next()?)?;
    let g = parse_channel(parts.next()?)?;
    let b = parse_channel(parts.next()?)?;
    if parts.next().is_some() {
        return None; // more than three channels → malformed
    }
    Some(Rgb::new(r, g, b))
}

/// Parse one 1–4 hex-digit channel and scale it to 8 bits.
fn parse_channel(h: &str) -> Option<u8> {
    if h.is_empty() || h.len() > 4 || !h.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    let max = (1u32 << (4 * h.len())) - 1; // 16^len - 1
    Some(((v * 255 + max / 2) / max) as u8)
}

/// Whether a background color reads as "dark" (perceived luminance below mid), used to auto-pick
/// the theme from an OSC 11 response.
pub fn is_dark(bg: Rgb) -> bool {
    let lum = 0.299 * bg.r as f32 + 0.587 * bg.g as f32 + 0.114 * bg.b as f32;
    lum < 128.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osc8_hyperlink_frames_url_and_text() {
        assert_eq!(
            hyperlink("https://example.com", "click"),
            "\x1b]8;;https://example.com\x1b\\click\x1b]8;;\x1b\\"
        );
    }

    #[test]
    fn osc52_clipboard_base64_encodes() {
        // base64("hi") == "aGk="
        assert_eq!(clipboard("hi"), "\x1b]52;c;aGk=\x1b\\");
    }

    #[test]
    fn clipboard_within_caps_large_payloads() {
        assert!(clipboard_within("hi", 100).is_some());
        // ~1.33× base64 expansion; 10 bytes → 16 encoded, over a tiny cap.
        assert!(clipboard_within("0123456789", 8).is_none());
    }

    #[test]
    fn parse_bg_four_digit_channels() {
        // Catppuccin-Mocha background #1E1E2E.
        let rgb = parse_bg_response("\x1b]11;rgb:1e1e/1e1e/2e2e\x1b\\").unwrap();
        assert_eq!(rgb, Rgb::new(0x1e, 0x1e, 0x2e));
    }

    #[test]
    fn parse_bg_two_digit_channels() {
        let rgb = parse_bg_response("\x1b]11;rgb:ff/00/80\x07").unwrap();
        assert_eq!(rgb, Rgb::new(255, 0, 128));
    }

    #[test]
    fn parse_bg_rejects_malformed() {
        assert!(parse_bg_response("no rgb here").is_none());
        assert!(parse_bg_response("rgb:zz/00/00").is_none());
        assert!(parse_bg_response("rgb:ff/00").is_none()); // too few channels
        assert!(parse_bg_response("rgb:ff/00/00/00").is_none()); // too many
    }

    #[test]
    fn is_dark_classifies_backgrounds() {
        assert!(is_dark(Rgb::new(0x1e, 0x1e, 0x2e))); // dark theme bg
        assert!(!is_dark(Rgb::new(255, 255, 255))); // light bg
        assert!(!is_dark(Rgb::new(0xee, 0xee, 0xee)));
    }
}

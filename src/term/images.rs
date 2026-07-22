//! Image rendering for the terminal (Phase 3).
//!
//! The universal path is the **half-block** renderer: it works in any color terminal with no
//! special protocol. Each character cell is a `▀` (upper half block) whose *foreground* is the
//! top sub-pixel and whose *background* is the bottom sub-pixel — so one cell encodes two vertical
//! pixels, doubling vertical resolution. Kitty/iTerm2 graphics protocols (crisper) layer on top as
//! fast paths where the terminal advertises them.
//!
//! Rendering is pure — an image buffer in, styled [`Line`]s out — so it is unit-testable without a
//! terminal. Fetch/decode and the progressive background patch land with the worker (next).

use base64::Engine;
use image::{imageops::FilterType, DynamicImage};

use crate::style::{Line, Span, Style};
use crate::term::ansi::Rgb;

/// The upper-half-block glyph: paints the top half in the foreground color, bottom in background.
const UPPER_HALF: &str = "▀";

/// Max base64 payload per Kitty graphics chunk (the protocol requires ≤4096 encoded bytes/chunk).
const KITTY_CHUNK: usize = 4096;

/// Encode `png` bytes as a Kitty graphics-protocol sequence that transmits **and** displays the
/// image at the cursor (`a=T`, `f=100` = PNG). The base64 payload is split into ≤4096-byte chunks;
/// the first chunk carries the control keys, and every chunk sets `m=1` (more follow) except the
/// last (`m=0`). Pure — the framing is unit-tested without a terminal.
pub fn kitty_png(png: &[u8]) -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(png);
    let bytes = b64.as_bytes();
    if bytes.is_empty() {
        return String::new();
    }
    let chunks: Vec<&[u8]> = bytes.chunks(KITTY_CHUNK).collect();
    let n = chunks.len();
    let mut out = String::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let more = u8::from(i + 1 < n); // 1 while more chunks follow, 0 on the last
        out.push_str("\x1b_G");
        if i == 0 {
            out.push_str(&format!("a=T,f=100,m={more}"));
        } else {
            out.push_str(&format!("m={more}"));
        }
        out.push(';');
        // `chunk` is a slice of valid base64 ASCII, so this is always valid UTF-8.
        out.push_str(std::str::from_utf8(chunk).unwrap_or(""));
        out.push_str("\x1b\\");
    }
    out
}

/// Cell dimensions (`cols` × `rows`) to render `img` into, fitting `max_cols` wide while preserving
/// aspect ratio. Because a half-block cell stacks two sub-pixels vertically, the rendered pixel
/// grid is `cols × (rows*2)`, which keeps sub-pixels roughly square: `rows = cols·H / (2·W)`.
pub fn cell_size(img_w: u32, img_h: u32, max_cols: u32) -> (u32, u32) {
    let cols = max_cols.max(1);
    let rows = ((cols as u64 * img_h.max(1) as u64) / (2 * img_w.max(1) as u64)).max(1) as u32;
    (cols, rows)
}

/// Render `img` into a `cols`×`rows` grid of half-block cells. Each returned [`Line`] is `no_wrap`
/// (image rows must never reflow) and holds `cols` spans, each `▀` with `fg` = top sub-pixel and
/// `bg` = bottom sub-pixel.
pub fn half_block(img: &DynamicImage, cols: u32, rows: u32) -> Vec<Line> {
    if cols == 0 || rows == 0 {
        return Vec::new();
    }
    // One cell wide = one pixel; one cell tall = two pixels.
    let scaled = img
        .resize_exact(cols, rows * 2, FilterType::Triangle)
        .to_rgba8();
    (0..rows)
        .map(|r| {
            let spans = (0..cols)
                .map(|c| {
                    let top = scaled.get_pixel(c, 2 * r);
                    let bottom = scaled.get_pixel(c, 2 * r + 1);
                    Span::new(
                        UPPER_HALF,
                        Style {
                            fg: Some(Rgb::new(top[0], top[1], top[2])),
                            bg: Some(Rgb::new(bottom[0], bottom[1], bottom[2])),
                            ..Default::default()
                        },
                    )
                })
                .collect();
            Line {
                spans,
                no_wrap: true,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn cell_size_preserves_aspect() {
        // Square image → rows = cols/2 (half-block doubles vertical resolution).
        assert_eq!(cell_size(100, 100, 20), (20, 10));
        // Wide image → fewer rows.
        assert_eq!(cell_size(200, 50, 40), (40, 5));
        // Never zero.
        assert_eq!(cell_size(0, 0, 0), (1, 1));
    }

    #[test]
    fn half_block_maps_top_and_bottom_pixels() {
        // A 2×2 image: top row red, bottom row blue.
        let mut img = RgbaImage::new(2, 2);
        img.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        img.put_pixel(1, 0, Rgba([255, 0, 0, 255]));
        img.put_pixel(0, 1, Rgba([0, 0, 255, 255]));
        img.put_pixel(1, 1, Rgba([0, 0, 255, 255]));
        let dynimg = DynamicImage::ImageRgba8(img);

        // Render to exactly 2 cols × 1 row → the 2×2 grid maps 1:1.
        let lines = half_block(&dynimg, 2, 1);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 2);
        let cell = &lines[0].spans[0];
        assert_eq!(cell.text, "▀");
        assert_eq!(cell.style.fg, Some(Rgb::new(255, 0, 0))); // top = red
        assert_eq!(cell.style.bg, Some(Rgb::new(0, 0, 255))); // bottom = blue
        assert!(lines[0].no_wrap);
    }

    #[test]
    fn zero_dimensions_produce_no_lines() {
        let img = DynamicImage::ImageRgba8(RgbaImage::new(4, 4));
        assert!(half_block(&img, 0, 5).is_empty());
        assert!(half_block(&img, 5, 0).is_empty());
    }

    #[test]
    fn kitty_single_chunk_framing() {
        let seq = kitty_png(b"small");
        assert!(seq.starts_with("\x1b_Ga=T,f=100,m=0;")); // control keys + last-chunk marker
        assert!(seq.ends_with("\x1b\\"));
        assert_eq!(seq.matches("\x1b_G").count(), 1); // one chunk
        assert!(kitty_png(b"").is_empty());
    }

    #[test]
    fn kitty_multi_chunk_sets_more_flag() {
        // >3 KiB raw → >4096 base64 chars → at least two chunks.
        let big = vec![0u8; 4000];
        let seq = kitty_png(&big);
        let chunks = seq.matches("\x1b_G").count();
        assert!(chunks >= 2, "expected multiple chunks, got {chunks}");
        assert!(seq.contains("a=T,f=100,m=1")); // first chunk: more follow
        assert!(seq.contains(";").then_some(true).is_some());
        // The last chunk closes with m=0 (a continuation chunk, no control keys).
        assert!(seq.contains("\x1b_Gm=0;"));
        // Every chunk terminates with ST.
        assert_eq!(seq.matches("\x1b\\").count(), chunks);
    }
}

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

use image::{imageops::FilterType, DynamicImage};

use crate::style::{Line, Span, Style};
use crate::term::ansi::Rgb;

/// The upper-half-block glyph: paints the top half in the foreground color, bottom in background.
const UPPER_HALF: &str = "▀";

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
}

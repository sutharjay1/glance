//! Golden snapshot tests: the full render pipeline (parse → layout → highlight → paint) over a
//! curated feature fixture, at the three reference widths. Plain (no-color) goldens are the
//! readable, reviewable contract; one colored golden locks the SGR output.
//!
//! Regenerate after an intentional change with `INSTA_UPDATE=always cargo test --test
//! render_snapshots` (or `cargo insta review`).

use glance::paint::render_document;
use glance::term::caps::ColorDepth;
use glance::theme;

const FEATURES: &str = include_str!("fixtures/features.md");

fn render(width: usize, depth: ColorDepth) -> String {
    render_document(FEATURES, width, &theme::dark(), depth, false)
}

#[test]
fn plain_width_44() {
    insta::assert_snapshot!("features_plain_44", render(44, ColorDepth::None));
}

#[test]
fn plain_width_80() {
    insta::assert_snapshot!("features_plain_80", render(80, ColorDepth::None));
}

#[test]
fn plain_width_120() {
    insta::assert_snapshot!("features_plain_120", render(120, ColorDepth::None));
}

#[test]
fn colored_width_80() {
    insta::assert_snapshot!("features_color_80", render(80, ColorDepth::TrueColor));
}

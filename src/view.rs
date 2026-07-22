//! The interactive viewer: frame rendering (`render`), and — in later iterations — the
//! `ViewerState` and event loop (`app`) plus overlays. Rendering is kept separate and pure so
//! it can be snapshot-tested without a terminal.

pub mod app;
pub mod copy;
pub mod highlighter;
pub mod images;
pub mod overlays;
pub mod render;
pub mod search;
pub mod slides;
pub mod state;
pub mod tabs;
pub mod watch;

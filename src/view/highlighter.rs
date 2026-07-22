//! Background syntax-highlight worker (Phase 3).
//!
//! syntect is accurate but its `SyntaxSet` load is exactly what makes mdterm slow at startup, so
//! we keep it entirely off the UI thread and the first-paint path. This mirrors the auto-reload
//! watcher: a producer thread + two channels + a drain step in the event loop.
//!
//! The main thread posts [`HighlightRequest`]s (visible code blocks first, see
//! [`blocks_by_priority`]); the worker lazily loads the `SyntaxSet` on its *first* highlight call,
//! renders the block to ready-to-patch [`Line`]s (so the layout work also happens off the UI
//! thread), and returns a [`HighlightResult`]. The main loop drains results and patches the tab
//! in place — progressively upgrading each block from the instant micro-tokenizer output.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::md::layout::{self, CodeRef};
use crate::md::syntect_hl;
use crate::style::Line;

/// A request to highlight one code block. `width`/`line_numbers` are captured so a result computed
/// for stale geometry (after a resize or `l` toggle) can be discarded on arrival.
pub struct HighlightRequest {
    pub tab: usize,
    pub block: usize,
    pub content: String,
    pub lang: String,
    pub width: usize,
    pub line_numbers: bool,
}

/// A highlighted block, rendered to display lines ready to patch into the tab's layout.
pub struct HighlightResult {
    pub tab: usize,
    pub block: usize,
    pub lines: Vec<Line>,
    pub width: usize,
    pub line_numbers: bool,
}

/// Handle to the worker thread and its channels. Dropping it closes the request channel, which
/// ends the worker loop.
pub struct Highlighter {
    tx: Sender<HighlightRequest>,
    rx: Receiver<HighlightResult>,
    _handle: JoinHandle<()>,
}

impl Highlighter {
    pub fn spawn() -> Self {
        let (req_tx, req_rx) = mpsc::channel::<HighlightRequest>();
        let (res_tx, res_rx) = mpsc::channel::<HighlightResult>();
        let handle = thread::spawn(move || {
            // The first `highlight` call lazily loads the SyntaxSet — here, off the UI thread and
            // off the startup path. The loop ends when the request channel closes (Highlighter
            // dropped) or the result channel breaks (main loop gone).
            for req in req_rx {
                if let Some(rows) = syntect_hl::highlight(&req.content, &req.lang) {
                    let lines = layout::layout_code_with(rows, req.width, req.line_numbers);
                    let sent = res_tx.send(HighlightResult {
                        tab: req.tab,
                        block: req.block,
                        lines,
                        width: req.width,
                        line_numbers: req.line_numbers,
                    });
                    if sent.is_err() {
                        break;
                    }
                }
            }
        });
        Highlighter {
            tx: req_tx,
            rx: res_rx,
            _handle: handle,
        }
    }

    pub fn request(&self, req: HighlightRequest) {
        let _ = self.tx.send(req);
    }

    /// Drain all ready highlight results without blocking.
    pub fn drain(&self) -> impl Iterator<Item = HighlightResult> + '_ {
        self.rx.try_iter()
    }
}

/// Order code-block indices so on-screen blocks come first (each group in document order), so the
/// worker highlights what the user is looking at before scrolling ahead. A block `[start, end)` is
/// visible when it intersects the viewport `[top, top + height)`.
pub fn blocks_by_priority(blocks: &[CodeRef], top: usize, height: usize) -> Vec<usize> {
    let bottom = top + height;
    let mut visible = Vec::new();
    let mut rest = Vec::new();
    for (i, b) in blocks.iter().enumerate() {
        if b.start < bottom && b.end > top {
            visible.push(i);
        } else {
            rest.push(i);
        }
    }
    visible.extend(rest);
    visible
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::md::layout::layout_document;
    use crate::md::parse::parse;

    #[test]
    fn priority_puts_visible_blocks_first() {
        // Three code blocks spread through a tall doc.
        let md = format!(
            "```rust\nfn a() {{}}\n```\n\n{}\n\n```rust\nfn b() {{}}\n```\n\n{}\n\n```rust\nfn c() {{}}\n```",
            "para\n\n".repeat(20),
            "para\n\n".repeat(20),
        );
        let doc = layout_document(&parse(&md).blocks, 80, false);
        assert_eq!(doc.code_blocks.len(), 3);
        // Viewport at the very top sees only the first block.
        let order = blocks_by_priority(&doc.code_blocks, 0, 10);
        assert_eq!(order.len(), 3); // all blocks present
        assert_eq!(order[0], 0); // the visible one is first
                                 // Scrolled to the last block, it should lead the ordering.
        let last = &doc.code_blocks[2];
        let order = blocks_by_priority(&doc.code_blocks, last.start, 10);
        assert_eq!(order[0], 2);
    }

    #[test]
    fn priority_is_a_permutation_of_all_indices() {
        let md = "```rust\nfn a() {}\n```\n\ntext\n\n```rust\nfn b() {}\n```";
        let doc = layout_document(&parse(md).blocks, 80, false);
        let mut order = blocks_by_priority(&doc.code_blocks, 0, 100);
        order.sort_unstable();
        assert_eq!(order, vec![0, 1]);
    }
}

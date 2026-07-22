//! Streaming stdin: progressive rendering of a document that arrives in chunks (Phase 4).
//!
//! The `llm | glance` demo. A reader thread pushes stdin bytes to the event loop, which appends
//! them to a [`StreamState`] and re-parses only the **active tail** — the growing region after the
//! last *stable* block boundary. Everything before that boundary is a complete block that can't
//! change as more text arrives, so it's parsed once and cached. This keeps live reflow cheap even
//! as the document grows to thousands of lines.
//!
//! Keys still come from `/dev/tty` (crossterm reads the tty, not stdin fd 0), so the piped
//! document and interactive input coexist with no special handling.

use std::io::Read;
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};

use crate::md::parse::{parse, Block};
use crate::term::input::Key;

/// The byte index up to which `text` consists of *complete* blocks — the end of the last blank line
/// that lies **outside** a fenced code block. Text before it is stable (won't change as the stream
/// grows); text after it is the active tail still being written. Fence-aware so a blank line inside
/// ```` ``` ```` / `~~~` is never mistaken for a block boundary.
pub fn stable_boundary(text: &str) -> usize {
    let mut in_fence = false;
    let mut boundary = 0;
    let mut pos = 0;
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches('\n');
        let trimmed = body.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
        } else if !in_fence && body.trim().is_empty() {
            // A blank line outside a fence closes a block: everything through it is stable.
            boundary = pos + line.len();
        }
        pos += line.len();
    }
    boundary
}

/// Accumulates streamed bytes and yields the current block list, re-parsing only the active tail.
#[derive(Default)]
pub struct StreamState {
    /// All bytes received so far (kept as bytes so a chunk boundary can't split a UTF-8 char).
    accumulated: Vec<u8>,
    /// Bytes of `accumulated` already folded into `stable_blocks`.
    stable_len: usize,
    /// Blocks for the stable prefix — parsed once as the boundary advances.
    stable_blocks: Vec<Block>,
}

impl StreamState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.accumulated.is_empty()
    }

    /// Append `bytes` and return the full current block list: the cached stable prefix plus a fresh
    /// parse of the (short) active tail. Only the tail is re-parsed per call.
    pub fn append(&mut self, bytes: &[u8]) -> Vec<Block> {
        self.accumulated.extend_from_slice(bytes);
        let text = String::from_utf8_lossy(&self.accumulated);
        let boundary = stable_boundary(&text);
        if boundary > self.stable_len {
            // Parse only the newly-stabilized region and append it to the cached prefix.
            self.stable_blocks
                .extend(parse(&text[self.stable_len..boundary]).blocks);
            self.stable_len = boundary;
        }
        let mut blocks = self.stable_blocks.clone();
        blocks.extend(parse(&text[self.stable_len..]).blocks);
        blocks
    }
}

/// A background reader that pushes stdin bytes to the event loop until EOF (then the channel
/// closes). Dropping it detaches the thread.
pub struct StreamReader {
    rx: Receiver<Vec<u8>>,
    _handle: JoinHandle<()>,
}

impl StreamReader {
    /// Spawn a thread reading `stdin` in chunks. On EOF the sender drops and [`drain`] runs dry.
    pub fn spawn_stdin() -> Self {
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        let handle = thread::spawn(move || {
            let mut stdin = std::io::stdin().lock();
            let mut buf = [0u8; 8192];
            loop {
                match stdin.read(&mut buf) {
                    Ok(0) | Err(_) => break, // EOF or error → stop; channel closes on drop
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break; // main loop gone
                        }
                    }
                }
            }
        });
        StreamReader {
            rx,
            _handle: handle,
        }
    }

    /// Drain all bytes received since the last call (non-blocking), concatenated.
    pub fn drain(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for chunk in self.rx.try_iter() {
            out.extend_from_slice(&chunk);
        }
        out
    }
}

/// Keys that mean "the user is reading back" → pause auto-follow (any upward / absolute-top move).
pub fn key_pauses_follow(key: Key) -> bool {
    matches!(
        key,
        Key::Char('k')
            | Key::Up
            | Key::Char('b')
            | Key::PageUp
            | Key::Char('u')
            | Key::Ctrl('u')
            | Key::Char('g')
            | Key::Home
    )
}

/// Keys that jump to the bottom → resume auto-follow.
pub fn key_resumes_follow(key: Key) -> bool {
    matches!(key, Key::Char('G') | Key::End)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_is_after_last_blank_line() {
        assert_eq!(stable_boundary(""), 0);
        assert_eq!(stable_boundary("no blank yet"), 0); // one incomplete block
        assert_eq!(stable_boundary("a\n\n"), 3); // through the blank line
        assert_eq!(stable_boundary("a\n\nb"), 3); // "b" is the active tail
        assert_eq!(stable_boundary("a\n\nb\n\nc"), 6); // last blank wins
    }

    #[test]
    fn boundary_ignores_blank_lines_inside_a_fence() {
        // The blank line is inside an open code fence → NOT a boundary.
        let t = "```\ncode\n\nmore\n";
        assert_eq!(stable_boundary(t), 0);
        // Once the fence closes and a real blank follows, that blank is the boundary.
        let t2 = "```\ncode\n\nmore\n```\n\ndone";
        let b = stable_boundary(t2);
        assert!(b > 0 && b < t2.len());
        assert_eq!(&t2[b..], "done");
    }

    #[test]
    fn append_caches_stable_and_reparses_tail() {
        let mut s = StreamState::new();
        // First chunk: one complete paragraph + a partial one.
        let blocks = s.append(b"# Title\n\nfirst para\n\npartial");
        // Heading + first para + the partial paragraph.
        assert!(blocks.len() >= 3);
        // More arrives completing the tail and starting another.
        let blocks2 = s.append(b" finished\n\n## Next");
        let headings = blocks2
            .iter()
            .filter(|b| matches!(b, Block::Heading { .. }))
            .count();
        assert_eq!(headings, 2); // Title + Next
    }

    #[test]
    fn append_preserves_a_streamed_code_block_with_blank_lines() {
        let mut s = StreamState::new();
        s.append(b"```rust\nfn a() {}\n");
        s.append(b"\nfn b() {}\n"); // blank line *inside* the still-open fence
        let blocks = s.append(b"```\n\ntext");
        // Exactly one code block (the blank line did not split it) + a paragraph.
        let code = blocks
            .iter()
            .filter(|b| matches!(b, Block::Code { .. }))
            .count();
        assert_eq!(code, 1);
    }

    #[test]
    fn follow_key_classification() {
        assert!(key_pauses_follow(Key::Char('k')));
        assert!(key_pauses_follow(Key::Up));
        assert!(key_pauses_follow(Key::Char('g')));
        assert!(!key_pauses_follow(Key::Char('j'))); // scrolling down keeps following
        assert!(key_resumes_follow(Key::Char('G')));
        assert!(key_resumes_follow(Key::End));
        assert!(!key_resumes_follow(Key::Char('j')));
    }
}

//! Background image worker (Phase 3): fetch → decode → render, off the UI thread.
//!
//! Same shape as [`crate::view::highlighter`] (producer thread + two channels + a drain step in the
//! event loop), but the apply step differs: a rendered image is *N* rows from a 1-row placeholder,
//! so the result is applied by re-laying-out (`ViewerState::set_resolved_image`), not an in-place
//! patch. Local paths resolve against the document's directory; remote `http(s)` URLs fetch via
//! `ureq`. Everything here runs on the worker thread, so first paint never blocks on I/O or decode.
//!
//! Rendering currently always uses the universal half-block path (works in every color terminal).
//! The Kitty encoder ([`crate::term::images::kitty_png`]) exists and is tested; wiring its raw
//! passthrough into the damage-diff renderer (reserving rows for one escape emission) is a
//! documented fast-follow.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use image::GenericImageView;

use crate::style::Line;
use crate::term::images::{cell_size, half_block};

/// Cap on fetched image bytes (~20 MB) so a hostile/huge URL can't exhaust memory.
const MAX_IMAGE_BYTES: u64 = 20 * 1024 * 1024;

/// A request to fetch + render one image. `cols` is the target width; `doc_dir` resolves relatives.
pub struct ImageRequest {
    pub tab: usize,
    pub index: usize,
    pub url: String,
    pub doc_dir: Option<PathBuf>,
    pub cols: usize,
}

/// A rendered image ready to apply. `cols` lets the main loop drop a result whose width is stale.
pub struct ImageResult {
    pub tab: usize,
    pub index: usize,
    pub cols: usize,
    pub lines: Vec<Line>,
}

/// Handle to the worker thread + channels. Dropping it ends the worker (request channel closes).
pub struct ImageLoader {
    tx: Sender<ImageRequest>,
    rx: Receiver<ImageResult>,
    _handle: JoinHandle<()>,
}

impl ImageLoader {
    pub fn spawn() -> Self {
        let (req_tx, req_rx) = mpsc::channel::<ImageRequest>();
        let (res_tx, res_rx) = mpsc::channel::<ImageResult>();
        let handle = thread::spawn(move || {
            for req in req_rx {
                if let Some(lines) = render_image(&req.url, req.doc_dir.as_deref(), req.cols) {
                    let sent = res_tx.send(ImageResult {
                        tab: req.tab,
                        index: req.index,
                        cols: req.cols,
                        lines,
                    });
                    if sent.is_err() {
                        break; // main loop gone
                    }
                }
                // On any fetch/decode failure we simply don't reply — the placeholder stays.
            }
        });
        ImageLoader {
            tx: req_tx,
            rx: res_rx,
            _handle: handle,
        }
    }

    pub fn request(&self, req: ImageRequest) {
        let _ = self.tx.send(req);
    }

    pub fn drain(&self) -> impl Iterator<Item = ImageResult> + '_ {
        self.rx.try_iter()
    }
}

/// Fetch, decode, and render an image to half-block rows at `cols` wide. `None` on any failure.
fn render_image(url: &str, doc_dir: Option<&Path>, cols: usize) -> Option<Vec<Line>> {
    let bytes = fetch(url, doc_dir)?;
    let img = image::load_from_memory(&bytes).ok()?;
    let (w, h) = img.dimensions();
    let (c, r) = cell_size(w, h, cols as u32);
    let lines = half_block(&img, c, r);
    (!lines.is_empty()).then_some(lines)
}

/// Whether `url` is a remote `http(s)` URL (vs. a local/relative path).
pub fn is_remote(url: &str) -> bool {
    let u = url.trim_start().to_ascii_lowercase();
    u.starts_with("http://") || u.starts_with("https://")
}

/// Fetch raw image bytes: `ureq` for remote URLs, the filesystem for local/relative paths.
fn fetch(url: &str, doc_dir: Option<&Path>) -> Option<Vec<u8>> {
    if is_remote(url) {
        let resp = ureq::get(url).call().ok()?;
        let mut buf = Vec::new();
        resp.into_reader()
            .take(MAX_IMAGE_BYTES)
            .read_to_end(&mut buf)
            .ok()?;
        Some(buf)
    } else {
        std::fs::read(resolve_local(url, doc_dir)).ok()
    }
}

/// Resolve a local image reference to a filesystem path: strip a `file://` scheme, and join a
/// relative path against the document's directory (if known).
fn resolve_local(url: &str, doc_dir: Option<&Path>) -> PathBuf {
    let raw = url.strip_prefix("file://").unwrap_or(url);
    let p = Path::new(raw);
    match doc_dir {
        Some(dir) if p.is_relative() => dir.join(p),
        _ => p.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_remote_vs_local() {
        assert!(is_remote("http://example.com/a.png"));
        assert!(is_remote("https://example.com/a.png"));
        assert!(is_remote("  HTTPS://EXAMPLE.com/a.png")); // trimmed + case-insensitive
        assert!(!is_remote("a.png"));
        assert!(!is_remote("./img/a.png"));
        assert!(!is_remote("/abs/a.png"));
        assert!(!is_remote("file:///abs/a.png"));
    }

    #[test]
    fn resolves_relative_against_doc_dir() {
        let dir = Path::new("/docs/guide");
        assert_eq!(
            resolve_local("img/a.png", Some(dir)),
            PathBuf::from("/docs/guide/img/a.png")
        );
        // Absolute paths ignore the doc dir.
        assert_eq!(
            resolve_local("/abs/a.png", Some(dir)),
            PathBuf::from("/abs/a.png")
        );
        // file:// scheme is stripped.
        assert_eq!(
            resolve_local("file:///abs/a.png", None),
            PathBuf::from("/abs/a.png")
        );
        // No doc dir → used as-is.
        assert_eq!(resolve_local("a.png", None), PathBuf::from("a.png"));
    }
}

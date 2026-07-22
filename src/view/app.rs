//! The interactive terminal event loop.
//!
//! [`TerminalGuard`] owns terminal setup/teardown as an RAII value: constructing it enters the
//! alt-screen + raw mode + mouse capture + hides the cursor; dropping it restores everything.
//! A panic hook performs the same restore, because the release build uses `panic = "abort"` (so
//! `Drop` does **not** run on panic) — a crash must never leave the terminal broken (plan §8).
//!
//! [`run`] is the loop: read a crossterm event → normalize via `term::input` → mutate
//! `ViewerState` → repaint changed rows via `view::render`. It is exercised by PTY integration
//! tests (spawn, scroll, quit, assert clean teardown) rather than unit tests.

use std::collections::HashSet;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, queue};

use std::path::PathBuf;

use crate::md::layout::LinkRef;
use crate::md::parse::Block;
use crate::open::{self, classify, LinkTarget};
use crate::paint::paint;
use crate::stream::{key_pauses_follow, key_resumes_follow, StreamReader, StreamState};
use crate::style::{Line as StyleLine, Span, Style};
use crate::term::caps::ColorDepth;
use crate::term::input::{map_event, Event, Key, Mouse};
use crate::text::width as text_width;
use crate::theme::{self, Theme};
use crate::view::copy;
use crate::view::highlighter::{blocks_by_priority, HighlightRequest, Highlighter};
use crate::view::images::{ImageLoader, ImageRequest};
use crate::view::overlays::{help_lines, Fuzzy, Links, Toc};
use crate::view::render::{build_frame, render, Frame};
use crate::view::slides::Slides;
use crate::view::state::{Action, ViewerState};
use crate::view::tabs::Tabs;
use crate::view::watch::{Debouncer, FileWatcher};

/// Key into the "already requested" set: (tab, code-block index, width, line-numbers). Width and
/// line-numbers are included so a resize / `l` toggle re-requests at the new geometry.
type HlKey = (usize, usize, usize, bool);

/// Post highlight requests for `state`'s code blocks (visible ones first), skipping any already
/// requested at the current geometry. Blocks with no language are skipped — syntect needs one.
fn enqueue_tab(
    h: &Highlighter,
    requested: &mut HashSet<HlKey>,
    tab_idx: usize,
    state: &ViewerState,
) {
    let width = state.width;
    let line_numbers = state.line_numbers();
    for block in blocks_by_priority(&state.doc.code_blocks, state.top, state.height) {
        let cb = &state.doc.code_blocks[block];
        if cb.lang.trim().is_empty() {
            continue;
        }
        if requested.insert((tab_idx, block, width, line_numbers)) {
            h.request(HighlightRequest {
                tab: tab_idx,
                block,
                content: cb.content.clone(),
                lang: cb.lang.clone(),
                width,
                line_numbers,
            });
        }
    }
}

/// Key into the requested-images set: (tab, image index, cols). Cols-keyed so a resize re-fetches
/// at the new width (and stale-width in-flight results are dropped on arrival).
type ImgKey = (usize, usize, usize);

/// Post fetch/render requests for `state`'s images (visible first), skipping already-requested.
fn enqueue_images(
    loader: &ImageLoader,
    requested: &mut HashSet<ImgKey>,
    tab_idx: usize,
    state: &ViewerState,
) {
    let cols = state.width;
    let doc_dir = state.current_dir();
    let n = state.image_count();
    // Visible images first, then the rest (both in document order).
    let order = (0..n)
        .filter(|&i| state.image_visible(i))
        .chain((0..n).filter(|&i| !state.image_visible(i)));
    for idx in order {
        if let Some((url, _alt)) = state.image_at(idx) {
            if requested.insert((tab_idx, idx, cols)) {
                loader.request(ImageRequest {
                    tab: tab_idx,
                    index: idx,
                    url,
                    doc_dir: doc_dir.clone(),
                    cols,
                });
            }
        }
    }
}

/// The streaming status pill shown at the bottom: live-following vs. paused (scrolled up).
fn stream_pill_text(following: bool) -> String {
    if following {
        "▼ following".to_string()
    } else {
        "▼ paused (G to follow)".to_string()
    }
}

/// Debounce window for auto-reload: a file must be quiet this long after its last change event
/// before we re-read it (coalesces an editor's write→rename→truncate burst into one reload).
const RELOAD_DEBOUNCE: Duration = Duration::from_millis(120);
/// How long to block for terminal input between watcher checks (only when watching files).
const POLL_TICK: Duration = Duration::from_millis(50);

/// Input mode of the viewer: `/` search prompt, `o` TOC, `:` fuzzy filter, `f` link picker,
/// `h`/`?` help, or normal document navigation.
enum Mode {
    Normal,
    Search(String),
    Toc(Toc),
    Fuzzy(Fuzzy),
    Links(Links),
    Help,
}

/// Copy `text` (if any) to the clipboard, writing the OSC 52 sequence to the terminal when that
/// path is used, and set a toast describing the outcome. `what` names the thing copied.
fn copy_to(
    out: &mut io::Stdout,
    state: &mut ViewerState,
    what: &str,
    text: Option<String>,
) -> io::Result<()> {
    let Some(text) = text else {
        state.set_toast(format!("no {what} here"));
        return Ok(());
    };
    let result = copy::copy(&text);
    if let Some(seq) = result.as_ref().and_then(|c| c.osc52.as_ref()) {
        write!(out, "{seq}")?;
        out.flush()?;
    }
    state.set_toast(copy::toast(what, result.as_ref()));
    Ok(())
}

/// Act on a chosen link: open web/other URLs externally; follow local markdown in-app, open other
/// local files externally. Errors are swallowed (a missing opener must not crash the viewer).
fn follow_link(state: &mut ViewerState, link: &LinkRef) {
    match classify(&link.url, state.current_dir().as_deref()) {
        LinkTarget::Url(u) | LinkTarget::Other(u) => {
            let _ = open::open_url(&u);
        }
        LinkTarget::LocalFile(p) => {
            if open::is_markdown(&p) {
                let _ = state.load(p);
            } else {
                let _ = open::open_url(&p.to_string_lossy());
            }
        }
    }
}

/// RAII terminal setup/teardown. Enter on construction, restore on drop.
pub struct TerminalGuard;

impl TerminalGuard {
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture, Hide)?;
        Ok(TerminalGuard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best-effort restore; ignore errors during teardown.
        let _ = execute!(
            io::stdout(),
            Show,
            DisableMouseCapture,
            LeaveAlternateScreen
        );
        let _ = disable_raw_mode();
    }
}

/// Restore the terminal from a panic hook (release builds abort, so `Drop` won't run).
fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(
            io::stdout(),
            Show,
            DisableMouseCapture,
            LeaveAlternateScreen
        );
        let _ = disable_raw_mode();
        default(info);
    }));
}

/// Run the interactive viewer over `blocks`. `width_override` (non-zero) fixes the content
/// width; otherwise the terminal width is used. Returns when the user quits.
#[allow(clippy::too_many_arguments)] // interactive entry point legitimately takes the full config
pub fn run(
    docs: Vec<(Vec<Block>, Option<PathBuf>)>,
    theme_dark: bool,
    depth: ColorDepth,
    hyperlinks: bool,
    width_override: Option<usize>,
    line_numbers: bool,
    stream: Option<StreamReader>,
) -> io::Result<()> {
    install_panic_hook();
    // Streaming mode: the (single) document grows from stdin; auto-follow the bottom until the
    // user scrolls up. `following` is only meaningful when `stream` is Some.
    let mut stream_state = StreamState::new();
    let mut following = stream.is_some();
    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let width = width_override
        .filter(|&w| w > 0)
        .map_or(cols as usize, |w| w.min(cols as usize));
    // One tab per document; per-tab scroll/search state is preserved across switches. The bottom
    // row is reserved for the persistent status/hint bar, so the content viewport is `rows - 1`.
    let content_rows = (rows as usize).saturating_sub(1).max(1);
    let states = docs
        .into_iter()
        .map(|(blocks, path)| ViewerState::new(blocks, width, content_rows, path, line_numbers))
        .collect();
    let mut tabs = Tabs::new(states);
    // Theme is toggled at runtime (`t`); it affects paint only, never layout.
    let mut dark = theme_dark;
    let mut theme = if dark { theme::dark() } else { theme::light() };

    let _guard = TerminalGuard::enter()?;
    let mut out = io::stdout();
    let mut prev: Option<Frame> = None;
    let mut mode = Mode::Normal;

    draw(
        &mut out, &tabs, &theme, depth, hyperlinks, &mut prev, true, &mode,
    )?;

    // Auto-reload: watch every open file's directory; a settled change re-reads that tab in
    // place. `None` (no files, e.g. piped stdin, or a watch error).
    let watcher = FileWatcher::new(&tabs.paths()).unwrap_or(None);
    let mut debouncer = Debouncer::new(RELOAD_DEBOUNCE);

    // Background syntect highlighting: spawn the worker only after the first paint (its SyntaxSet
    // load happens on that thread, never on the startup path) and enqueue the active tab's blocks.
    let highlighter = Highlighter::spawn();
    let mut requested: HashSet<HlKey> = HashSet::new();
    enqueue_tab(
        &highlighter,
        &mut requested,
        tabs.active_index(),
        tabs.active(),
    );

    // Background image fetch/decode/render. Only meaningful with color; in `None` the placeholders
    // stay. Enqueued the same way (visible first) and applied by re-layout when each resolves.
    let images = depth != ColorDepth::None;
    let image_loader = ImageLoader::spawn();
    let mut requested_images: HashSet<ImgKey> = HashSet::new();
    if images {
        enqueue_images(
            &image_loader,
            &mut requested_images,
            tabs.active_index(),
            tabs.active(),
        );
    }
    if stream.is_some() {
        tabs.active_mut()
            .set_stream_pill(Some(stream_pill_text(following)));
    }

    loop {
        // Fold in filesystem events, then reload any file that has gone quiet.
        if let Some(w) = &watcher {
            let now = Instant::now();
            for p in w.drain() {
                debouncer.mark(p, now);
            }
            if !debouncer.is_empty() {
                let mut redraw = false;
                for p in debouncer.ready(Instant::now()) {
                    redraw |= tabs.reload_path(&p);
                }
                if redraw {
                    // The doc changed → its highlights + images are stale; re-enqueue.
                    requested.clear();
                    requested_images.clear();
                    enqueue_tab(
                        &highlighter,
                        &mut requested,
                        tabs.active_index(),
                        tabs.active(),
                    );
                    if images {
                        enqueue_images(
                            &image_loader,
                            &mut requested_images,
                            tabs.active_index(),
                            tabs.active(),
                        );
                    }
                    tabs.active_mut().set_toast("reloaded");
                    draw(
                        &mut out, &tabs, &theme, depth, hyperlinks, &mut prev, true, &mode,
                    )?;
                }
            }
        }

        // Patch in finished highlight results; repaint only when the active tab's visible content
        // actually changed. Stale-geometry results (post-resize / `l`) are dropped by the checks.
        let active = tabs.active_index();
        let mut hl_repaint = false;
        for res in highlighter.drain() {
            if let Some(tab) = tabs.get_mut(res.tab) {
                if tab.width == res.width && tab.line_numbers() == res.line_numbers {
                    let visible = tab.code_block_visible(res.block);
                    if tab.patch_code_block(res.block, res.lines) && res.tab == active && visible {
                        hl_repaint = true;
                    }
                }
            }
        }
        if hl_repaint {
            draw(
                &mut out, &tabs, &theme, depth, hyperlinks, &mut prev, false, &mode,
            )?;
        }

        // Apply finished images: each resolve re-lays-out the tab (placeholder → N rows), so a
        // visible one needs a full repaint. Stale-width results are dropped by the cols check.
        let mut img_repaint = false;
        for res in image_loader.drain() {
            if let Some(tab) = tabs.get_mut(res.tab) {
                if tab.width == res.cols {
                    let visible = tab.set_resolved_image(res.index, res.lines);
                    if res.tab == active && visible {
                        img_repaint = true;
                    }
                }
            }
        }
        if img_repaint {
            draw(
                &mut out, &tabs, &theme, depth, hyperlinks, &mut prev, true, &mode,
            )?;
        }

        // Streaming: fold in any bytes that arrived from stdin, re-parse the active tail, and
        // (while following) keep the viewport pinned to the bottom. Re-enqueue highlight/images
        // since the doc grew. Nothing renders until the first chunk, so first paint isn't blocked.
        if let Some(reader) = &stream {
            let bytes = reader.drain();
            if !bytes.is_empty() {
                let blocks = stream_state.append(&bytes);
                let state = tabs.active_mut();
                state.set_blocks(blocks);
                if following {
                    state.to_bottom();
                }
                requested.clear();
                requested_images.clear();
                enqueue_tab(&highlighter, &mut requested, 0, tabs.active());
                if images {
                    enqueue_images(&image_loader, &mut requested_images, 0, tabs.active());
                }
                draw(
                    &mut out, &tabs, &theme, depth, hyperlinks, &mut prev, true, &mode,
                )?;
            }
        }

        // Always poll (never block): the watcher, highlight, image, and stream workers all deliver
        // events that must be serviced while the user is idle.
        if !event::poll(POLL_TICK)? {
            continue;
        }
        match map_event(event::read()?) {
            Some(Event::Resize { cols, rows }) => {
                // Reserve the bottom row for the status/hint bar (see `run`).
                tabs.resize_all(cols as usize, (rows as usize).saturating_sub(1).max(1));
                // Width changed → prior highlights + image renders are the wrong geometry.
                requested.clear();
                enqueue_tab(
                    &highlighter,
                    &mut requested,
                    tabs.active_index(),
                    tabs.active(),
                );
                if images {
                    enqueue_images(
                        &image_loader,
                        &mut requested_images,
                        tabs.active_index(),
                        tabs.active(),
                    );
                }
                draw(
                    &mut out, &tabs, &theme, depth, hyperlinks, &mut prev, true, &mode,
                )?;
            }
            // A left-click on a code block copies it (via the OSC 52 + native copy stack);
            // any other mouse event (wheel) scrolls.
            Some(Event::Mouse(Mouse::Click { row, .. })) if matches!(mode, Mode::Normal) => {
                let state = tabs.active_mut();
                if let Some(text) = state.code_block_at(row as usize) {
                    copy_to(&mut out, state, "code block", Some(text))?;
                    draw(
                        &mut out, &tabs, &theme, depth, hyperlinks, &mut prev, false, &mode,
                    )?;
                }
            }
            Some(Event::Mouse(m)) if matches!(mode, Mode::Normal) => {
                if tabs.active_mut().on_mouse(m) == Action::Redraw {
                    draw(
                        &mut out, &tabs, &theme, depth, hyperlinks, &mut prev, false, &mode,
                    )?;
                }
            }
            // Tab / Shift+Tab switch tabs (Normal mode), before we borrow the active tab.
            Some(Event::Key(k))
                if matches!(mode, Mode::Normal) && matches!(k, Key::Tab | Key::BackTab) =>
            {
                if k == Key::Tab {
                    tabs.next();
                } else {
                    tabs.prev();
                }
                // Newly-active tab hasn't been highlighted / imaged yet — enqueue its blocks.
                enqueue_tab(
                    &highlighter,
                    &mut requested,
                    tabs.active_index(),
                    tabs.active(),
                );
                if images {
                    enqueue_images(
                        &image_loader,
                        &mut requested_images,
                        tabs.active_index(),
                        tabs.active(),
                    );
                }
                draw(
                    &mut out, &tabs, &theme, depth, hyperlinks, &mut prev, true, &mode,
                )?;
            }
            Some(Event::Key(k)) => {
                let mut full = false;
                let state = tabs.active_mut();
                state.clear_toast(); // any keypress dismisses a transient toast
                                     // Take mode out so we can reassign it inside the match.
                match std::mem::replace(&mut mode, Mode::Normal) {
                    Mode::Normal => match k {
                        Key::Char('/') => mode = Mode::Search(String::new()),
                        Key::Char('o') => {
                            let toc = Toc::new(&state.doc.headings);
                            if !toc.is_empty() {
                                mode = Mode::Toc(toc);
                                full = true; // overlay replaces the screen
                            }
                        }
                        Key::Char(':') => {
                            if !state.doc.headings.is_empty() {
                                mode = Mode::Fuzzy(Fuzzy::new(&state.doc.headings));
                                full = true;
                            }
                        }
                        Key::Char('f') => {
                            let links = Links::new(&state.doc.links);
                            if !links.is_empty() {
                                mode = Mode::Links(links);
                                full = true;
                            }
                        }
                        Key::Char('c') => {
                            let text = state.nearest_code_block();
                            copy_to(&mut out, state, "code block", text)?;
                        }
                        Key::Char('Y') => {
                            let text = Some(state.document_text());
                            copy_to(&mut out, state, "document", text)?;
                        }
                        Key::Char('p') => match state.file_path_string() {
                            Some(p) => copy_to(&mut out, state, "path", Some(p))?,
                            None => state.set_toast("(stdin — no path)"),
                        },
                        Key::Char('h') | Key::Char('?') | Key::F(1) => {
                            mode = Mode::Help;
                            full = true;
                        }
                        Key::Char('t') => {
                            dark = !dark;
                            theme = if dark { theme::dark() } else { theme::light() };
                            full = true; // theme changes every color → full repaint
                            state.set_toast(if dark { "theme: dark" } else { "theme: light" });
                        }
                        other => {
                            // Streaming: scrolling up pauses auto-follow; `G`/End resumes it.
                            if stream.is_some() {
                                if key_resumes_follow(other) {
                                    following = true;
                                } else if key_pauses_follow(other) {
                                    following = false;
                                }
                                state.set_stream_pill(Some(stream_pill_text(following)));
                            }
                            match state.on_key(other) {
                                Action::Quit => break,
                                Action::Redraw => {}
                                Action::Ignore => continue,
                            }
                        }
                    },
                    Mode::Search(mut query) => match k {
                        Key::Char(c) => {
                            query.push(c);
                            mode = Mode::Search(query);
                        }
                        Key::Backspace => {
                            query.pop();
                            mode = Mode::Search(query);
                        }
                        Key::Enter => {
                            state.run_search(&query);
                            full = true; // search may jump the viewport → full repaint
                        }
                        Key::Esc => full = true, // cancel input; mode already Normal
                        _ => mode = Mode::Search(query),
                    },
                    Mode::Toc(mut toc) => {
                        full = true; // overlay is a full-screen repaint either way
                        match k {
                            Key::Char('j') | Key::Down => {
                                toc.down();
                                mode = Mode::Toc(toc);
                            }
                            Key::Char('k') | Key::Up => {
                                toc.up();
                                mode = Mode::Toc(toc);
                            }
                            Key::Enter => {
                                if let Some(line) = toc.selected_line() {
                                    state.center_on(line);
                                }
                                // mode already Normal → overlay closes
                            }
                            Key::Esc | Key::Char('o') | Key::Char('q') => {} // close
                            _ => mode = Mode::Toc(toc),                      // stay open
                        }
                    }
                    // Fuzzy filter: typed chars edit the query; arrows move the selection.
                    Mode::Fuzzy(mut f) => {
                        full = true;
                        match k {
                            Key::Char(c) => {
                                f.push(c);
                                mode = Mode::Fuzzy(f);
                            }
                            Key::Backspace => {
                                f.pop();
                                mode = Mode::Fuzzy(f);
                            }
                            Key::Down => {
                                f.down();
                                mode = Mode::Fuzzy(f);
                            }
                            Key::Up => {
                                f.up();
                                mode = Mode::Fuzzy(f);
                            }
                            Key::Enter => {
                                if let Some(line) = f.selected_line() {
                                    state.center_on(line);
                                }
                            }
                            Key::Esc => {} // close
                            _ => mode = Mode::Fuzzy(f),
                        }
                    }
                    // Link picker: digits open directly; arrows move, Enter opens the selection.
                    Mode::Links(mut links) => {
                        full = true;
                        match k {
                            Key::Char(c) if c.is_ascii_digit() && c != '0' => {
                                let idx = (c as u8 - b'1') as usize;
                                if let Some(link) = links.at(idx).cloned() {
                                    follow_link(state, &link); // opens, then closes
                                } else {
                                    mode = Mode::Links(links);
                                }
                            }
                            Key::Char('j') | Key::Down => {
                                links.down();
                                mode = Mode::Links(links);
                            }
                            Key::Char('k') | Key::Up => {
                                links.up();
                                mode = Mode::Links(links);
                            }
                            Key::Enter => {
                                if let Some(link) = links.selected().cloned() {
                                    follow_link(state, &link);
                                }
                            }
                            Key::Esc | Key::Char('f') | Key::Char('q') => {} // close
                            _ => mode = Mode::Links(links),
                        }
                    }
                    // Help: any key closes it.
                    Mode::Help => full = true,
                }
                draw(
                    &mut out, &tabs, &theme, depth, hyperlinks, &mut prev, full, &mode,
                )?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Present `slides` one at a time (slide mode, `-s`). Navigation keys advance/retreat slides; each
/// slide is laid out like a mini-document and vertically centered. Full repaint per transition.
pub fn run_slides(
    slides: Vec<Vec<Block>>,
    theme_dark: bool,
    depth: ColorDepth,
    hyperlinks: bool,
    width_override: Option<usize>,
) -> io::Result<()> {
    install_panic_hook();
    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let mut width = width_override
        .filter(|&w| w > 0)
        .map_or(cols as usize, |w| w.min(cols as usize));
    let mut height = rows as usize;
    let mut deck = Slides::new(slides);
    let theme = if theme_dark {
        theme::dark()
    } else {
        theme::light()
    };

    let _guard = TerminalGuard::enter()?;
    let mut out = io::stdout();
    draw_slide(&mut out, &deck, width, height, &theme, depth, hyperlinks)?;

    loop {
        match map_event(event::read()?) {
            Some(Event::Resize { cols, rows }) => {
                width = width_override
                    .filter(|&w| w > 0)
                    .map_or(cols as usize, |w| w.min(cols as usize));
                height = rows as usize;
            }
            Some(Event::Key(k)) => match k {
                Key::Char('q') | Key::Ctrl('c') => break,
                Key::Right
                | Key::Char(' ')
                | Key::Char('l')
                | Key::Char('j')
                | Key::Down
                | Key::PageDown => deck.next(),
                Key::Left
                | Key::Char('h')
                | Key::Char('k')
                | Key::Char('b')
                | Key::Up
                | Key::PageUp => deck.prev(),
                Key::Char('g') | Key::Home => deck.first(),
                Key::Char('G') | Key::End => deck.last(),
                _ => continue,
            },
            _ => continue,
        }
        draw_slide(&mut out, &deck, width, height, &theme, depth, hyperlinks)?;
    }
    Ok(())
}

/// Lay out the current slide, vertically center it, and paint it with a `slide n/N` footer.
fn draw_slide(
    out: &mut io::Stdout,
    deck: &Slides,
    width: usize,
    height: usize,
    theme: &Theme,
    depth: ColorDepth,
    hyperlinks: bool,
) -> io::Result<()> {
    let doc = crate::md::layout::layout_document(deck.current(), width, false);
    let content_rows = height.saturating_sub(1); // reserve the last row for the footer
    let mut frame: Vec<String> = Vec::with_capacity(height);
    // Vertically center a short slide.
    let pad = content_rows.saturating_sub(doc.lines.len()) / 2;
    for _ in 0..pad {
        frame.push(String::new());
    }
    for line in &doc.lines {
        frame.push(paint(line, theme, depth, hyperlinks));
    }
    frame.truncate(content_rows);
    frame.resize(content_rows, String::new());
    let mut footer = format!(
        "slide {}/{}  · →/Space next · ←/b prev · g/G ends · q quit",
        deck.index() + 1,
        deck.len()
    );
    if footer.chars().count() > width {
        footer = footer.chars().take(width).collect();
    }
    frame.push(footer);
    // Full repaint each transition (slides change wholesale) — no damage diff needed.
    queue!(out, crossterm::style::Print(render(None, &frame)))?;
    out.flush()?;
    Ok(())
}

/// Build the current frame (with search highlighting + a status/prompt line) and write the diff.
#[allow(clippy::too_many_arguments)] // a render step legitimately takes the full frame context
fn draw(
    out: &mut io::Stdout,
    tabs: &Tabs,
    theme: &Theme,
    depth: ColorDepth,
    hyperlinks: bool,
    prev: &mut Option<Frame>,
    force_full: bool,
    mode: &Mode,
) -> io::Result<()> {
    let state = tabs.active();
    let overlay = match mode {
        Mode::Toc(toc) => Some(toc.view(state.width, state.height)),
        Mode::Fuzzy(f) => Some(f.view(state.width, state.height)),
        Mode::Links(l) => Some(l.view(state.width, state.height)),
        Mode::Help => Some(help_lines()),
        _ => None,
    };
    let mut frame = match overlay {
        // A picker overlay takes over the screen.
        Some(lines) => {
            let mut f: Vec<String> = lines
                .iter()
                .map(|l| paint(l, theme, depth, hyperlinks))
                .collect();
            f.resize(state.height, String::new());
            f
        }
        None => build_frame(
            &state.doc,
            state.top,
            state.height,
            theme,
            depth,
            hyperlinks,
            state.search.as_ref(),
        ),
    };
    // Append the persistent status/hint bar as its own (reserved) bottom row.
    frame.push(status_bar(
        &bar_text(state, mode, tabs.label()),
        state.width,
        theme,
        depth,
    ));
    let base = if force_full { None } else { prev.as_ref() };
    queue!(out, crossterm::style::Print(render(base, &frame)))?;
    out.flush()?;
    *prev = Some(frame);
    Ok(())
}

/// The default Normal-mode key legend shown on the status bar (truncated to width). Ordered by how
/// commonly the shortcut is reached for; `q quit` leads so exiting is always discoverable.
const LEGEND: &str =
    " q quit · / search · o toc · f links · c copy · Y all · t theme · Tab files · h help ";

/// The text for the status/hint bar: a contextual prompt while an overlay/search/toast is active,
/// otherwise the key legend (with the tab label / streaming pill folded in).
fn bar_text(state: &ViewerState, mode: &Mode, tab_label: Option<String>) -> String {
    match mode {
        Mode::Search(query) => format!(" /{query}"),
        Mode::Toc(toc) => format!(
            " TOC  {}/{}  · j/k move · Enter jump · Esc close",
            toc.selected_index() + 1,
            toc.len()
        ),
        Mode::Fuzzy(f) => {
            if f.count() == 0 {
                format!(" :{}  no matches · Esc close", f.query)
            } else {
                format!(
                    " :{}  {}/{}  · ↑↓ move · Enter jump · Esc close",
                    f.query,
                    f.selected_index() + 1,
                    f.count()
                )
            }
        }
        Mode::Links(l) => format!(
            " Links  {}/{}  · digits/↑↓ select · Enter open · Esc close",
            l.selected_index() + 1,
            l.len()
        ),
        Mode::Help => " Help  ·  press any key to close ".to_string(),
        // A transient toast, then an active search readout, then the streaming pill, else the
        // legend — each keeps the always-present `h help` reachable.
        Mode::Normal => {
            if let Some(t) = state.toast() {
                return format!(" {t}  · h help ");
            }
            if let Some(s) = &state.search {
                return if s.is_empty() {
                    format!(" /{}  no matches · n/N cycle · Esc clear", s.query)
                } else {
                    format!(
                        " /{}  {}/{}  · n/N cycle · Esc clear",
                        s.query,
                        s.position(),
                        s.len()
                    )
                };
            }
            if let Some(p) = state.stream_pill() {
                return format!(" {p} ·{LEGEND}");
            }
            match tab_label {
                Some(lbl) => format!(" {lbl} ·{LEGEND}"),
                None => LEGEND.to_string(),
            }
        }
    }
}

/// Render the bar `text` as a full-width reverse-video row (truncated / space-padded to `width`).
fn status_bar(text: &str, width: usize, theme: &Theme, depth: ColorDepth) -> String {
    let mut shown = String::new();
    let mut w = 0;
    for ch in text.chars() {
        let cw = text_width(&ch.to_string());
        if w + cw > width {
            break;
        }
        w += cw;
        shown.push(ch);
    }
    if w < width {
        shown.push_str(&" ".repeat(width - w));
    }
    // Reverse video reads as a status bar at any color depth (it's an attribute, not a color).
    let line = StyleLine {
        spans: vec![Span::new(
            shown,
            Style {
                highlight: true,
                ..Default::default()
            },
        )],
        no_wrap: true,
    };
    paint(&line, theme, depth, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::md::parse::parse;

    fn state() -> ViewerState {
        ViewerState::new(parse("# hi\n\nbody").blocks, 80, 24, None, false)
    }

    #[test]
    fn legend_shows_quit_and_help() {
        let bar = bar_text(&state(), &Mode::Normal, None);
        assert!(bar.contains("q quit"), "legend must make exit discoverable");
        assert!(bar.contains("h help"));
        assert!(bar.contains("/ search"));
    }

    #[test]
    fn multi_tab_label_folds_into_legend() {
        let bar = bar_text(&state(), &Mode::Normal, Some("[2/3 b.md]".into()));
        assert!(bar.contains("[2/3 b.md]"));
        assert!(bar.contains("q quit"));
    }

    #[test]
    fn status_bar_pads_to_exact_width() {
        // Depth None → plain text (no ANSI), padded to the full width.
        let out = status_bar(" hi ", 20, &theme::dark(), ColorDepth::None);
        assert_eq!(out.chars().count(), 20);
    }
}

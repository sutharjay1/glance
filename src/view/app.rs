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

use std::io::{self, Write};

use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, queue};

use crate::md::parse::Block;
use crate::paint::paint;
use crate::term::caps::ColorDepth;
use crate::term::input::{map_event, Event, Key};
use crate::theme::Theme;
use crate::view::overlays::Toc;
use crate::view::render::{build_frame, render, Frame};
use crate::view::state::{Action, ViewerState};

/// Input mode of the viewer: the `/` search prompt, the `o` table-of-contents overlay, or normal.
enum Mode {
    Normal,
    Search(String),
    Toc(Toc),
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
pub fn run(
    blocks: Vec<Block>,
    theme: Theme,
    depth: ColorDepth,
    hyperlinks: bool,
    width_override: Option<usize>,
) -> io::Result<()> {
    install_panic_hook();
    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let width = width_override
        .filter(|&w| w > 0)
        .map_or(cols as usize, |w| w.min(cols as usize));
    let mut state = ViewerState::new(blocks, width, rows as usize);

    let _guard = TerminalGuard::enter()?;
    let mut out = io::stdout();
    let mut prev: Option<Frame> = None;
    let mut mode = Mode::Normal;

    draw(
        &mut out, &state, &theme, depth, hyperlinks, &mut prev, true, &mode,
    )?;

    loop {
        match map_event(event::read()?) {
            Some(Event::Resize { cols, rows }) => {
                state.on_resize(cols as usize, rows as usize);
                draw(
                    &mut out, &state, &theme, depth, hyperlinks, &mut prev, true, &mode,
                )?;
            }
            Some(Event::Mouse(m)) if matches!(mode, Mode::Normal) => {
                if state.on_mouse(m) == Action::Redraw {
                    draw(
                        &mut out, &state, &theme, depth, hyperlinks, &mut prev, false, &mode,
                    )?;
                }
            }
            Some(Event::Key(k)) => {
                let mut full = false;
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
                        other => match state.on_key(other) {
                            Action::Quit => break,
                            Action::Redraw => {}
                            Action::Ignore => continue,
                        },
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
                }
                draw(
                    &mut out, &state, &theme, depth, hyperlinks, &mut prev, full, &mode,
                )?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Build the current frame (with search highlighting + a status/prompt line) and write the diff.
#[allow(clippy::too_many_arguments)] // a render step legitimately takes the full frame context
fn draw(
    out: &mut io::Stdout,
    state: &ViewerState,
    theme: &Theme,
    depth: ColorDepth,
    hyperlinks: bool,
    prev: &mut Option<Frame>,
    force_full: bool,
    mode: &Mode,
) -> io::Result<()> {
    let mut frame = match mode {
        // The TOC overlay takes over the screen.
        Mode::Toc(toc) => {
            let mut f: Vec<String> = toc
                .view(state.width, state.height)
                .iter()
                .map(|l| paint(l, theme, depth, hyperlinks))
                .collect();
            f.resize(state.height, String::new());
            f
        }
        _ => build_frame(
            &state.doc,
            state.top,
            state.height,
            theme,
            depth,
            hyperlinks,
            state.search.as_ref(),
        ),
    };
    // Overlay the prompt / status on the bottom row when relevant.
    if let Some(status) = status_line(state, mode) {
        if let Some(last) = frame.last_mut() {
            *last = status;
        }
    }
    let base = if force_full { None } else { prev.as_ref() };
    queue!(out, crossterm::style::Print(render(base, &frame)))?;
    out.flush()?;
    *prev = Some(frame);
    Ok(())
}

/// The bottom-row status line: the live `/` prompt while typing, or a `query  3/12` readout
/// while a search is active.
fn status_line(state: &ViewerState, mode: &Mode) -> Option<String> {
    match mode {
        Mode::Search(query) => Some(format!("/{query}")),
        Mode::Toc(toc) => Some(format!(
            "TOC  {}/{}  · j/k move · Enter jump · Esc close",
            toc.selected_index() + 1,
            toc.len()
        )),
        Mode::Normal => state.search.as_ref().map(|s| {
            if s.is_empty() {
                format!("/{}  no matches", s.query)
            } else {
                format!("/{}  {}/{}", s.query, s.position(), s.len())
            }
        }),
    }
}

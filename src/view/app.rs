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
use crate::term::caps::ColorDepth;
use crate::term::input::{map_event, Event};
use crate::theme::Theme;
use crate::view::render::{build_frame, render, Frame};
use crate::view::state::{Action, ViewerState};

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

    draw(&mut out, &state, &theme, depth, hyperlinks, &mut prev, true)?;

    loop {
        let ev = event::read()?;
        let mut force_full = false;
        let action = match map_event(ev) {
            Some(Event::Key(k)) => state.on_key(k),
            Some(Event::Mouse(m)) => state.on_mouse(m),
            Some(Event::Resize { cols, rows }) => {
                state.on_resize(cols as usize, rows as usize);
                force_full = true; // geometry changed → repaint everything
                Action::Redraw
            }
            None => Action::Ignore,
        };
        match action {
            Action::Quit => break,
            Action::Redraw => draw(
                &mut out, &state, &theme, depth, hyperlinks, &mut prev, force_full,
            )?,
            Action::Ignore => {}
        }
    }
    Ok(())
}

/// Build the current frame and write the diff (or a full repaint) to the terminal.
fn draw(
    out: &mut io::Stdout,
    state: &ViewerState,
    theme: &Theme,
    depth: ColorDepth,
    hyperlinks: bool,
    prev: &mut Option<Frame>,
    force_full: bool,
) -> io::Result<()> {
    let frame = build_frame(
        &state.doc,
        state.top,
        state.height,
        theme,
        depth,
        hyperlinks,
    );
    let base = if force_full { None } else { prev.as_ref() };
    queue!(out, crossterm::style::Print(render(base, &frame)))?;
    out.flush()?;
    *prev = Some(frame);
    Ok(())
}

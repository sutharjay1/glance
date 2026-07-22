//! Input events, normalized from crossterm.
//!
//! Per ADR 0003, crossterm owns the platform-specific decoding (Windows ConPTY, mouse
//! encodings, key sequences). This module maps its rich event type down to the small,
//! stable vocabulary glance's keybinding dispatch cares about — so the event loop matches on
//! `Key::Char('j')`, not raw escape bytes. The mapping is pure and unit-testable.

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton as CtMouseButton,
    MouseEvent, MouseEventKind,
};

/// A normalized key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    /// Ctrl + a letter (letter lowercased), e.g. `Ctrl('c')`.
    Ctrl(char),
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Enter,
    Esc,
    Backspace,
    Tab,
    /// Shift+Tab.
    BackTab,
    F(u8),
}

/// A normalized mouse action. Coordinates are 0-based cells for click hit-testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mouse {
    ScrollUp,
    ScrollDown,
    Click { col: u16, row: u16 },
}

/// A normalized terminal event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Key(Key),
    Mouse(Mouse),
    Resize { cols: u16, rows: u16 },
}

/// Map a crossterm event to a glance [`Event`], or `None` for events we don't act on
/// (key releases, paste, focus, unmapped keys/buttons).
pub fn map_event(ev: CtEvent) -> Option<Event> {
    match ev {
        CtEvent::Key(k) => map_key(k).map(Event::Key),
        CtEvent::Mouse(m) => map_mouse(m).map(Event::Mouse),
        CtEvent::Resize(cols, rows) => Some(Event::Resize { cols, rows }),
        _ => None,
    }
}

fn map_key(k: KeyEvent) -> Option<Key> {
    // Ignore key releases so a single press isn't processed twice (Windows emits both).
    if k.kind == KeyEventKind::Release {
        return None;
    }
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    Some(match k.code {
        KeyCode::Char(c) if ctrl => Key::Ctrl(c.to_ascii_lowercase()),
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Esc,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Tab => Key::Tab,
        KeyCode::BackTab => Key::BackTab,
        KeyCode::F(n) => Key::F(n),
        _ => return None,
    })
}

fn map_mouse(m: MouseEvent) -> Option<Mouse> {
    Some(match m.kind {
        MouseEventKind::ScrollUp => Mouse::ScrollUp,
        MouseEventKind::ScrollDown => Mouse::ScrollDown,
        MouseEventKind::Down(CtMouseButton::Left) => Mouse::Click {
            col: m.column,
            row: m.row,
        },
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventState;

    fn press(code: KeyCode, mods: KeyModifiers) -> CtEvent {
        CtEvent::Key(KeyEvent::new(code, mods))
    }

    #[test]
    fn plain_char() {
        assert_eq!(
            map_event(press(KeyCode::Char('j'), KeyModifiers::NONE)),
            Some(Event::Key(Key::Char('j')))
        );
    }

    #[test]
    fn ctrl_char_is_lowercased() {
        assert_eq!(
            map_event(press(KeyCode::Char('C'), KeyModifiers::CONTROL)),
            Some(Event::Key(Key::Ctrl('c')))
        );
    }

    #[test]
    fn navigation_keys() {
        assert_eq!(
            map_event(press(KeyCode::Up, KeyModifiers::NONE)),
            Some(Event::Key(Key::Up))
        );
        assert_eq!(
            map_event(press(KeyCode::PageDown, KeyModifiers::NONE)),
            Some(Event::Key(Key::PageDown))
        );
        assert_eq!(
            map_event(press(KeyCode::Esc, KeyModifiers::NONE)),
            Some(Event::Key(Key::Esc))
        );
        assert_eq!(
            map_event(press(KeyCode::BackTab, KeyModifiers::SHIFT)),
            Some(Event::Key(Key::BackTab))
        );
        assert_eq!(
            map_event(press(KeyCode::F(1), KeyModifiers::NONE)),
            Some(Event::Key(Key::F(1)))
        );
    }

    #[test]
    fn key_release_is_ignored() {
        let release = CtEvent::Key(KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        });
        assert_eq!(map_event(release), None);
    }

    #[test]
    fn mouse_scroll_and_click() {
        let scroll = CtEvent::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(map_event(scroll), Some(Event::Mouse(Mouse::ScrollUp)));

        let click = CtEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(CtMouseButton::Left),
            column: 5,
            row: 3,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            map_event(click),
            Some(Event::Mouse(Mouse::Click { col: 5, row: 3 }))
        );
    }

    #[test]
    fn right_click_is_ignored() {
        let rclick = CtEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(CtMouseButton::Right),
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(map_event(rclick), None);
    }

    #[test]
    fn resize_passes_through() {
        assert_eq!(
            map_event(CtEvent::Resize(80, 24)),
            Some(Event::Resize { cols: 80, rows: 24 })
        );
    }

    #[test]
    fn paste_is_ignored() {
        assert_eq!(map_event(CtEvent::Paste("x".into())), None);
    }
}

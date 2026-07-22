//! Multi-file tabs: a set of open documents with one active, cycled by `Tab`/`Shift+Tab`.
//!
//! Each tab is a full [`ViewerState`], so per-tab scroll position, search, and line-number state
//! are preserved when switching. The holder itself is trivial; keeping it separate makes the
//! next/prev cycling testable without a terminal.

use std::path::{Path, PathBuf};

use crate::view::state::ViewerState;

pub struct Tabs {
    tabs: Vec<ViewerState>,
    active: usize,
}

impl Tabs {
    /// Create a tab set (must be non-empty; the caller guarantees at least one document).
    pub fn new(tabs: Vec<ViewerState>) -> Self {
        debug_assert!(!tabs.is_empty(), "Tabs must hold at least one document");
        Tabs { tabs, active: 0 }
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn active(&self) -> &ViewerState {
        &self.tabs[self.active]
    }

    pub fn active_mut(&mut self) -> &mut ViewerState {
        &mut self.tabs[self.active]
    }

    /// Mutable access to a tab by index (for patching a highlight result into any tab).
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut ViewerState> {
        self.tabs.get_mut(idx)
    }

    /// Switch to the next / previous tab (wrapping).
    pub fn next(&mut self) {
        if self.tabs.len() > 1 {
            self.active = (self.active + 1) % self.tabs.len();
        }
    }

    pub fn prev(&mut self) {
        if self.tabs.len() > 1 {
            self.active = (self.active + self.tabs.len() - 1) % self.tabs.len();
        }
    }

    /// Re-layout every tab for a new terminal size (they all share the viewport).
    pub fn resize_all(&mut self, width: usize, height: usize) {
        for t in &mut self.tabs {
            t.on_resize(width, height);
        }
    }

    /// Canonical paths of all tabs backed by a file (for filesystem watching).
    pub fn paths(&self) -> Vec<PathBuf> {
        self.tabs
            .iter()
            .filter_map(|t| t.canonical_path())
            .collect()
    }

    /// Reload every tab whose file is `path`. Returns whether the *active* tab changed — the
    /// only case that needs a repaint now (background tabs just refresh in place and show fresh
    /// content when next selected).
    pub fn reload_path(&mut self, path: &Path) -> bool {
        let active = self.active;
        let mut active_changed = false;
        for (i, t) in self.tabs.iter_mut().enumerate() {
            if t.canonical_path().as_deref() == Some(path) && t.reload() && i == active {
                active_changed = true;
            }
        }
        active_changed
    }

    /// The tab-bar label, e.g. `[2/3 guide.md]` — `None` for a single tab.
    pub fn label(&self) -> Option<String> {
        (self.tabs.len() > 1).then(|| {
            format!(
                "[{}/{} {}]",
                self.active + 1,
                self.tabs.len(),
                self.active().name()
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::md::parse::parse;

    fn tabs(n: usize) -> Tabs {
        let states = (0..n)
            .map(|i| ViewerState::new(parse(&format!("# doc{i}")).blocks, 80, 24, None, false))
            .collect();
        Tabs::new(states)
    }

    #[test]
    fn single_tab_has_no_label_and_no_cycle() {
        let mut t = tabs(1);
        assert_eq!(t.label(), None);
        t.next();
        assert_eq!(t.active_index(), 0); // nowhere to go
    }

    #[test]
    fn cycles_forward_and_back_wrapping() {
        let mut t = tabs(3);
        assert_eq!(t.active_index(), 0);
        t.next();
        assert_eq!(t.active_index(), 1);
        t.next();
        t.next(); // wrap 2 → 0
        assert_eq!(t.active_index(), 0);
        t.prev(); // wrap 0 → 2
        assert_eq!(t.active_index(), 2);
    }

    #[test]
    fn label_shows_position_and_count() {
        let mut t = tabs(3);
        t.next();
        let label = t.label().unwrap();
        assert!(label.starts_with("[2/3 "));
    }

    #[test]
    fn per_tab_scroll_is_independent() {
        // scrolling one tab doesn't move another (each keeps its own ViewerState).
        let mut states: Vec<ViewerState> = (0..2)
            .map(|_| {
                let md: String = (0..50).map(|i| format!("line{i}\n\n")).collect();
                ViewerState::new(parse(&md).blocks, 80, 10, None, false)
            })
            .collect();
        states[0].scroll(20);
        let t = Tabs::new(states);
        assert_eq!(t.active().top, 20);
    }
}

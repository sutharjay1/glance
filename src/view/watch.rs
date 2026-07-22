//! Auto-reload: watch open files and coalesce a burst of save events into one reload each.
//!
//! Two pieces, split so the tricky part is testable:
//! - [`Debouncer`] — pure, clock-injected coalescing. A path becomes *ready* only after it has
//!   been quiet (no new event) for `debounce`, so an editor's write→rename→truncate burst yields
//!   a single reload instead of several mid-write flashes. Unit-tested without sleeping.
//! - [`FileWatcher`] — thin `notify` wiring. It watches the *parent directories* of the files
//!   (editors replace files via rename, which breaks a watch on the file inode itself) and
//!   forwards only events whose canonical path is one we care about. Not unit-tested (real I/O).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

/// Coalesces filesystem events per path into one reload once the path has been quiet long enough.
pub struct Debouncer {
    debounce: Duration,
    /// Path → time of its most recent event.
    dirty: HashMap<PathBuf, Instant>,
}

impl Debouncer {
    pub fn new(debounce: Duration) -> Self {
        Debouncer {
            debounce,
            dirty: HashMap::new(),
        }
    }

    /// Record that `path` changed at `now` (resets its quiet timer).
    pub fn mark(&mut self, path: PathBuf, now: Instant) {
        self.dirty.insert(path, now);
    }

    pub fn is_empty(&self) -> bool {
        self.dirty.is_empty()
    }

    /// Paths quiet for at least `debounce` as of `now`, removed from the pending set.
    pub fn ready(&mut self, now: Instant) -> Vec<PathBuf> {
        let ready: Vec<PathBuf> = self
            .dirty
            .iter()
            .filter(|(_, &t)| now.saturating_duration_since(t) >= self.debounce)
            .map(|(p, _)| p.clone())
            .collect();
        for p in &ready {
            self.dirty.remove(p);
        }
        ready
    }
}

/// Watches a set of files and reports the canonical paths that change.
pub struct FileWatcher {
    // Kept alive for the lifetime of the watch; dropping it stops watching.
    _watcher: RecommendedWatcher,
    rx: Receiver<PathBuf>,
}

impl FileWatcher {
    /// Start watching `paths` (canonicalized). Returns `Ok(None)` when there's nothing watchable.
    pub fn new(paths: &[PathBuf]) -> notify::Result<Option<Self>> {
        let watched: Vec<PathBuf> = paths
            .iter()
            .filter_map(|p| std::fs::canonicalize(p).ok())
            .collect();
        if watched.is_empty() {
            return Ok(None);
        }
        let set: HashSet<PathBuf> = watched.iter().cloned().collect();

        let (tx, rx) = mpsc::channel();
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                if let Ok(ev) = res {
                    for p in ev.paths {
                        // Canonicalize best-effort; a briefly-missing file (mid-rename) falls back
                        // to the raw path so we still match on the following settle event.
                        let cp = std::fs::canonicalize(&p).unwrap_or(p);
                        if set.contains(&cp) {
                            let _ = tx.send(cp);
                        }
                    }
                }
            })?;

        // Watch each unique parent directory non-recursively.
        let mut dirs: HashSet<&Path> = HashSet::new();
        for p in &watched {
            if let Some(dir) = p.parent() {
                dirs.insert(dir);
            }
        }
        for dir in dirs {
            watcher.watch(dir, RecursiveMode::NonRecursive)?;
        }

        Ok(Some(FileWatcher {
            _watcher: watcher,
            rx,
        }))
    }

    /// Drain all pending change events without blocking.
    pub fn drain(&self) -> impl Iterator<Item = PathBuf> + '_ {
        self.rx.try_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_not_ready_until_quiet() {
        let mut d = Debouncer::new(Duration::from_millis(120));
        let t0 = Instant::now();
        let p = PathBuf::from("/tmp/a.md");
        d.mark(p.clone(), t0);
        // Still within the debounce window → nothing ready.
        assert!(d.ready(t0 + Duration::from_millis(50)).is_empty());
        assert!(!d.is_empty());
        // Quiet long enough → ready, and consumed.
        assert_eq!(d.ready(t0 + Duration::from_millis(200)), vec![p]);
        assert!(d.is_empty());
    }

    #[test]
    fn later_event_resets_the_quiet_timer() {
        let mut d = Debouncer::new(Duration::from_millis(120));
        let t0 = Instant::now();
        let p = PathBuf::from("/tmp/a.md");
        d.mark(p.clone(), t0);
        // A fresh event at t0+100 pushes readiness out to t0+220.
        d.mark(p.clone(), t0 + Duration::from_millis(100));
        assert!(d.ready(t0 + Duration::from_millis(150)).is_empty());
        assert_eq!(d.ready(t0 + Duration::from_millis(230)), vec![p]);
    }

    #[test]
    fn independent_paths_ready_independently() {
        let mut d = Debouncer::new(Duration::from_millis(120));
        let t0 = Instant::now();
        let a = PathBuf::from("/tmp/a.md");
        let b = PathBuf::from("/tmp/b.md");
        d.mark(a.clone(), t0);
        d.mark(b.clone(), t0 + Duration::from_millis(100));
        // Only `a` has been quiet 120ms by t0+150.
        assert_eq!(d.ready(t0 + Duration::from_millis(150)), vec![a]);
        assert!(!d.is_empty());
        assert_eq!(d.ready(t0 + Duration::from_millis(250)), vec![b]);
    }
}

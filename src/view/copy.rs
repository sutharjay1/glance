//! The copy stack — clipboard that works everywhere (fixes reference weakness #3).
//!
//! Strategy: **OSC 52 first** (base64 escape sequence; works over SSH, inside tmux, in every
//! modern terminal, no external tool). For payloads above the OSC 52 cap (~100 KB, where some
//! terminals silently drop it) fall back to the platform's native clipboard command. Every
//! spawned command is error-handled — a missing `xclip` degrades, it never crashes the viewer.
//!
//! [`copy`] returns a [`Copied`]: for OSC 52 it carries the escape sequence for the caller to
//! write to the terminal; for a native command it just names the method that succeeded.

use crate::term::osc;

/// The outcome of a successful copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Copied {
    /// The OSC 52 sequence to write to the terminal, if that path was used.
    pub osc52: Option<String>,
    /// Human-readable name of the method that worked (for the toast).
    pub method: &'static str,
}

/// Copy `text` to the clipboard. Returns `None` only if every method failed.
pub fn copy(text: &str) -> Option<Copied> {
    if let Some(seq) = osc::clipboard_within(text, osc::OSC52_MAX_ENCODED) {
        return Some(Copied {
            osc52: Some(seq),
            method: "OSC 52",
        });
    }
    // Too large for OSC 52 → native command.
    native_copy(text).map(|method| Copied {
        osc52: None,
        method,
    })
}

/// Try the platform's native clipboard commands in order; return the one that succeeded.
fn native_copy(text: &str) -> Option<&'static str> {
    for (name, program, args) in candidates() {
        if pipe_to(program, &args, text) {
            return Some(name);
        }
    }
    None
}

/// Candidate `(name, program, args)` clipboard commands, most-preferred first, per platform.
fn candidates() -> Vec<(&'static str, &'static str, Vec<&'static str>)> {
    #[cfg(target_os = "macos")]
    {
        vec![("pbcopy", "pbcopy", vec![])]
    }
    #[cfg(target_os = "windows")]
    {
        vec![("clip", "clip", vec![])]
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        vec![
            ("wl-copy", "wl-copy", vec![]),
            ("xclip", "xclip", vec!["-selection", "clipboard"]),
            ("xsel", "xsel", vec!["--clipboard", "--input"]),
        ]
    }
}

/// Spawn `program args`, write `text` to its stdin, and report success. Any error (missing
/// binary, broken pipe, non-zero exit) returns `false` — never panics.
fn pipe_to(program: &str, args: &[&str], text: &str) -> bool {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let Ok(mut child) = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    if let Some(mut stdin) = child.stdin.take() {
        if stdin.write_all(text.as_bytes()).is_err() {
            let _ = child.wait();
            return false;
        }
        // drop closes stdin so the command finishes
    }
    matches!(child.wait(), Ok(status) if status.success())
}

/// Toast text for a copy outcome: `outcome` names *what* was copied ("code block").
pub fn toast(what: &str, result: Option<&Copied>) -> String {
    match result {
        Some(c) => format!("copied {what} · {}", c.method),
        None => format!("copy failed ({what})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_text_uses_osc52() {
        let c = copy("hello").expect("copy should succeed via OSC 52");
        assert_eq!(c.method, "OSC 52");
        let seq = c.osc52.expect("OSC 52 carries a sequence");
        assert!(seq.starts_with("\x1b]52;c;"));
        assert!(seq.ends_with("\x1b\\"));
    }

    #[test]
    fn oversized_text_skips_osc52() {
        // Larger than the OSC 52 cap → osc52 path returns None, so `copy` tries native. On CI a
        // native clipboard may be absent, so we only assert it did NOT use OSC 52.
        let big = "x".repeat(osc::OSC52_MAX_ENCODED * 2);
        if let Some(c) = copy(&big) {
            assert_ne!(c.method, "OSC 52");
            assert!(c.osc52.is_none());
        }
    }

    #[test]
    fn toast_text() {
        let c = copy("hi").unwrap();
        assert_eq!(toast("document", Some(&c)), "copied document · OSC 52");
        assert_eq!(toast("path", None), "copy failed (path)");
    }
}

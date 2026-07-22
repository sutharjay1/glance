//! Opening links: classifying a URL and launching the platform opener.
//!
//! [`classify`] decides whether a link is a web URL, a local file (resolved against the current
//! document's directory), or something else. [`open_command`] returns the platform's opener
//! invocation; [`open_url`] runs it with stdio nulled and errors swallowed — a missing `xdg-open`
//! must degrade, never crash the viewer (plan §4.1).

use std::path::{Path, PathBuf};

/// What a link points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    /// An http(s) URL to open in the browser.
    Url(String),
    /// A local file (resolved), followed in-app if it's markdown, else opened externally.
    LocalFile(PathBuf),
    /// Anything else (mailto:, other schemes, fragments) — opened externally as-is.
    Other(String),
}

/// Classify `url`, resolving relative local paths against `base_dir` (the current file's folder).
pub fn classify(url: &str, base_dir: Option<&Path>) -> LinkTarget {
    if url.starts_with("http://") || url.starts_with("https://") {
        return LinkTarget::Url(url.to_string());
    }
    // Any explicit scheme (mailto:, ftp:, tel:) or in-page fragment → external/other.
    if url.starts_with('#') || url.contains("://") || url.starts_with("mailto:") {
        return LinkTarget::Other(url.to_string());
    }
    let p = Path::new(url);
    let resolved = match base_dir {
        Some(d) if p.is_relative() => d.join(p),
        _ => p.to_path_buf(),
    };
    LinkTarget::LocalFile(resolved)
}

/// True if a local file should be followed *inside* glance (a markdown file) vs opened externally.
pub fn is_markdown(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("md" | "markdown" | "mdown" | "mkd")
    )
}

/// The platform opener command as `(program, args)`.
pub fn open_command(target: &str) -> (&'static str, Vec<String>) {
    #[cfg(target_os = "macos")]
    {
        ("open", vec![target.to_string()])
    }
    #[cfg(target_os = "windows")]
    {
        // `start` needs an empty title argument first.
        (
            "cmd",
            vec![
                "/C".into(),
                "start".into(),
                String::new(),
                target.to_string(),
            ],
        )
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        ("xdg-open", vec![target.to_string()])
    }
}

/// Launch the platform opener for `target`. Errors (e.g. the opener binary is missing) are
/// returned, not panicked — the caller shows a toast and carries on.
pub fn open_url(target: &str) -> std::io::Result<()> {
    use std::process::{Command, Stdio};
    let (program, args) = open_command(target);
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_is_url() {
        assert_eq!(
            classify("https://example.com", None),
            LinkTarget::Url("https://example.com".into())
        );
    }

    #[test]
    fn relative_path_resolves_against_base() {
        let t = classify("../docs/guide.md", Some(Path::new("/home/u/proj/readme")));
        match t {
            LinkTarget::LocalFile(p) => {
                assert!(p.ends_with("guide.md"));
                assert!(p.to_string_lossy().contains("docs"));
            }
            other => panic!("expected LocalFile, got {other:?}"),
        }
    }

    #[test]
    fn mailto_and_fragment_are_other() {
        assert!(matches!(
            classify("mailto:a@b.com", None),
            LinkTarget::Other(_)
        ));
        assert!(matches!(classify("#section", None), LinkTarget::Other(_)));
    }

    #[test]
    fn markdown_detection() {
        assert!(is_markdown(Path::new("a/b.md")));
        assert!(is_markdown(Path::new("README.Markdown")));
        assert!(!is_markdown(Path::new("image.png")));
        assert!(!is_markdown(Path::new("noext")));
    }

    #[test]
    fn open_command_includes_target_and_program() {
        let (prog, args) = open_command("https://x.io");
        assert!(!prog.is_empty());
        assert!(args.iter().any(|a| a == "https://x.io"));
    }
}

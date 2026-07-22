//! CLI parsing (via `lexopt` — tiny and fast, matching the small-binary priority).
//!
//! Produces an [`Args`] with everything the spec's flag list needs. Values are optional so
//! `run` can layer them over the config file (CLI wins). Kept independent of I/O so it is
//! unit-testable.

use std::ffi::OsString;

/// Parsed command-line arguments. Fields left `None`/`false` when absent so config can fill in.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Args {
    pub files: Vec<String>,
    pub theme: Option<String>,
    pub width: Option<usize>,
    pub line_numbers: bool,
    pub slides: bool,
    pub follow: bool,
    pub export: Option<String>,
    pub no_color: bool,
    pub pipe: bool,
    pub help: bool,
    pub version: bool,
}

/// Parse args (excluding the binary name). A dummy program name is prepended for lexopt.
pub fn parse(args: &[String]) -> Result<Args, lexopt::Error> {
    use lexopt::prelude::*;

    let iter = std::iter::once(OsString::from("glance")).chain(args.iter().map(OsString::from));
    let mut parser = lexopt::Parser::from_iter(iter);
    let mut out = Args::default();

    while let Some(arg) = parser.next()? {
        match arg {
            Short('T') | Long("theme") => out.theme = Some(parser.value()?.string()?),
            Short('w') | Long("width") => out.width = Some(parser.value()?.parse()?),
            Short('l') | Long("line-numbers") => out.line_numbers = true,
            Short('s') | Long("slides") => out.slides = true,
            Short('f') | Long("follow") => out.follow = true,
            Long("export") => out.export = Some(parser.value()?.string()?),
            Long("no-color") => out.no_color = true,
            Long("pipe") => out.pipe = true,
            Short('h') | Long("help") => out.help = true,
            Short('V') | Long("version") => out.version = true,
            Value(v) => out.files.push(v.string()?),
            _ => return Err(arg.unexpected()),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(args: &[&str]) -> Args {
        parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
    }

    #[test]
    fn empty_args() {
        let a = parse_ok(&[]);
        assert!(a.files.is_empty());
        assert_eq!(a.theme, None);
        assert!(!a.help);
    }

    #[test]
    fn files_collected() {
        let a = parse_ok(&["a.md", "b.md"]);
        assert_eq!(a.files, vec!["a.md", "b.md"]);
    }

    #[test]
    fn short_and_long_flags() {
        let a = parse_ok(&["-T", "light", "--width", "72", "-l", "README.md"]);
        assert_eq!(a.theme.as_deref(), Some("light"));
        assert_eq!(a.width, Some(72));
        assert!(a.line_numbers);
        assert_eq!(a.files, vec!["README.md"]);
    }

    #[test]
    fn equals_form_and_booleans() {
        let a = parse_ok(&["--theme=dark", "--no-color", "--pipe", "--slides", "-f"]);
        assert_eq!(a.theme.as_deref(), Some("dark"));
        assert!(a.no_color && a.pipe && a.slides && a.follow);
    }

    #[test]
    fn help_and_version() {
        assert!(parse_ok(&["-h"]).help);
        assert!(parse_ok(&["--version"]).version);
    }

    #[test]
    fn export_value() {
        assert_eq!(
            parse_ok(&["--export", "html", "x.md"]).export.as_deref(),
            Some("html")
        );
    }

    #[test]
    fn bad_width_is_error() {
        assert!(parse(&["-w".into(), "notanumber".into()]).is_err());
    }

    #[test]
    fn unknown_flag_is_error() {
        assert!(parse(&["--frobnicate".to_string()]).is_err());
    }
}

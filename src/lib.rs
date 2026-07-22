//! glance — a fast terminal markdown viewer.
//!
//! Library crate: all logic lives here so it is unit-testable without spawning the binary
//! (`src/main.rs` is a thin wrapper). This mirrors the "never run `main()` on import" rule
//! from the plan — the entry point holds no logic. See `ROADMAP.md` for the phase plan and
//! `docs/adr/` for the decisions behind this structure.

pub mod md;
pub mod paint;
pub mod style;
pub mod term;
pub mod text;
pub mod theme;
pub mod view;

use std::io::IsTerminal;
use term::caps::{Capabilities, ColorDepth};

/// Crate version, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
glance — a fast terminal markdown viewer

USAGE:
    glance [OPTIONS] [FILES]...

OPTIONS:
    -T, --theme <dark|light>    theme
    -w, --width <N>             display width override (0 = auto)
    -s, --slides                slide mode
    -l, --line-numbers          line numbers in code blocks
    -f, --follow                follow file/stdin as it grows
        --export <html>         export instead of viewing
        --no-color              disable ANSI colors
        --pipe                  force non-interactive styled output
    -h, --help                  print help
    -V, --version               print version
";

/// Run glance with the given CLI args (excluding `argv[0]`). Returns a process exit code.
///
/// Dispatch: `-V/-h` short-circuit; then a file argument is read and either shown in the
/// **interactive TUI** (stdout is a TTY and not `--pipe`) or rendered once to stdout in **pipe
/// mode** (piped/`--no-color`/`--pipe` → clean plain or styled text). Full CLI parsing,
/// multi-file, `--export`, and the streaming stdin path are later modules.
pub fn run(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("glance {VERSION}");
        return 0;
    }
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{HELP}");
        return 0;
    }

    let no_color = args.iter().any(|a| a == "--no-color");
    let force_pipe = args.iter().any(|a| a == "--pipe");
    let theme_name = flag_value(args, "-T", "--theme").unwrap_or("dark");
    let theme = theme::by_name(theme_name);

    let Some(path) = args.iter().find(|a| !a.starts_with('-')) else {
        eprintln!("glance {VERSION}: no input file. Try --help.");
        return 0;
    };

    let input = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("glance: {path}: {e}");
            return 1;
        }
    };

    let is_tty = std::io::stdout().is_terminal();

    // Interactive TUI when we own a terminal and weren't asked to pipe.
    if is_tty && !force_pipe {
        let depth = if no_color {
            ColorDepth::None
        } else {
            Capabilities::from_env(false).color
        };
        let blocks = md::parse::parse(&input).blocks;
        return match view::app::run(blocks, theme, depth, true) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("glance: {e}");
                1
            }
        };
    }

    // Pipe mode: render once to stdout. Plain when not a TTY or --no-color.
    let depth = if no_color || !is_tty {
        ColorDepth::None
    } else {
        Capabilities::from_env(false).color
    };
    print!(
        "{}",
        paint::render_document(&input, 80, &theme, depth, false)
    );
    0
}

/// Read the value of a `-x`/`--long` flag (space-separated form), if present.
fn flag_value<'a>(args: &'a [String], short: &str, long: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == short || a == long)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_flag_exits_zero() {
        assert_eq!(run(&["--version".to_string()]), 0);
        assert_eq!(run(&["-V".to_string()]), 0);
    }

    #[test]
    fn help_flag_exits_zero() {
        assert_eq!(run(&["--help".to_string()]), 0);
    }

    #[test]
    fn no_args_does_not_panic() {
        assert_eq!(run(&[]), 0);
    }
}

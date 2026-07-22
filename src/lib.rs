//! glance — a fast terminal markdown viewer.
//!
//! Library crate: all logic lives here so it is unit-testable without spawning the binary
//! (`src/main.rs` is a thin wrapper). This mirrors the "never run `main()` on import" rule
//! from the plan — the entry point holds no logic. See `ROADMAP.md` for the phase plan and
//! `docs/adr/` for the decisions behind this structure.

pub mod cli;
pub mod config;
pub mod fuzzy;
pub mod md;
pub mod open;
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
    let parsed = match cli::parse(args) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("glance: {e}");
            return 2;
        }
    };
    if parsed.version {
        println!("glance {VERSION}");
        return 0;
    }
    if parsed.help {
        print!("{HELP}");
        return 0;
    }

    // CLI overrides config; config overrides built-in defaults.
    let cfg = config::load();
    let theme_name = parsed.theme.as_deref().unwrap_or(&cfg.theme);
    let theme = theme::by_name(theme_name);
    let theme_dark = theme_name != "light";
    let line_numbers = parsed.line_numbers || cfg.line_numbers;
    let no_color = parsed.no_color;
    let width_override = parsed.width.or((cfg.width > 0).then_some(cfg.width));

    let Some(path) = parsed.files.first() else {
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

    // `--timing`: measure launch→first-paint (parse + viewport layout) and exit. This is the
    // number the perf gate guards — it must stay well under 80 ms (ADR 0004).
    if parsed.timing {
        let t0 = std::time::Instant::now();
        let blocks = md::parse::parse(&input).blocks;
        let t_parse = t0.elapsed();
        let doc = md::layout::layout_document(&blocks, width_override.unwrap_or(80), line_numbers);
        let t_total = t0.elapsed();
        eprintln!(
            "glance --timing: parse {:.2?}, parse+layout {:.2?} ({} lines, {} bytes)",
            t_parse,
            t_total,
            doc.len(),
            input.len()
        );
        return 0;
    }

    let is_tty = std::io::stdout().is_terminal();

    // Interactive TUI when we own a terminal and weren't asked to pipe.
    if is_tty && !parsed.pipe {
        let depth = if no_color {
            ColorDepth::None
        } else {
            Capabilities::from_env(false).color
        };
        // One tab per file: the first file is already read into `input`; read the rest here,
        // skipping any that fail (with a warning) so one bad path doesn't sink the session.
        let mut docs: Vec<(Vec<md::parse::Block>, Option<std::path::PathBuf>)> = Vec::new();
        docs.push((
            md::parse::parse(&input).blocks,
            Some(std::path::PathBuf::from(path)),
        ));
        for extra in &parsed.files[1..] {
            match std::fs::read_to_string(extra) {
                Ok(s) => docs.push((
                    md::parse::parse(&s).blocks,
                    Some(std::path::PathBuf::from(extra)),
                )),
                Err(e) => eprintln!("glance: {extra}: {e} (skipped)"),
            }
        }
        return match view::app::run(docs, theme_dark, depth, true, width_override, line_numbers) {
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
    let width = width_override.filter(|&w| w > 0).unwrap_or(80);
    print!(
        "{}",
        paint::render_document(&input, width, &theme, depth, false, line_numbers)
    );
    0
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

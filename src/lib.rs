//! glance — a fast terminal markdown viewer.
//!
//! Library crate: all logic lives here so it is unit-testable without spawning the binary
//! (`src/main.rs` is a thin wrapper). This mirrors the "never run `main()` on import" rule
//! from the plan — the entry point holds no logic. See `ROADMAP.md` for the phase plan and
//! `docs/adr/` for the decisions behind this structure.

pub mod cli;
pub mod config;
pub mod export;
pub mod fuzzy;
pub mod md;
pub mod open;
pub mod paint;
pub mod stream;
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

/// Read the document from the first file argument, else from piped stdin. Returns `None` (with a
/// message on a file error) when there's no readable source. Used by the whole-document modes
/// (slides, export) that never stream.
fn read_source(parsed: &cli::Args, stdin_piped: bool) -> Option<String> {
    if let Some(path) = parsed.files.first() {
        match std::fs::read_to_string(path) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("glance: {path}: {e}");
                None
            }
        }
    } else if stdin_piped {
        Some(std::io::read_to_string(std::io::stdin()).unwrap_or_default())
    } else {
        None
    }
}

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
    // The user "chose" a theme iff they passed `-T` or set `theme` in config; only then does an
    // explicit choice suppress OSC 11 auto-detection (applied in the interactive branch below).
    let theme_explicit = parsed.theme.is_some() || config::theme_is_configured();
    let line_numbers = parsed.line_numbers || cfg.line_numbers;
    let no_color = parsed.no_color;
    let width_override = parsed.width.or((cfg.width > 0).then_some(cfg.width));

    let stdin_piped = !std::io::stdin().is_terminal();
    let stdout_tty = std::io::stdout().is_terminal();

    // Export (`--export html`): render the whole document to a self-contained HTML file on stdout
    // and exit — no TUI, regardless of whether stdout is a terminal.
    if let Some(fmt) = parsed.export.as_deref() {
        if !fmt.eq_ignore_ascii_case("html") {
            eprintln!("glance: unknown export format '{fmt}' (supported: html)");
            return 2;
        }
        let Some(input) = read_source(&parsed, stdin_piped) else {
            eprintln!("glance {VERSION}: no input file. Try --help.");
            return 0;
        };
        // Dark unless the user explicitly chose the light theme.
        print!("{}", export::to_html(&input, theme_name != "light"));
        return 0;
    }

    // Slide mode (`-s`): present the whole document as slides. Needs the full document, so it reads
    // its source completely (never streams).
    if parsed.slides && stdout_tty && !parsed.pipe {
        let Some(input) = read_source(&parsed, stdin_piped) else {
            eprintln!("glance {VERSION}: no input file. Try --help.");
            return 0;
        };
        let depth = if no_color {
            ColorDepth::None
        } else {
            Capabilities::from_env(false).color
        };
        let theme_dark = if theme_explicit {
            theme_dark
        } else {
            term::osc::detect_dark_background().unwrap_or(theme_dark)
        };
        let slides = view::slides::split_slides(&md::parse::parse(&input).blocks);
        return match view::app::run_slides(slides, theme_dark, depth, true, width_override) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("glance: {e}");
                1
            }
        };
    }

    // Streaming mode (the `llm | glance` demo): piped stdin + interactive stdout + no file → live-
    // render stdin as it arrives. Keys still come from /dev/tty (crossterm reads the tty, not fd 0).
    if stdin_piped
        && stdout_tty
        && !parsed.pipe
        && !parsed.timing
        && !parsed.slides
        && parsed.files.is_empty()
    {
        let depth = if no_color {
            ColorDepth::None
        } else {
            Capabilities::from_env(false).color
        };
        // OSC 11 auto-theme self-skips here (stdin isn't the tty), falling back to the default.
        let theme_dark = if theme_explicit {
            theme_dark
        } else {
            term::osc::detect_dark_background().unwrap_or(theme_dark)
        };
        let reader = stream::StreamReader::spawn_stdin();
        let docs = vec![(Vec::new(), None)]; // starts empty; fills from the stream
        return match view::app::run(
            docs,
            theme_dark,
            depth,
            true,
            width_override,
            line_numbers,
            Some(reader),
        ) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("glance: {e}");
                1
            }
        };
    }

    // Remaining paths (timing / interactive / pipe) need a file. Piped stdin with an interactive
    // stdout was already taken by the streaming branch; `glance < x.md` in a terminal streams too.
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

    // Interactive TUI when we own a terminal, weren't asked to pipe, and have file(s) to open.
    // (Interactive stdin is the streaming case, already handled above.)
    if is_tty && !parsed.pipe {
        let depth = if no_color {
            ColorDepth::None
        } else {
            Capabilities::from_env(false).color
        };
        // Auto-pick dark/light from the terminal background (OSC 11) unless the user chose a
        // theme explicitly. Runs before the alt-screen (inside `app::run`). Falls back to the
        // explicit/default value when the terminal doesn't answer.
        let theme_dark = if theme_explicit {
            theme_dark
        } else {
            term::osc::detect_dark_background().unwrap_or(theme_dark)
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
        return match view::app::run(
            docs,
            theme_dark,
            depth,
            true,
            width_override,
            line_numbers,
            None,
        ) {
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

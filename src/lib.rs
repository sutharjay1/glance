//! glance — a fast terminal markdown viewer.
//!
//! Library crate: all logic lives here so it is unit-testable without spawning the binary
//! (`src/main.rs` is a thin wrapper). This mirrors the "never run `main()` on import" rule
//! from the plan — the entry point holds no logic. See `ROADMAP.md` for the phase plan and
//! `docs/adr/` for the decisions behind this structure.

pub mod md;
pub mod term;
pub mod text;

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
/// Phase 0.5 scaffold: only `-V/--version` and `-h/--help` are wired. The viewer core
/// (arg parsing, parse → layout → render) lands in Phase 1 (JAY-91).
pub fn run(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("glance {VERSION}");
        return 0;
    }
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{HELP}");
        return 0;
    }
    eprintln!("glance {VERSION}: viewer core not yet implemented (Phase 1 / JAY-91). Try --help.");
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

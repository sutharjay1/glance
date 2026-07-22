//! Configuration file: `~/.config/glance/config.toml`.
//!
//! Uses the same keys as the reference (`theme`, `line_numbers`, `width`) so a user's mdterm
//! config migrates unchanged. Missing keys fall back to defaults; a missing or malformed file is
//! silently ignored (defaults). CLI flags override config (handled in `run`).

use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: String,
    pub line_numbers: bool,
    /// Display width override; `0` means auto (use the terminal width).
    pub width: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            theme: "dark".to_string(),
            line_numbers: false,
            width: 0,
        }
    }
}

/// Parse config from a TOML string, falling back to defaults on any error.
pub fn parse_str(s: &str) -> Config {
    toml::from_str(s).unwrap_or_default()
}

/// The path to the config file (`$XDG_CONFIG_HOME/glance/config.toml`, else
/// `$HOME/.config/glance/config.toml`), if a home/config dir is known.
pub fn config_path() -> Option<PathBuf> {
    if let Ok(x) = std::env::var("XDG_CONFIG_HOME") {
        if !x.is_empty() {
            return Some(PathBuf::from(x).join("glance").join("config.toml"));
        }
    }
    std::env::var("HOME").ok().map(|h| {
        PathBuf::from(h)
            .join(".config")
            .join("glance")
            .join("config.toml")
    })
}

/// Load config from disk, or defaults if absent/unreadable/malformed.
pub fn load() -> Config {
    config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| parse_str(&s))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_empty() {
        assert_eq!(parse_str(""), Config::default());
        assert_eq!(parse_str("").theme, "dark");
    }

    #[test]
    fn parses_all_keys() {
        let c = parse_str("theme = \"light\"\nline_numbers = true\nwidth = 100\n");
        assert_eq!(c.theme, "light");
        assert!(c.line_numbers);
        assert_eq!(c.width, 100);
    }

    #[test]
    fn partial_config_keeps_other_defaults() {
        let c = parse_str("width = 72");
        assert_eq!(c.width, 72);
        assert_eq!(c.theme, "dark"); // untouched
        assert!(!c.line_numbers);
    }

    #[test]
    fn malformed_falls_back_to_defaults() {
        assert_eq!(
            parse_str("this is not = valid = toml ]["),
            Config::default()
        );
    }
}

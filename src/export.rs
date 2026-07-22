//! HTML export (`--export html`): render markdown to a self-contained, themed HTML document.
//!
//! Uses pulldown-cmark's HTML renderer on the original markdown, wrapped in a `<!doctype html>`
//! document with an **inlined** `<style>` derived from glance's theme — no external stylesheets,
//! fonts, or scripts, so the output is a single portable file. Non-interactive: printed to stdout.

use pulldown_cmark::{html, Options, Parser};

use crate::term::ansi::Rgb;
use crate::theme::{self, Theme};

fn hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

/// Render `markdown` to a complete themed HTML document (dark theme unless `dark` is false).
pub fn to_html(markdown: &str, dark: bool) -> String {
    // Match the viewer's GFM feature set so exported output mirrors what glance renders.
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_FOOTNOTES);
    let parser = Parser::new_ext(markdown, opts);
    let mut body = String::new();
    html::push_html(&mut body, parser);

    let t: Theme = if dark { theme::dark() } else { theme::light() };
    // Page/background tones aren't in the terminal theme (the terminal owns the bg), so pick
    // matching surfaces per mode; text + accents come from the theme so it stays on-brand.
    let (bg, surface, border) = if dark {
        ("#1e1e2e", "#181825", "#313244")
    } else {
        ("#ffffff", "#f4f4f5", "#e4e4e7")
    };
    let style = format!(
        "\
:root{{color-scheme:{scheme}}}
*{{box-sizing:border-box}}
body{{background:{bg};color:{fg};margin:0;padding:2.5rem 1rem;\
font:16px/1.65 -apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif}}
main{{max-width:46rem;margin:0 auto}}
h1,h2,h3,h4,h5,h6{{color:{heading};line-height:1.25;margin:2rem 0 .75rem;font-weight:650}}
h1{{font-size:2rem;border-bottom:1px solid {border};padding-bottom:.3rem}}
h2{{font-size:1.5rem;border-bottom:1px solid {border};padding-bottom:.3rem}}
a{{color:{link};text-decoration:none}}a:hover{{text-decoration:underline}}
code{{background:{surface};color:{code};padding:.15em .35em;border-radius:4px;\
font:.9em ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}}
pre{{background:{surface};padding:1rem;border-radius:8px;overflow:auto}}
pre code{{background:none;padding:0}}
blockquote{{margin:1rem 0;padding:.25rem 1rem;border-left:3px solid {accent};color:{fg}}}
table{{border-collapse:collapse;width:100%;margin:1rem 0}}
th,td{{border:1px solid {border};padding:.4rem .6rem;text-align:left}}
th{{background:{surface}}}
hr{{border:none;border-top:1px solid {border};margin:2rem 0}}
img{{max-width:100%}}
",
        scheme = if dark { "dark" } else { "light" },
        bg = bg,
        surface = surface,
        border = border,
        fg = hex(t.body),
        heading = hex(t.heading),
        link = hex(t.link),
        code = hex(t.code),
        accent = hex(t.accent),
    );

    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n\
<title>glance export</title>\n<style>\n{style}</style>\n</head>\n\
<body>\n<main>\n{body}</main>\n</body>\n</html>\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_self_contained_document() {
        let out = to_html("# Hello\n\nsome **bold** text\n", true);
        assert!(out.starts_with("<!doctype html>"));
        assert!(out.contains("<style>")); // inlined CSS
        assert!(out.contains("<h1>Hello</h1>")); // rendered content
        assert!(out.contains("<strong>bold</strong>"));
        // No external references — fully portable.
        assert!(!out.contains("http://"));
        assert!(!out.contains("https://"));
        assert!(!out.contains("<link"));
        assert!(!out.contains("<script"));
    }

    #[test]
    fn renders_tables_and_code_and_uses_accent() {
        let out = to_html("| a | b |\n|---|---|\n| 1 | 2 |\n\n```\ncode\n```\n", true);
        assert!(out.contains("<table>"));
        assert!(out.contains("<pre><code>"));
        assert!(out.contains("#ff5800")); // brand accent reaches the CSS
    }

    #[test]
    fn light_theme_switches_color_scheme() {
        let dark = to_html("# H", true);
        let light = to_html("# H", false);
        assert!(dark.contains("color-scheme:dark"));
        assert!(light.contains("color-scheme:light"));
    }
}

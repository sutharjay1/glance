//! syntect-backed syntax highlighting (Phase 3), mapping scopes → glance's semantic [`Role`]s.
//!
//! This is the *accurate* highlighter: syntect recognizes ~75 languages vs. the regex
//! micro-tokenizer's six. Crucially it is **never loaded on the startup/first-paint path** —
//! mdterm's #1 slowness is `SyntaxSet::load_defaults` at launch. The `SyntaxSet` is loaded
//! lazily (a `OnceLock`) on first `highlight` call, which only happens from the background
//! highlight worker after first paint; the micro-tokenizer remains the instant cold path.
//!
//! We use only syntect's *parser* (no theme engine): each token's scope stack is mapped to our
//! `Role` enum, so glance's own dark/light + OSC 11 theming colors the output. That keeps the
//! binary small (no embedded themes) and the palette consistent with the rest of the UI.

use std::sync::OnceLock;

use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxReference, SyntaxSet};

use crate::style::{Role, Span, Style};

/// The default syntax set, loaded once on first use (off the startup path — see module docs).
fn syntax_set() -> &'static SyntaxSet {
    static SS: OnceLock<SyntaxSet> = OnceLock::new();
    SS.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// Whether syntect recognizes `lang` (by name/alias or extension). Cheap — but note the first
/// call still triggers the one-time `SyntaxSet` load, so only call it from the worker.
pub fn supported(lang: &str) -> bool {
    find_syntax(syntax_set(), lang).is_some()
}

fn find_syntax<'a>(ss: &'a SyntaxSet, lang: &str) -> Option<&'a SyntaxReference> {
    let lang = lang.trim();
    if lang.is_empty() {
        return None;
    }
    ss.find_syntax_by_token(lang)
        .or_else(|| ss.find_syntax_by_extension(lang))
}

/// Highlight `code` in language `lang`, one inner `Vec<Span>` per line, with each span carrying a
/// semantic [`Role`] mapped from its syntect scope. Returns `None` when the language isn't
/// recognized — the caller then keeps the micro-tokenizer's output.
pub fn highlight(code: &str, lang: &str) -> Option<Vec<Vec<Span>>> {
    let ss = syntax_set();
    let syntax = find_syntax(ss, lang)?;
    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let mut lines = Vec::new();

    // syntect's default syntaxes are the "newlines" set: they expect each line to include its
    // trailing '\n'. `split_inclusive` preserves it; we trim it back off each emitted span.
    for line in code.split_inclusive('\n') {
        let ops = state.parse_line(line, ss).ok()?;
        let mut spans: Vec<Span> = Vec::new();
        let mut last = 0usize;
        for (idx, op) in ops {
            if idx > last {
                push_span(&mut spans, &line[last..idx], &stack);
            }
            if stack.apply(&op).is_err() {
                return None; // malformed scope stack — bail to the fallback highlighter
            }
            last = idx;
        }
        if last < line.len() {
            push_span(&mut spans, &line[last..], &stack);
        }
        lines.push(spans);
    }
    Some(lines)
}

/// Emit a span for `text` (minus any trailing newline) with the role of the current scope stack.
/// Empty/whitespace-only pieces still get a span so column positions are preserved.
fn push_span(spans: &mut Vec<Span>, text: &str, stack: &ScopeStack) {
    let text = text.strip_suffix('\n').unwrap_or(text);
    if text.is_empty() {
        return;
    }
    let role = role_for(stack);
    spans.push(Span::new(text.to_string(), Style::role(role)));
}

/// Map a scope stack to a role by scanning from the most specific (innermost) scope outward for
/// the first recognized category.
fn role_for(stack: &ScopeStack) -> Role {
    for scope in stack.as_slice().iter().rev() {
        if let Some(role) = role_from_scope(scope) {
            return role;
        }
    }
    Role::Body
}

/// Map a single syntect scope (e.g. `keyword.control.rust`) to a glance role, or `None` if it
/// isn't a category we color. Uses TextMate scope prefixes, which are stable across languages.
fn role_from_scope(scope: &Scope) -> Option<Role> {
    let s = scope.build_string();
    // Ordered most-specific first so `constant.numeric` beats a bare `constant`.
    if s.starts_with("comment") {
        Some(Role::Comment)
    } else if s.starts_with("string") || s.starts_with("constant.character") {
        Some(Role::Str)
    } else if s.starts_with("constant.numeric") {
        Some(Role::Number)
    } else if s.starts_with("entity.name.function") || s.starts_with("support.function") {
        Some(Role::Function)
    } else if s.starts_with("keyword")
        || s.starts_with("storage")
        || s.starts_with("constant.language")
    {
        Some(Role::Keyword)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Find the first span whose text (trimmed) equals `needle`, and return its role.
    fn role_of(lines: &[Vec<Span>], needle: &str) -> Option<Role> {
        lines
            .iter()
            .flatten()
            .find(|s| s.text.trim() == needle)
            .map(|s| s.style.role)
    }

    #[test]
    fn unknown_language_returns_none() {
        assert!(highlight("some text", "definitely-not-a-language").is_none());
        assert!(highlight("x", "").is_none());
    }

    #[test]
    fn rust_keywords_strings_and_comments_get_roles() {
        let code = "// a comment\nfn main() {\n    let s = \"hi\";\n}\n";
        let lines = highlight(code, "rust").expect("rust is supported");
        // Same number of lines back (incl. the closing brace line).
        assert_eq!(lines.len(), 4);
        assert_eq!(role_of(&lines, "fn"), Some(Role::Keyword));
        assert_eq!(role_of(&lines, "let"), Some(Role::Keyword));
        // The comment line maps to Comment (its text includes the // marker).
        assert!(lines[0].iter().all(|s| s.style.role == Role::Comment));
        // A string literal span carries the Str role.
        assert!(lines
            .iter()
            .flatten()
            .any(|s| s.text.contains("hi") && s.style.role == Role::Str));
    }

    #[test]
    fn language_aliases_resolve() {
        assert!(supported("rust"));
        assert!(supported("py"));
        assert!(supported("javascript"));
        // Highlighting via an alias works and preserves line count.
        let lines = highlight("x = 1\n", "python").unwrap();
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn numbers_are_tagged() {
        let lines = highlight("let n = 42;\n", "rust").unwrap();
        assert_eq!(role_of(&lines, "42"), Some(Role::Number));
    }
}

//! Instant syntax highlighting — a tiny language-agnostic lexer.
//!
//! This is the *cold-path* highlighter (plan feature #2): it loads no grammars and does no
//! allocation-heavy work, so it can run on the first-paint path without ever blocking it. A
//! richer syntect-based highlighter (Phase 3) patches over this for visible code blocks; this
//! stays as the always-instant fallback.
//!
//! It classifies each line into keyword / string / comment / number / function / plain runs
//! using a per-language [`LangSpec`]. It is intentionally approximate — good enough to read,
//! never a full parser.

use crate::style::{Role, Span, Style};

/// True if `lang` (or an alias) has a highlighting spec.
pub fn supported(lang: &str) -> bool {
    LangSpec::for_lang(lang).is_some()
}

/// Highlight `code` as `lang`, returning styled spans per line. Unknown languages return each
/// line as a single plain code span.
pub fn highlight(code: &str, lang: &str) -> Vec<Vec<Span>> {
    match LangSpec::for_lang(lang) {
        Some(spec) => {
            let mut in_block = false;
            code.lines()
                .map(|l| tokenize(l, &spec, &mut in_block))
                .collect()
        }
        None => code
            .lines()
            .map(|l| vec![code_span(l, Role::Body)])
            .collect(),
    }
}

fn code_span(text: &str, role: Role) -> Span {
    Span::new(
        text,
        Style {
            role,
            code: true,
            ..Default::default()
        },
    )
}

struct LangSpec {
    keywords: &'static [&'static str],
    line_comment: &'static [&'static str],
    block_comment: Option<(&'static str, &'static str)>,
    strings: &'static [char],
    /// Match keywords case-insensitively (SQL). Keyword tables are stored lowercase.
    ci_keywords: bool,
}

fn tokenize(line: &str, spec: &LangSpec, in_block: &mut bool) -> Vec<Span> {
    let chars: Vec<char> = line.chars().collect();
    let mut out: Vec<Span> = Vec::new();
    let mut i = 0;
    let mut plain_start = 0usize;

    // Flush accumulated plain text [plain_start, end) before emitting a classified token.
    macro_rules! flush_plain {
        ($end:expr) => {{
            if plain_start < $end {
                out.push(code_span(&collect(&chars, plain_start, $end), Role::Body));
            }
        }};
    }

    // Continue an open multi-line block comment.
    if *in_block {
        if let Some((_, close)) = spec.block_comment {
            if let Some(end) = find(&chars, 0, close) {
                let stop = end + close.chars().count();
                out.push(code_span(&collect(&chars, 0, stop), Role::Comment));
                *in_block = false;
                i = stop;
                plain_start = stop;
            } else {
                out.push(code_span(line, Role::Comment));
                return out;
            }
        }
    }

    while i < chars.len() {
        // line comment → rest of line
        if spec.line_comment.iter().any(|p| at(&chars, i, p)) {
            flush_plain!(i);
            out.push(code_span(&collect(&chars, i, chars.len()), Role::Comment));
            return out;
        }
        // block comment
        if let Some((open, close)) = spec.block_comment {
            if at(&chars, i, open) {
                flush_plain!(i);
                let from = i + open.chars().count();
                if let Some(end) = find(&chars, from, close) {
                    let stop = end + close.chars().count();
                    out.push(code_span(&collect(&chars, i, stop), Role::Comment));
                    i = stop;
                    plain_start = i;
                    continue;
                } else {
                    out.push(code_span(&collect(&chars, i, chars.len()), Role::Comment));
                    *in_block = true;
                    return out;
                }
            }
        }
        let c = chars[i];
        if spec.strings.contains(&c) {
            flush_plain!(i);
            let end = string_end(&chars, i, c);
            out.push(code_span(&collect(&chars, i, end), Role::Str));
            i = end;
            plain_start = i;
        } else if c.is_ascii_digit() && (i == plain_start || !is_ident(chars[i - 1])) {
            flush_plain!(i);
            let end = number_end(&chars, i);
            out.push(code_span(&collect(&chars, i, end), Role::Number));
            i = end;
            plain_start = i;
        } else if is_ident_start(c) {
            flush_plain!(i);
            let end = ident_end(&chars, i);
            let word = collect(&chars, i, end);
            let key = if spec.ci_keywords {
                word.to_ascii_lowercase()
            } else {
                word.clone()
            };
            let role = if spec.keywords.contains(&key.as_str()) {
                Role::Keyword
            } else if next_nonspace(&chars, end) == Some('(') {
                Role::Function
            } else {
                Role::Body
            };
            out.push(code_span(&word, role));
            i = end;
            plain_start = i;
        } else {
            i += 1; // accumulate into the plain run
        }
    }
    // trailing plain
    if plain_start < chars.len() {
        out.push(code_span(
            &collect(&chars, plain_start, chars.len()),
            Role::Body,
        ));
    }
    out
}

fn collect(chars: &[char], a: usize, b: usize) -> String {
    chars[a..b].iter().collect()
}

/// Does `chars[i..]` start with the string `pat`?
fn at(chars: &[char], i: usize, pat: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    i + p.len() <= chars.len() && chars[i..i + p.len()] == p[..]
}

/// First index ≥ `from` where `pat` starts, if any.
fn find(chars: &[char], from: usize, pat: &str) -> Option<usize> {
    (from..chars.len()).find(|&j| at(chars, j, pat))
}

fn string_end(chars: &[char], open: usize, delim: char) -> usize {
    let mut i = open + 1;
    while i < chars.len() {
        if chars[i] == '\\' {
            i += 2;
            continue;
        }
        if chars[i] == delim {
            return i + 1;
        }
        i += 1;
    }
    chars.len()
}

fn number_end(chars: &[char], start: usize) -> usize {
    let mut i = start;
    while i < chars.len()
        && (chars[i].is_ascii_alphanumeric() || chars[i] == '.' || chars[i] == '_')
    {
        i += 1;
    }
    i
}

fn ident_end(chars: &[char], start: usize) -> usize {
    let mut i = start;
    while i < chars.len() && is_ident(chars[i]) {
        i += 1;
    }
    i
}

fn next_nonspace(chars: &[char], from: usize) -> Option<char> {
    chars[from..].iter().copied().find(|c| !c.is_whitespace())
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '$'
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$'
}

static SLASH: &[&str] = &["//"];
static HASH: &[&str] = &["#"];
static DASHES: &[&str] = &["--"];
static STR_DQ_SQ_BT: &[char] = &['"', '\'', '`'];
static STR_DQ_SQ: &[char] = &['"', '\''];
static STR_DQ: &[char] = &['"'];
static STR_SQ: &[char] = &['\''];
const C_BLOCK: Option<(&str, &str)> = Some(("/*", "*/"));

impl LangSpec {
    fn for_lang(lang: &str) -> Option<LangSpec> {
        match lang.to_ascii_lowercase().as_str() {
            "js" | "javascript" | "ts" | "typescript" | "jsx" | "tsx" => Some(LangSpec {
                keywords: JS_KW,
                line_comment: SLASH,
                block_comment: C_BLOCK,
                strings: STR_DQ_SQ_BT,
                ci_keywords: false,
            }),
            "py" | "python" => Some(LangSpec {
                keywords: PY_KW,
                line_comment: HASH,
                block_comment: None,
                strings: STR_DQ_SQ,
                ci_keywords: false,
            }),
            "rs" | "rust" => Some(LangSpec {
                keywords: RUST_KW,
                line_comment: SLASH,
                block_comment: C_BLOCK,
                strings: STR_DQ,
                ci_keywords: false,
            }),
            "go" | "golang" => Some(LangSpec {
                keywords: GO_KW,
                line_comment: SLASH,
                block_comment: C_BLOCK,
                strings: STR_DQ_SQ_BT,
                ci_keywords: false,
            }),
            "sh" | "bash" | "shell" | "zsh" => Some(LangSpec {
                keywords: BASH_KW,
                line_comment: HASH,
                block_comment: None,
                strings: STR_DQ_SQ,
                ci_keywords: false,
            }),
            "sql" => Some(LangSpec {
                keywords: SQL_KW,
                line_comment: DASHES,
                block_comment: C_BLOCK,
                strings: STR_SQ,
                ci_keywords: true,
            }),
            _ => None,
        }
    }
}

static JS_KW: &[&str] = &[
    "const",
    "let",
    "var",
    "function",
    "return",
    "if",
    "else",
    "for",
    "while",
    "do",
    "switch",
    "case",
    "break",
    "continue",
    "new",
    "class",
    "extends",
    "super",
    "this",
    "import",
    "export",
    "from",
    "default",
    "async",
    "await",
    "try",
    "catch",
    "finally",
    "throw",
    "typeof",
    "instanceof",
    "in",
    "of",
    "null",
    "undefined",
    "true",
    "false",
    "void",
    "yield",
    "static",
    "get",
    "set",
    "interface",
    "type",
    "enum",
    "public",
    "private",
    "protected",
    "readonly",
];
static PY_KW: &[&str] = &[
    "def", "return", "if", "elif", "else", "for", "while", "break", "continue", "class", "import",
    "from", "as", "with", "try", "except", "finally", "raise", "lambda", "yield", "async", "await",
    "and", "or", "not", "is", "in", "None", "True", "False", "pass", "global", "nonlocal", "del",
    "assert", "self",
];
static RUST_KW: &[&str] = &[
    "fn", "let", "mut", "const", "static", "if", "else", "match", "for", "while", "loop", "break",
    "continue", "return", "struct", "enum", "trait", "impl", "pub", "use", "mod", "crate", "self",
    "super", "where", "as", "dyn", "ref", "move", "async", "await", "unsafe", "type", "true",
    "false", "Some", "None", "Ok", "Err", "Box", "Vec", "String", "Option", "Result",
];
static GO_KW: &[&str] = &[
    "func",
    "var",
    "const",
    "type",
    "struct",
    "interface",
    "map",
    "chan",
    "if",
    "else",
    "for",
    "range",
    "switch",
    "case",
    "default",
    "break",
    "continue",
    "return",
    "go",
    "defer",
    "select",
    "package",
    "import",
    "nil",
    "true",
    "false",
    "make",
    "new",
    "append",
    "len",
    "cap",
];
static BASH_KW: &[&str] = &[
    "if", "then", "else", "elif", "fi", "for", "while", "do", "done", "case", "esac", "function",
    "return", "in", "echo", "export", "local", "read", "set", "unset", "source", "exit",
];
static SQL_KW: &[&str] = &[
    "select",
    "from",
    "where",
    "insert",
    "into",
    "values",
    "update",
    "set",
    "delete",
    "create",
    "table",
    "drop",
    "alter",
    "add",
    "primary",
    "key",
    "foreign",
    "references",
    "join",
    "left",
    "right",
    "inner",
    "outer",
    "on",
    "group",
    "by",
    "order",
    "having",
    "limit",
    "offset",
    "and",
    "or",
    "not",
    "null",
    "as",
    "distinct",
    "count",
    "sum",
    "avg",
    "min",
    "max",
    "index",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn roles(spans: &[Span]) -> Vec<(Role, String)> {
        spans
            .iter()
            .map(|s| (s.style.role, s.text.clone()))
            .collect()
    }

    fn line0(code: &str, lang: &str) -> Vec<Span> {
        highlight(code, lang).into_iter().next().unwrap()
    }

    #[test]
    fn unsupported_lang_is_plain_code() {
        let spans = line0("anything here", "brainfuck");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style.role, Role::Body);
        assert!(spans[0].style.code);
    }

    #[test]
    fn all_spans_marked_code() {
        for s in line0("let x = 1; // hi", "rust") {
            assert!(s.style.code, "{:?} not marked code", s);
        }
    }

    #[test]
    fn rust_keyword_and_number_and_comment() {
        let r = roles(&line0("let x = 42; // note", "rust"));
        assert!(r.contains(&(Role::Keyword, "let".into())));
        assert!(r.contains(&(Role::Number, "42".into())));
        assert!(r
            .iter()
            .any(|(role, t)| *role == Role::Comment && t.contains("note")));
    }

    #[test]
    fn strings_are_classified() {
        let r = roles(&line0(r#"name = "value \" x""#, "python"));
        assert!(r
            .iter()
            .any(|(role, t)| *role == Role::Str && t.contains("value")));
    }

    #[test]
    fn function_calls_detected() {
        let r = roles(&line0("print(x)", "python"));
        assert!(r.contains(&(Role::Function, "print".into())));
    }

    #[test]
    fn hash_comment_for_bash_and_python() {
        assert!(roles(&line0("# a comment", "bash"))
            .iter()
            .any(|(role, _)| *role == Role::Comment));
        assert!(roles(&line0("x = 1  # trailing", "py"))
            .iter()
            .any(|(role, _)| *role == Role::Comment));
    }

    #[test]
    fn sql_keywords_and_dash_comment() {
        let r = roles(&line0("SELECT * FROM t -- all", "sql"));
        assert!(r
            .iter()
            .any(|(role, t)| *role == Role::Keyword && t.eq_ignore_ascii_case("select")));
        assert!(r.iter().any(|(role, _)| *role == Role::Comment));
    }

    #[test]
    fn aliases_resolve() {
        assert!(supported("ts"));
        assert!(supported("typescript"));
        assert!(supported("golang"));
        assert!(supported("shell"));
        assert!(!supported("cobol"));
    }

    #[test]
    fn multiline_block_comment_spans_lines() {
        let lines = highlight("code /* open\nstill comment\nclose */ done", "rust");
        // middle line is entirely comment
        assert!(lines[1].iter().all(|s| s.style.role == Role::Comment));
        // last line resumes normal code after the close
        assert!(lines[2]
            .iter()
            .any(|s| s.text.contains("done") && s.style.role == Role::Body));
    }

    #[test]
    fn line_text_roundtrips() {
        let src = "const foo = bar(42) + 'x'; // c";
        let joined: String = line0(src, "js").iter().map(|s| s.text.as_str()).collect();
        assert_eq!(joined, src);
    }
}

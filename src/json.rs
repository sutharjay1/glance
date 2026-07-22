//! JSON viewer port (Phase 5): `glance data.json` → syntax-colored, pretty-indented JSON.
//!
//! A pure `Value → Vec<Line>` renderer using glance's semantic [`Role`]s (keys, strings, numbers,
//! literals, punctuation each get a role, colored later by `paint`). 2-space indent per depth.
//! Non-JSON input falls back to showing the raw text with an error note.

use serde_json::Value;

use crate::style::{Line, Role, Span, Style};
use crate::text::width as text_width;

fn span(text: impl Into<String>, role: Role) -> Span {
    Span::new(text, Style::role(role))
}

fn dim(text: impl Into<String>) -> Span {
    span(text, Role::Dim)
}

fn indent_span(depth: usize) -> Span {
    dim("  ".repeat(depth))
}

/// Render a parsed JSON value to display lines at content width `width` (long lines truncated).
pub fn json_to_lines(value: &Value, width: usize) -> Vec<Line> {
    let mut out = Vec::new();
    render_value(value, 0, Vec::new(), false, &mut out);
    out.into_iter().map(|l| truncate_line(l, width)).collect()
}

/// Render `raw` JSON text to display lines. On a parse error, returns a dim error note followed by
/// the raw text so the file is still viewable.
pub fn render(raw: &str, width: usize) -> Vec<Line> {
    match serde_json::from_str::<Value>(raw) {
        Ok(v) => json_to_lines(&v, width),
        Err(e) => {
            let mut lines = vec![line(vec![span(
                format!("⚠ invalid JSON: {e}"),
                Role::Accent,
            )])];
            for l in raw.lines() {
                lines.push(truncate_line(
                    line(vec![span(l.to_string(), Role::Body)]),
                    width,
                ));
            }
            lines
        }
    }
}

fn line(spans: Vec<Span>) -> Line {
    Line {
        spans,
        no_wrap: true,
    }
}

/// Append the lines for `v`. `prefix` precedes the value on its opening line (a `"key": ` run or
/// nothing for array elements); `trailing` adds a comma after the value's last line.
fn render_value(v: &Value, depth: usize, prefix: Vec<Span>, trailing: bool, out: &mut Vec<Line>) {
    match v {
        Value::Object(map) if !map.is_empty() => {
            out.push(open_line(depth, prefix, "{"));
            let n = map.len();
            for (i, (k, val)) in map.iter().enumerate() {
                let key_prefix = vec![span(format!("\"{k}\""), Role::Heading), dim(": ")];
                render_value(val, depth + 1, key_prefix, i + 1 < n, out);
            }
            out.push(close_line(depth, "}", trailing));
        }
        Value::Array(arr) if !arr.is_empty() => {
            out.push(open_line(depth, prefix, "["));
            let n = arr.len();
            for (i, val) in arr.iter().enumerate() {
                render_value(val, depth + 1, Vec::new(), i + 1 < n, out);
            }
            out.push(close_line(depth, "]", trailing));
        }
        // Empty containers + scalars render on a single line.
        _ => {
            let mut spans = vec![indent_span(depth)];
            spans.extend(prefix);
            spans.push(scalar_span(v));
            if trailing {
                spans.push(dim(","));
            }
            out.push(line(spans));
        }
    }
}

fn open_line(depth: usize, prefix: Vec<Span>, bracket: &str) -> Line {
    let mut spans = vec![indent_span(depth)];
    spans.extend(prefix);
    spans.push(dim(bracket.to_string()));
    line(spans)
}

fn close_line(depth: usize, bracket: &str, trailing: bool) -> Line {
    let mut spans = vec![indent_span(depth), dim(bracket.to_string())];
    if trailing {
        spans.push(dim(","));
    }
    line(spans)
}

/// A span for a scalar (or empty container), colored by JSON type.
fn scalar_span(v: &Value) -> Span {
    match v {
        Value::String(_) => span(serde_json::to_string(v).unwrap_or_default(), Role::Str),
        Value::Number(_) => span(v.to_string(), Role::Number),
        Value::Bool(_) | Value::Null => span(v.to_string(), Role::Keyword),
        Value::Object(_) => dim("{}"), // empty object
        Value::Array(_) => dim("[]"),  // empty array
    }
}

/// Truncate a line's spans to `width` display columns, appending a dim `…` when clipped.
fn truncate_line(l: Line, width: usize) -> Line {
    let mut acc = 0usize;
    let mut spans: Vec<Span> = Vec::new();
    for sp in l.spans {
        let w = text_width(&sp.text);
        if acc + w <= width {
            acc += w;
            spans.push(sp);
            continue;
        }
        let budget = width.saturating_sub(acc).saturating_sub(1);
        let mut t = String::new();
        let mut wsum = 0usize;
        for ch in sp.text.chars() {
            let cw = text_width(&ch.to_string());
            if wsum + cw > budget {
                break;
            }
            wsum += cw;
            t.push(ch);
        }
        t.push('…');
        spans.push(Span::new(t, sp.style));
        break;
    }
    line(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roles_of(lines: &[Line], text: &str) -> Option<Role> {
        lines
            .iter()
            .flat_map(|l| &l.spans)
            .find(|s| s.text == text)
            .map(|s| s.style.role)
    }

    #[test]
    fn scalars_get_typed_roles() {
        let v: Value =
            serde_json::from_str(r#"{"name":"ada","age":36,"admin":true,"note":null}"#).unwrap();
        let lines = json_to_lines(&v, 80);
        assert_eq!(roles_of(&lines, "\"name\""), Some(Role::Heading)); // key
        assert_eq!(roles_of(&lines, "\"ada\""), Some(Role::Str)); // string value
        assert_eq!(roles_of(&lines, "36"), Some(Role::Number));
        assert_eq!(roles_of(&lines, "true"), Some(Role::Keyword));
        assert_eq!(roles_of(&lines, "null"), Some(Role::Keyword));
    }

    #[test]
    fn nesting_indents_by_depth() {
        let v: Value = serde_json::from_str(r#"{"a":{"b":1}}"#).unwrap();
        let lines = json_to_lines(&v, 80);
        // { / "a": { / "b": 1 / } / }
        assert_eq!(lines.len(), 5);
        // The inner key line starts with 4 spaces of indent (depth 2).
        assert!(lines[2].spans[0].text == "    ");
        assert!(lines[2].spans.iter().any(|s| s.text == "\"b\""));
    }

    #[test]
    fn arrays_expand_with_trailing_commas() {
        let v: Value = serde_json::from_str(r#"[1,2,3]"#).unwrap();
        let lines = json_to_lines(&v, 80);
        assert_eq!(lines.len(), 5); // [ 1, 2, 3 ]
                                    // First two elements have trailing commas, last does not.
        assert!(lines[1].spans.iter().any(|s| s.text == ","));
        assert!(!lines[3].spans.iter().any(|s| s.text == ","));
    }

    #[test]
    fn empty_containers_inline() {
        let v: Value = serde_json::from_str(r#"{"e":{},"a":[]}"#).unwrap();
        let lines = json_to_lines(&v, 80);
        assert!(lines.iter().any(|l| l.spans.iter().any(|s| s.text == "{}")));
        assert!(lines.iter().any(|l| l.spans.iter().any(|s| s.text == "[]")));
    }

    #[test]
    fn invalid_json_falls_back_to_raw_with_note() {
        let lines = render("{not valid", 80);
        assert!(lines[0]
            .spans
            .iter()
            .any(|s| s.text.contains("invalid JSON")));
        assert!(lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.text.contains("not valid"))));
    }

    #[test]
    fn long_line_is_truncated() {
        let v: Value = serde_json::from_str(r#"{"k":"aaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#).unwrap();
        let lines = json_to_lines(&v, 12);
        for l in &lines {
            let w: usize = l.spans.iter().map(|s| text_width(&s.text)).sum();
            assert!(w <= 12, "line width {w} exceeds 12");
        }
    }
}

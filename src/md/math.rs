//! Inline math (`$…$`) rendered as Unicode (Phase 5 port).
//!
//! A pragmatic LaTeX→Unicode transform: Greek letters, common operators/relations, and
//! super/subscripts map to their Unicode glyphs; anything unrecognized passes through readably.
//! This is not a TeX engine — it covers the inline math that shows up in READMEs and notes so it
//! reads naturally in a terminal. Pure and heavily unit-tested.

/// Replace every `$…$` (and `$$…$$`) span in `text` with its Unicode form, leaving other text and
/// escaped `\$` untouched. Used on inline text runs during layout.
pub fn apply_inline_math(text: &str) -> String {
    if !text.contains('$') {
        return text.to_string();
    }
    let bytes: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == '\\' && i + 1 < bytes.len() && bytes[i + 1] == '$' {
            out.push('$'); // escaped dollar → literal
            i += 2;
            continue;
        }
        if c == '$' {
            // `$$` display math or `$` inline math — find the matching closer.
            let display = i + 1 < bytes.len() && bytes[i + 1] == '$';
            let open = if display { i + 2 } else { i + 1 };
            let close_len = if display { 2 } else { 1 };
            if let Some(end) = find_close(&bytes, open, display) {
                let content: String = bytes[open..end].iter().collect();
                if is_mathy(&content) {
                    out.push_str(&math_to_unicode(&content));
                    i = end + close_len;
                    continue;
                }
                // Doesn't look like math (e.g. `$5` currency) → leave the whole span literal.
                for _ in 0..close_len {
                    out.push('$');
                }
                out.push_str(&content);
                for _ in 0..close_len {
                    out.push('$');
                }
                i = end + close_len;
                continue;
            }
            // Unbalanced `$` → emit literally and move on.
            out.push('$');
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Heuristic: a `$…$` span is treated as math only if it contains a LaTeX command (`\`), a
/// super/subscript (`^`/`_`), or a brace group — so plain `$5` / `$10` currency stays literal.
fn is_mathy(content: &str) -> bool {
    content.contains('\\')
        || content.contains('^')
        || content.contains('_')
        || content.contains('{')
}

/// Pre-parse pass over raw markdown: replace `$…$` math with Unicode **before** the CommonMark
/// parser runs, so subscript underscores (`x_i`) aren't mistaken for emphasis. Fenced code blocks
/// and inline `` `code` `` spans are left untouched.
pub fn preprocess_math(md: &str) -> String {
    if !md.contains('$') {
        return md.to_string();
    }
    let mut out = String::with_capacity(md.len());
    let mut in_fence = false;
    for line in md.split_inclusive('\n') {
        let trimmed = line.trim_end_matches('\n').trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            out.push_str(line);
        } else if in_fence {
            out.push_str(line);
        } else {
            out.push_str(&transform_line_math(line));
        }
    }
    out
}

/// Apply inline-math conversion to a line, copying inline `` `code` `` spans verbatim.
fn transform_line_math(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut seg = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '`' {
            out.push_str(&apply_inline_math(&seg));
            seg.clear();
            out.push('`');
            i += 1;
            while i < chars.len() && chars[i] != '`' {
                out.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                out.push('`');
                i += 1;
            }
        } else {
            seg.push(chars[i]);
            i += 1;
        }
    }
    out.push_str(&apply_inline_math(&seg));
    out
}

/// Find the closing `$` (or `$$`) for a span opened at `open`, or `None` if unbalanced.
fn find_close(chars: &[char], open: usize, display: bool) -> Option<usize> {
    let mut i = open;
    while i < chars.len() {
        if chars[i] == '$' {
            if display {
                if i + 1 < chars.len() && chars[i + 1] == '$' {
                    return Some(i);
                }
            } else {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Convert a LaTeX fragment (the content between `$`s) to Unicode: commands → symbols, `^`/`_` →
/// super/subscripts. Unknown commands pass through as their bare name.
pub fn math_to_unicode(latex: &str) -> String {
    let chars: Vec<char> = latex.chars().collect();
    let mut out = String::with_capacity(latex.len());
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' => {
                // Read a command name (ASCII letters); a non-letter after `\` is a literal escape.
                let start = i + 1;
                let mut j = start;
                while j < chars.len() && chars[j].is_ascii_alphabetic() {
                    j += 1;
                }
                if j == start {
                    // `\<sym>` — emit the following char literally (e.g. `\{`), or drop a lone `\`.
                    if start < chars.len() {
                        out.push(chars[start]);
                        i = start + 1;
                    } else {
                        i = start;
                    }
                    continue;
                }
                let name: String = chars[start..j].iter().collect();
                match symbol(&name) {
                    Some(sym) => out.push_str(sym),
                    None => out.push_str(&name), // unknown command → readable bare name
                }
                i = j;
            }
            '^' => {
                i += 1;
                let (grp, next) = read_group(&chars, i);
                out.push_str(&map_script(&math_to_unicode(&grp), superscript));
                i = next;
            }
            '_' => {
                i += 1;
                let (grp, next) = read_group(&chars, i);
                out.push_str(&map_script(&math_to_unicode(&grp), subscript));
                i = next;
            }
            '{' | '}' => i += 1, // grouping braces are not shown
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// Read the argument of a `^`/`_`: a `{…}` group, or the single next character.
fn read_group(chars: &[char], i: usize) -> (String, usize) {
    if i < chars.len() && chars[i] == '{' {
        let mut j = i + 1;
        let mut buf = String::new();
        while j < chars.len() && chars[j] != '}' {
            buf.push(chars[j]);
            j += 1;
        }
        let end = if j < chars.len() { j + 1 } else { j }; // skip the closing brace
        (buf, end)
    } else if i < chars.len() {
        (chars[i].to_string(), i + 1)
    } else {
        (String::new(), i)
    }
}

/// Map each char of a script group via `f`; if any char has no mapping, fall back to a caret/under
/// notation so nothing is silently lost.
fn map_script(group: &str, f: fn(char) -> Option<char>) -> String {
    let mapped: Option<String> = group.chars().map(f).collect();
    match mapped {
        Some(s) => s,
        None => group.to_string(), // at least one char unmappable → leave the group as-is
    }
}

fn superscript(c: char) -> Option<char> {
    Some(match c {
        '0' => '⁰',
        '1' => '¹',
        '2' => '²',
        '3' => '³',
        '4' => '⁴',
        '5' => '⁵',
        '6' => '⁶',
        '7' => '⁷',
        '8' => '⁸',
        '9' => '⁹',
        '+' => '⁺',
        '-' => '⁻',
        '=' => '⁼',
        '(' => '⁽',
        ')' => '⁾',
        'n' => 'ⁿ',
        'i' => 'ⁱ',
        _ => return None,
    })
}

fn subscript(c: char) -> Option<char> {
    Some(match c {
        '0' => '₀',
        '1' => '₁',
        '2' => '₂',
        '3' => '₃',
        '4' => '₄',
        '5' => '₅',
        '6' => '₆',
        '7' => '₇',
        '8' => '₈',
        '9' => '₉',
        '+' => '₊',
        '-' => '₋',
        '=' => '₌',
        '(' => '₍',
        ')' => '₎',
        'a' => 'ₐ',
        'e' => 'ₑ',
        'i' => 'ᵢ',
        'o' => 'ₒ',
        'x' => 'ₓ',
        _ => return None,
    })
}

/// LaTeX command name → Unicode symbol. Adapted from common math tables (see `vendor/NOTICE`).
fn symbol(name: &str) -> Option<&'static str> {
    Some(match name {
        // Lowercase Greek.
        "alpha" => "α",
        "beta" => "β",
        "gamma" => "γ",
        "delta" => "δ",
        "epsilon" => "ε",
        "zeta" => "ζ",
        "eta" => "η",
        "theta" => "θ",
        "iota" => "ι",
        "kappa" => "κ",
        "lambda" => "λ",
        "mu" => "μ",
        "nu" => "ν",
        "xi" => "ξ",
        "pi" => "π",
        "rho" => "ρ",
        "sigma" => "σ",
        "tau" => "τ",
        "upsilon" => "υ",
        "phi" => "φ",
        "chi" => "χ",
        "psi" => "ψ",
        "omega" => "ω",
        "varphi" => "ϕ",
        "varepsilon" => "ϵ",
        // Uppercase Greek.
        "Gamma" => "Γ",
        "Delta" => "Δ",
        "Theta" => "Θ",
        "Lambda" => "Λ",
        "Xi" => "Ξ",
        "Pi" => "Π",
        "Sigma" => "Σ",
        "Phi" => "Φ",
        "Psi" => "Ψ",
        "Omega" => "Ω",
        // Big operators.
        "sum" => "∑",
        "prod" => "∏",
        "int" => "∫",
        "iint" => "∬",
        "oint" => "∮",
        "partial" => "∂",
        "nabla" => "∇",
        "sqrt" => "√",
        // Relations.
        "leq" | "le" => "≤",
        "geq" | "ge" => "≥",
        "neq" | "ne" => "≠",
        "approx" => "≈",
        "equiv" => "≡",
        "sim" => "∼",
        "propto" => "∝",
        "cong" => "≅",
        "ll" => "≪",
        "gg" => "≫",
        // Arrows.
        "rightarrow" | "to" => "→",
        "leftarrow" | "gets" => "←",
        "leftrightarrow" => "↔",
        "Rightarrow" | "implies" => "⇒",
        "Leftarrow" => "⇐",
        "Leftrightarrow" | "iff" => "⇔",
        "mapsto" => "↦",
        "uparrow" => "↑",
        "downarrow" => "↓",
        // Binary operators.
        "times" => "×",
        "div" => "÷",
        "pm" => "±",
        "mp" => "∓",
        "cdot" => "·",
        "ast" => "∗",
        "star" => "⋆",
        "circ" => "∘",
        "bullet" => "•",
        "oplus" => "⊕",
        "otimes" => "⊗",
        // Set theory + logic.
        "in" => "∈",
        "notin" => "∉",
        "subset" => "⊂",
        "supset" => "⊃",
        "subseteq" => "⊆",
        "supseteq" => "⊇",
        "cup" => "∪",
        "cap" => "∩",
        "setminus" => "∖",
        "emptyset" => "∅",
        "forall" => "∀",
        "exists" => "∃",
        "nexists" => "∄",
        "neg" | "lnot" => "¬",
        "land" | "wedge" => "∧",
        "lor" | "vee" => "∨",
        // Misc.
        "infty" => "∞",
        "angle" => "∠",
        "deg" => "°",
        "prime" => "′",
        "ldots" | "dots" => "…",
        "cdots" => "⋯",
        "vdots" => "⋮",
        "ddots" => "⋱",
        "hbar" => "ℏ",
        "ell" => "ℓ",
        "Re" => "ℜ",
        "Im" => "ℑ",
        "aleph" => "ℵ",
        "perp" => "⊥",
        "parallel" => "∥",
        "therefore" => "∴",
        "because" => "∵",
        "quad" | "qquad" | "," | ";" | ":" | " " => " ",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greek_letters() {
        assert_eq!(math_to_unicode("\\alpha + \\beta"), "α + β");
        assert_eq!(math_to_unicode("\\Sigma \\omega"), "Σ ω");
    }

    #[test]
    fn operators_and_relations() {
        assert_eq!(math_to_unicode("\\sum \\int \\infty"), "∑ ∫ ∞");
        assert_eq!(
            math_to_unicode("a \\leq b \\neq c \\geq d"),
            "a ≤ b ≠ c ≥ d"
        );
        assert_eq!(
            math_to_unicode("x \\times y \\cdot z \\pm w"),
            "x × y · z ± w"
        );
        assert_eq!(
            math_to_unicode("a \\rightarrow b \\Rightarrow c"),
            "a → b ⇒ c"
        );
    }

    #[test]
    fn superscripts_and_subscripts() {
        assert_eq!(math_to_unicode("x^2"), "x²");
        assert_eq!(math_to_unicode("x^{2}"), "x²");
        assert_eq!(math_to_unicode("a_1 + a_2"), "a₁ + a₂");
        assert_eq!(math_to_unicode("x^{10}"), "x¹⁰");
        // A group with any unmappable char is left as-is (all-or-nothing), so `-x` stays literal.
        assert_eq!(math_to_unicode("e^{-x}"), "e-x");
        assert_eq!(math_to_unicode("x_i"), "xᵢ");
    }

    #[test]
    fn unknown_command_passes_through_as_name() {
        assert_eq!(math_to_unicode("\\foobar x"), "foobar x");
    }

    #[test]
    fn braces_are_grouping_only() {
        assert_eq!(math_to_unicode("{ab}"), "ab");
    }

    #[test]
    fn apply_inline_math_replaces_dollar_spans() {
        assert_eq!(
            apply_inline_math("mass is $E = mc^2$ per Einstein"),
            "mass is E = mc² per Einstein"
        );
        // Multiple spans.
        assert_eq!(apply_inline_math("$\\alpha$ and $\\beta$"), "α and β");
        // No math → unchanged.
        assert_eq!(apply_inline_math("no math here"), "no math here");
    }

    #[test]
    fn escaped_and_unbalanced_dollars_are_literal() {
        assert_eq!(apply_inline_math("costs \\$5 today"), "costs $5 today");
        assert_eq!(apply_inline_math("a lone $ sign"), "a lone $ sign");
    }

    #[test]
    fn display_math_does_not_crash() {
        assert_eq!(apply_inline_math("$$\\sum x_i$$"), "∑ xᵢ");
    }

    #[test]
    fn currency_is_left_literal() {
        // No LaTeX markers → not treated as math, so `$` stays.
        assert_eq!(
            apply_inline_math("costs $5 and $10 total"),
            "costs $5 and $10 total"
        );
    }

    #[test]
    fn preprocess_protects_code_and_transforms_prose() {
        // Prose math is converted; inline code and fenced code are untouched.
        let md = "text $\\alpha$ and `$x_i$`\n\n```\n$\\beta$\n```\n";
        let out = preprocess_math(md);
        assert!(out.contains("text α and `$x_i$`")); // prose converted, inline code preserved
        assert!(out.contains("$\\beta$")); // fenced code preserved
    }

    #[test]
    fn preprocess_subscripts_survive_as_non_markdown() {
        // The whole span becomes Unicode with no underscores left for the parser to see.
        let out = preprocess_math("value $x_1 + x_2$ here");
        assert_eq!(out, "value x₁ + x₂ here");
        assert!(!out.contains('_'));
    }
}

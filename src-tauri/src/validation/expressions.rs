//! Validation for the small SQL expression fragments that the table designer
//! accepts (CHECK constraints, calculated fields, DEFAULT expressions).
//!
//! DDL cannot be parameterized, so these fragments are embedded in SQL text.
//! The checks here guarantee a fragment cannot escape its surrounding
//! parentheses or append further statements; SQLite then rejects anything that
//! is not a valid expression when the DDL is executed.

use crate::errors::{AppError, AppResult};

use super::identifiers::quote_ident;

pub const MAX_EXPRESSION_LEN: usize = 2000;

#[derive(Debug, PartialEq)]
enum Token<'a> {
    /// '...' string literal, raw text including quotes.
    String(&'a str),
    /// "..." / `...` / [...] quoted identifier: (raw text, unescaped name).
    QuotedIdent(&'a str, String),
    Word(&'a str),
    Other(char),
}

fn tokenize(src: &str) -> AppResult<Vec<Token<'_>>> {
    let mut tokens = Vec::new();
    let chars: Vec<(usize, char)> = src.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let (start, c) = chars[i];
        match c {
            '\'' | '"' | '`' | '[' => {
                let close = if c == '[' { ']' } else { c };
                let mut j = i + 1;
                let mut name = String::new();
                loop {
                    if j >= chars.len() {
                        return Err(AppError::Validation(format!(
                            "unterminated quote in expression: {src}"
                        )));
                    }
                    let ch = chars[j].1;
                    if ch == close {
                        // Doubled quote is an escape, except for [...].
                        if close != ']' && j + 1 < chars.len() && chars[j + 1].1 == close {
                            name.push(ch);
                            j += 2;
                            continue;
                        }
                        break;
                    }
                    name.push(ch);
                    j += 1;
                }
                let end = chars[j].0 + chars[j].1.len_utf8();
                let raw = &src[start..end];
                tokens.push(if c == '\'' {
                    Token::String(raw)
                } else {
                    Token::QuotedIdent(raw, name)
                });
                i = j + 1;
            }
            c if c.is_alphanumeric() || c == '_' => {
                let mut j = i;
                while j < chars.len() && (chars[j].1.is_alphanumeric() || chars[j].1 == '_' || chars[j].1 == '$') {
                    j += 1;
                }
                let end = if j < chars.len() { chars[j].0 } else { src.len() };
                tokens.push(Token::Word(&src[start..end]));
                i = j;
            }
            c => {
                tokens.push(Token::Other(c));
                i += 1;
            }
        }
    }
    Ok(tokens)
}

/// Upper-cased bare words of a SQL text, ignoring literals and quoted names.
pub fn bare_words_upper(src: &str) -> AppResult<std::collections::HashSet<String>> {
    Ok(tokenize(src)?
        .into_iter()
        .filter_map(|t| match t {
            Token::Word(w) => Some(w.to_ascii_uppercase()),
            _ => None,
        })
        .collect())
}

/// Checks that `expr` is a single, self-contained expression fragment.
pub fn validate_expression(expr: &str) -> AppResult<()> {
    let bad = |why: &str| Err(AppError::Validation(format!("invalid expression {expr:?}: {why}")));
    if expr.trim().is_empty() {
        return bad("must not be empty");
    }
    if expr.len() > MAX_EXPRESSION_LEN {
        return bad("is too long");
    }
    if expr.chars().any(|c| c == '\0') {
        return bad("contains a NUL byte");
    }
    let tokens = tokenize(expr)?;
    let mut depth: i32 = 0;
    let mut prev: Option<char> = None;
    for token in &tokens {
        if let Token::Other(c) = token {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth < 0 {
                        return bad("unbalanced parentheses");
                    }
                }
                ';' => return bad("statement separators are not allowed"),
                '-' if prev == Some('-') => return bad("comments are not allowed"),
                '*' if prev == Some('/') => return bad("comments are not allowed"),
                _ => {}
            }
            prev = Some(*c);
        } else {
            prev = None;
        }
    }
    if depth != 0 {
        return bad("unbalanced parentheses");
    }
    if tokens.iter().any(|t| matches!(t, Token::Word(w) if w.eq_ignore_ascii_case("select"))) {
        return bad("subqueries are not allowed");
    }
    Ok(())
}

/// Rewrites references to column `from` as `to` in an expression. Function
/// names (words directly followed by `(`) and string literals are untouched.
pub fn rename_identifier(expr: &str, from: &str, to: &str) -> AppResult<String> {
    let tokens = tokenize(expr)?;
    let mut out = String::with_capacity(expr.len());
    for (idx, token) in tokens.iter().enumerate() {
        match token {
            Token::String(raw) => out.push_str(raw),
            Token::QuotedIdent(raw, name) => {
                if name.eq_ignore_ascii_case(from) {
                    out.push_str(&quote_ident(to));
                } else {
                    out.push_str(raw);
                }
            }
            Token::Word(w) => {
                let is_call = tokens[idx + 1..]
                    .iter()
                    .find(|t| !matches!(t, Token::Other(c) if c.is_whitespace()))
                    .is_some_and(|t| *t == Token::Other('('));
                if w.eq_ignore_ascii_case(from) && !is_call {
                    out.push_str(&quote_ident(to));
                } else {
                    out.push_str(w);
                }
            }
            Token::Other(c) => out.push(*c),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_expressions() {
        for ok in [
            "price > 0",
            "\"Unit Price\" * qty",
            "length(name) <= 10",
            "status IN ('a', 'b;c', 'x--y')",
            "(a + b) * (c - d)",
        ] {
            assert!(validate_expression(ok).is_ok(), "{ok}");
        }
    }

    #[test]
    fn rejects_escapes() {
        for bad in [
            "",
            "1); DROP TABLE x; --",
            "1) , evil TEXT, CHECK (1",
            "a -- comment",
            "a /* c */",
            "'unterminated",
            "(SELECT 1)",
            "a)",
        ] {
            assert!(validate_expression(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn renames_identifiers() {
        assert_eq!(
            rename_identifier("qty * price + length(qty) + 'qty'", "qty", "amount").unwrap(),
            "\"amount\" * price + length(\"amount\") + 'qty'"
        );
        assert_eq!(
            rename_identifier("\"Qty\" > 0", "qty", "Units").unwrap(),
            "\"Units\" > 0"
        );
        assert_eq!(rename_identifier("length (x)", "length", "y").unwrap(), "length (x)");
    }
}

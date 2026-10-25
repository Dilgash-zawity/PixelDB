//! Checks on SQL typed for an online database. Statements also run inside a
//! read-only (or rolled-back) transaction; this check just refuses obvious
//! writes and several statements up front with a clear message.

use crate::errors::{AppError, AppResult};

const READ_KEYWORDS: &[&str] = &["SELECT", "WITH", "VALUES", "TABLE", "SHOW", "EXPLAIN", "DESCRIBE", "DESC"];

/// The single read-only statement in `sql`, without a trailing `;`.
pub fn read_only_statement(sql: &str) -> AppResult<&str> {
    let body = single_statement(sql)?;
    let start = skip_trivia(body);
    let word: String = body[start..]
        .chars()
        .skip_while(|c| *c == '(')
        .take_while(|c| c.is_ascii_alphabetic())
        .collect::<String>()
        .to_ascii_uppercase();
    if word.is_empty() {
        return Err(AppError::Validation("enter a query".into()));
    }
    if !READ_KEYWORDS.contains(&word.as_str()) {
        return Err(AppError::Unsupported(format!(
            "only queries that read data can run on an online database (this one starts with {word})"
        )));
    }
    Ok(body)
}

/// Offset of the first character that is not whitespace or a comment.
fn skip_trivia(s: &str) -> usize {
    let b = s.as_bytes();
    let mut i = 0;
    loop {
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if b[i..].starts_with(b"--") {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if b[i..].starts_with(b"/*") {
            i = s[i + 2..].find("*/").map_or(b.len(), |e| i + 2 + e + 2);
        } else {
            return i;
        }
    }
}

/// `sql` up to its only statement's end. Quotes, comments and Postgres
/// dollar quotes are skipped; a `;` followed by more than trivia is refused.
fn single_statement(sql: &str) -> AppResult<&str> {
    let b = sql.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            q @ (b'\'' | b'"' | b'`') => {
                i += 1;
                while i < b.len() {
                    if b[i] == q {
                        // A doubled quote is an escaped one.
                        if b.get(i + 1) == Some(&q) {
                            i += 2;
                            continue;
                        }
                        break;
                    }
                    if b[i] == b'\\' && q != b'"' {
                        i += 1;
                    }
                    i += 1;
                }
                i += 1;
            }
            b'[' => i = sql[i..].find(']').map_or(b.len(), |e| i + e + 1),
            b'-' if b.get(i + 1) == Some(&b'-') => i = sql[i..].find('\n').map_or(b.len(), |e| i + e),
            b'/' if b.get(i + 1) == Some(&b'*') => i = sql[i + 2..].find("*/").map_or(b.len(), |e| i + 2 + e + 2),
            b'$' => {
                // $tag$ … $tag$
                let tag_end = sql[i + 1..]
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .map(|e| i + 1 + e);
                match tag_end {
                    Some(e) if b[e] == b'$' && !b[i + 1..e].first().is_some_and(u8::is_ascii_digit) => {
                        let tag = &sql[i..=e];
                        i = sql[e + 1..].find(tag).map_or(b.len(), |x| e + 1 + x + tag.len());
                    }
                    _ => i += 1,
                }
            }
            b';' => {
                let rest = &sql[i + 1..];
                if skip_trivia(rest) < rest.len() {
                    return Err(AppError::Unsupported("run one statement at a time".into()));
                }
                return Ok(sql[..i].trim());
            }
            _ => i += 1,
        }
    }
    Ok(sql.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_reads() {
        assert_eq!(read_only_statement("select 1;").unwrap(), "select 1");
        assert_eq!(read_only_statement("  -- note\n/* x */ WITH a AS (SELECT 1) SELECT * FROM a; -- end").unwrap(), "-- note\n/* x */ WITH a AS (SELECT 1) SELECT * FROM a");
        assert!(read_only_statement("(SELECT 1) UNION (SELECT 2)").is_ok());
        assert!(read_only_statement("SELECT ';' AS a, \"b;\" FROM [x;y]").is_ok());
        assert!(read_only_statement("SELECT $$a;b$$, $t$;$t$").is_ok());
        assert!(read_only_statement("SELECT 'it''s; fine'").is_ok());
        assert!(read_only_statement("SELECT $1").is_ok());
    }

    #[test]
    fn refuses_writes_and_batches() {
        assert!(matches!(read_only_statement("DELETE FROM t"), Err(AppError::Unsupported(_))));
        assert!(matches!(read_only_statement("SELECT 1; DROP TABLE t"), Err(AppError::Unsupported(_))));
        assert!(matches!(read_only_statement("  -- only a comment"), Err(AppError::Validation(_))));
        assert!(read_only_statement("PRAGMA writable_schema = 1").is_err());
    }
}

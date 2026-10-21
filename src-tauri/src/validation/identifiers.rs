use crate::errors::{AppError, AppResult};

pub const MAX_IDENTIFIER_LEN: usize = 128;
/// Prefix of tables owned by this application.
pub const APP_PREFIX: &str = "__app_";

/// Validates a user-supplied table, column or index name.
///
/// Names may contain spaces and punctuation (as in Access) because they are
/// always quoted, but control characters, surrounding whitespace and names
/// reserved by SQLite or this application are rejected.
pub fn validate_identifier(name: &str) -> AppResult<()> {
    let invalid = |why: &str| Err(AppError::InvalidIdentifier(format!("{name:?}: {why}")));
    if name.is_empty() {
        return invalid("must not be empty");
    }
    if name.chars().count() > MAX_IDENTIFIER_LEN {
        return invalid("is too long");
    }
    if name.trim() != name {
        return invalid("must not start or end with whitespace");
    }
    if name.chars().any(|c| c.is_control()) {
        return invalid("must not contain control characters");
    }
    let lower = name.to_ascii_lowercase();
    if lower.starts_with("sqlite_") {
        return invalid("names starting with sqlite_ are reserved by SQLite");
    }
    if lower.starts_with(APP_PREFIX) {
        return invalid("names starting with __app_ are reserved by the application");
    }
    Ok(())
}

pub fn is_reserved_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.starts_with("sqlite_") || lower.starts_with(APP_PREFIX)
}

/// Quotes an identifier for SQL. The input must already be validated or
/// originate from the database schema itself.
pub fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Quotes a string literal for SQL text where parameters cannot be used
/// (DDL such as DEFAULT clauses).
pub fn quote_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_access_style_names() {
        assert!(validate_identifier("Order Details").is_ok());
        assert!(validate_identifier("Café").is_ok());
        assert!(validate_identifier("a\"b").is_ok());
    }

    #[test]
    fn rejects_bad_names() {
        for bad in ["", " x", "x ", "a\0b", "a\nb", "sqlite_master", "__app_tables"] {
            assert!(validate_identifier(bad).is_err(), "{bad:?}");
        }
        assert!(validate_identifier(&"x".repeat(129)).is_err());
    }

    #[test]
    fn quoting_escapes() {
        assert_eq!(quote_ident("a\"b"), "\"a\"\"b\"");
        assert_eq!(quote_literal("it's"), "'it''s'");
    }
}

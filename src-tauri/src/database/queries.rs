//! Ad-hoc queries typed or built by the user, and saving them as views.
//!
//! Queries are read-only: a single SELECT, WITH or VALUES statement that
//! SQLite itself reports as not writing. Saved queries are ordinary views,
//! so they also open in the Datasheet. A query built in the visual designer
//! also keeps its design (in `__app_queries`) so it reopens in the designer.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use rusqlite::{Connection, ErrorCode};
use serde_json::Value as JsonValue;

use crate::errors::{AppError, AppResult};
use crate::models::{QueryResult, SavedQuery};
use crate::validation::identifiers::{quote_ident, validate_identifier};
use crate::validation::values::sql_to_json;

use super::metadata;
use super::schema::{find_object, require_object, result_types};
use super::transactions::with_transaction;

pub const MAX_QUERY_ROWS: usize = 1000;
pub const QUERY_TIMEOUT: Duration = Duration::from_secs(30);

/// Skips leading whitespace and comments, returning the rest of the text.
fn skip_trivia(mut s: &str) -> &str {
    loop {
        s = s.trim_start();
        if let Some(rest) = s.strip_prefix("--") {
            s = rest.find('\n').map_or("", |i| &rest[i + 1..]);
        } else if let Some(rest) = s.strip_prefix("/*") {
            s = rest.find("*/").map_or("", |i| &rest[i + 2..]);
        } else {
            return s;
        }
    }
}

/// The query text without leading trivia or trailing semicolons and whitespace.
fn normalize(sql: &str) -> AppResult<&str> {
    let body = skip_trivia(sql).trim_end().trim_end_matches(';').trim_end();
    if body.is_empty() {
        return Err(AppError::Validation("the query is empty".into()));
    }
    let word: String = body.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    if !["SELECT", "WITH", "VALUES"].contains(&word.to_ascii_uppercase().as_str()) {
        return Err(AppError::Unsupported(
            "only queries that read data (SELECT) can be run here".into(),
        ));
    }
    Ok(body)
}

fn prepare_error(e: rusqlite::Error) -> AppError {
    match e {
        rusqlite::Error::MultipleStatement => {
            AppError::Validation("run one statement at a time".into())
        }
        e => e.into(),
    }
}

fn read_error(e: rusqlite::Error) -> AppError {
    match &e {
        rusqlite::Error::SqliteFailure(f, _) if f.code == ErrorCode::OperationInterrupted => {
            AppError::Validation(format!(
                "the query took longer than {} seconds and was stopped",
                QUERY_TIMEOUT.as_secs()
            ))
        }
        _ => e.into(),
    }
}

/// Runs one read-only statement, returning at most `MAX_QUERY_ROWS` rows.
pub fn run_query(conn: &Connection, sql: &str) -> AppResult<QueryResult> {
    let body = normalize(sql)?;
    let started = Instant::now();
    let mut stmt = conn.prepare(body).map_err(prepare_error)?;
    if !stmt.readonly() {
        return Err(AppError::Unsupported("only queries that read data can be run here".into()));
    }
    let columns: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();
    let types = result_types(conn, &stmt);

    // Stops runaway queries (e.g. an unbounded recursive CTE) so the
    // database is not locked indefinitely.
    let handle = conn.get_interrupt_handle();
    let (done, wait) = mpsc::channel::<()>();
    let watchdog = std::thread::spawn(move || {
        if let Err(mpsc::RecvTimeoutError::Timeout) = wait.recv_timeout(QUERY_TIMEOUT) {
            handle.interrupt();
        }
    });
    let read = (|| -> rusqlite::Result<(Vec<Vec<serde_json::Value>>, bool)> {
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            if out.len() == MAX_QUERY_ROWS {
                return Ok((out, true));
            }
            out.push((0..columns.len()).map(|i| row.get_ref(i).map(sql_to_json)).collect::<Result<_, _>>()?);
        }
        Ok((out, false))
    })();
    let _ = done.send(());
    let _ = watchdog.join();
    let (rows, truncated) = read.map_err(read_error)?;
    Ok(QueryResult { columns, rows, truncated, elapsed_ms: started.elapsed().as_millis() as u64, types })
}

/// Saves a query as a view, with the visual `design` it came from if any.
/// An existing view is only replaced with `replace`.
pub fn save_view(
    conn: &Connection,
    name: &str,
    sql: &str,
    design: Option<&JsonValue>,
    replace: bool,
) -> AppResult<()> {
    validate_identifier(name)?;
    let body = normalize(sql)?;
    // Checks the query on its own first, so a second statement can't hide
    // behind the CREATE VIEW prefix.
    let stmt = conn.prepare(body).map_err(prepare_error)?;
    if !stmt.readonly() {
        return Err(AppError::Unsupported("only queries that read data can be saved".into()));
    }
    drop(stmt);
    let existing = find_object(conn, name)?;
    with_transaction(conn, |tx| {
        match existing {
            Some(o) if o.kind == "view" && replace => {
                tx.execute_batch(&format!("DROP VIEW {}", quote_ident(&o.name)))?
            }
            Some(o) if o.kind == "view" => {
                return Err(AppError::ConfirmationRequired(format!(
                    "a query named {:?} already exists; replacing it changes what it shows",
                    o.name
                )));
            }
            Some(o) => return Err(AppError::AlreadyExists(format!("{:?} is already used by a table", o.name))),
            None => {}
        }
        tx.execute(&format!("CREATE VIEW {} AS {body}", quote_ident(name)), [])
            .map_err(prepare_error)?;
        // Views compile lazily; make sure this one actually works.
        tx.prepare(&format!("SELECT * FROM {} LIMIT 0", quote_ident(name)))?;
        metadata::save_query_design(tx, name, design, body)
    })
}

pub fn drop_view(conn: &Connection, name: &str, confirm: bool) -> AppResult<()> {
    let o = require_object(conn, name)?;
    if o.kind != "view" {
        return Err(AppError::Unsupported(format!("{:?} is a table, not a query", o.name)));
    }
    if !confirm {
        return Err(AppError::ConfirmationRequired(format!("deleting query {:?}", o.name)));
    }
    with_transaction(conn, |tx| {
        tx.execute_batch(&format!("DROP VIEW {}", quote_ident(&o.name)))?;
        metadata::delete_query_design(tx, &o.name)
    })
}

/// The SELECT a view runs: its CREATE VIEW text after the top-level `AS`.
pub fn view_sql(conn: &Connection, name: &str) -> AppResult<String> {
    let o = require_object(conn, name)?;
    let sql = match (o.kind.as_str(), o.sql) {
        ("view", Some(sql)) => sql,
        _ => return Err(AppError::Unsupported(format!("{:?} is not a query", o.name))),
    };
    select_of_view(&sql)
        .map(str::to_string)
        .ok_or_else(|| AppError::Unsupported(format!("couldn't read the query behind {:?}", o.name)))
}

/// A saved query's SQL and, while that SQL is still what it was saved with,
/// its visual design. A view changed outside the designer (or rewritten by a
/// table rename) opens as plain SQL.
pub fn saved_query(conn: &Connection, name: &str) -> AppResult<SavedQuery> {
    let sql = view_sql(conn, name)?;
    let design = metadata::load_query_design(conn, name)?
        .and_then(|(design, saved_sql)| (saved_sql == sql).then_some(design));
    Ok(SavedQuery { sql, design })
}

/// Finds the first `AS` keyword outside quotes and parentheses.
fn select_of_view(create: &str) -> Option<&str> {
    let bytes = create.as_bytes();
    let (mut i, mut depth) = (0, 0usize);
    while i < bytes.len() {
        match bytes[i] {
            q @ (b'"' | b'\'' | b'`') => {
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == q {
                        if bytes.get(i + 1) == Some(&q) {
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    i += 1;
                }
            }
            b'[' => i += create[i..].find(']')?,
            b'(' => depth += 1,
            b')' => depth = depth.saturating_sub(1),
            b'a' | b'A'
                if depth == 0
                    && bytes.get(i + 1).is_some_and(|c| c.eq_ignore_ascii_case(&b's'))
                    && i > 0
                    && bytes[i - 1].is_ascii_whitespace()
                    && bytes.get(i + 2).is_none_or(|c| c.is_ascii_whitespace()) =>
            {
                return Some(create[i + 2..].trim());
            }
            _ => {}
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_select_of_a_view() {
        assert_eq!(select_of_view(r#"CREATE VIEW "Big as life" AS SELECT 1"#), Some("SELECT 1"));
        assert_eq!(select_of_view("CREATE VIEW v(a, b) AS\n SELECT 1, 2"), Some("SELECT 1, 2"));
        assert_eq!(select_of_view("create view [x as y] as select 2"), Some("select 2"));
        assert_eq!(select_of_view("CREATE VIEW v"), None);
    }

    #[test]
    fn only_reading_statements_are_accepted() {
        assert_eq!(normalize("  -- hi\n select 1;  ").unwrap(), "select 1");
        assert!(normalize("/* x */ WITH a AS (SELECT 1) SELECT * FROM a").is_ok());
        assert!(matches!(normalize("DELETE FROM t"), Err(AppError::Unsupported(_))));
        assert!(matches!(normalize("ATTACH 'x.db' AS x"), Err(AppError::Unsupported(_))));
        assert!(matches!(normalize(" ; "), Err(AppError::Validation(_))));
    }
}

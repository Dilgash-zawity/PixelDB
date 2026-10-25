//! Online databases: PostgreSQL (also Supabase and Neon), MySQL, SQL Server
//! and libSQL (a sqld server or Turso). Each call opens its own connection
//! and closes it again; everything runs read-only.

mod libsql;
mod mssql;
mod mysql;
mod postgres;
mod sqlx_rows;
pub mod sql;

use std::future::Future;
use std::time::{Duration, Instant};

use serde_json::Value as JsonValue;

use crate::errors::{AppError, AppResult};
use crate::models::{Connection, Engine, RemoteResult, RemoteTable, ServerInfo};

/// Rows a query returns at most (matches local queries).
pub const MAX_ROWS: usize = 1000;
/// Rows shown when browsing a table.
pub const PREVIEW_ROWS: usize = 200;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const QUERY_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn test(c: &Connection) -> AppResult<ServerInfo> {
    let version = within(CONNECT_TIMEOUT + QUERY_TIMEOUT, async {
        match c.provider.engine() {
            Engine::Postgres => postgres::version(c).await,
            Engine::Mysql => mysql::version(c).await,
            Engine::SqlServer => mssql::version(c).await,
            Engine::Libsql => libsql::version(c).await,
        }
    })
    .await?;
    Ok(ServerInfo { version })
}

pub async fn list_tables(c: &Connection) -> AppResult<Vec<RemoteTable>> {
    within(CONNECT_TIMEOUT + QUERY_TIMEOUT, async {
        match c.provider.engine() {
            Engine::Postgres => postgres::tables(c).await,
            Engine::Mysql => mysql::tables(c).await,
            Engine::SqlServer => mssql::tables(c).await,
            Engine::Libsql => libsql::tables(c).await,
        }
    })
    .await
}

/// Runs one read-only statement typed by the user.
pub async fn query(c: &Connection, sql: &str) -> AppResult<RemoteResult> {
    let sql = sql::read_only_statement(sql)?;
    run(c, sql, MAX_ROWS).await
}

/// The first rows of a table or view.
pub async fn preview(c: &Connection, schema: Option<&str>, table: &str) -> AppResult<RemoteResult> {
    let engine = c.provider.engine();
    let name = match schema {
        Some(s) => format!("{}.{}", quote(engine, s), quote(engine, table)),
        None => quote(engine, table),
    };
    let sql = match engine {
        Engine::SqlServer => format!("SELECT TOP ({}) * FROM {name}", PREVIEW_ROWS + 1),
        _ => format!("SELECT * FROM {name} LIMIT {}", PREVIEW_ROWS + 1),
    };
    run(c, &sql, PREVIEW_ROWS).await
}

async fn run(c: &Connection, sql: &str, limit: usize) -> AppResult<RemoteResult> {
    let started = Instant::now();
    let mut result = within(CONNECT_TIMEOUT + QUERY_TIMEOUT, async {
        match c.provider.engine() {
            Engine::Postgres => postgres::run(c, sql, limit).await,
            Engine::Mysql => mysql::run(c, sql, limit).await,
            Engine::SqlServer => mssql::run(c, sql, limit).await,
            Engine::Libsql => libsql::run(c, sql, limit).await,
        }
    })
    .await?;
    result.elapsed_ms = started.elapsed().as_millis() as u64;
    Ok(result)
}

async fn within<T>(limit: Duration, work: impl Future<Output = AppResult<T>>) -> AppResult<T> {
    tokio::time::timeout(limit, work)
        .await
        .map_err(|_| AppError::Remote(format!("the server did not answer within {} seconds", limit.as_secs())))?
}

/// Quotes an identifier for `engine`.
pub fn quote(engine: Engine, name: &str) -> String {
    match engine {
        Engine::Mysql => format!("`{}`", name.replace('`', "``")),
        Engine::SqlServer => format!("[{}]", name.replace(']', "]]")),
        Engine::Postgres | Engine::Libsql => format!("\"{}\"", name.replace('"', "\"\"")),
    }
}

/// Largest integer JavaScript numbers hold exactly; bigger ones stay text.
const SAFE_INTEGER: i64 = (1 << 53) - 1;

/// A value the server sent as text, typed by its column type name.
fn text_value(type_name: &str, text: &str) -> JsonValue {
    let t = type_name.to_ascii_uppercase();
    let t = t.trim_end_matches(" UNSIGNED");
    match t {
        "INT2" | "INT4" | "INT8" | "OID" | "TINYINT" | "SMALLINT" | "MEDIUMINT" | "INT" | "INTEGER" | "BIGINT" | "YEAR" => text
            .parse::<i64>()
            .ok()
            .filter(|n| n.abs() <= SAFE_INTEGER)
            .map_or_else(|| JsonValue::String(text.into()), JsonValue::from),
        "FLOAT4" | "FLOAT8" | "FLOAT" | "DOUBLE" | "REAL" => text
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map_or_else(|| JsonValue::String(text.into()), JsonValue::Number),
        "BOOL" | "BOOLEAN" => match text {
            "t" | "true" | "1" => JsonValue::Bool(true),
            "f" | "false" | "0" => JsonValue::Bool(false),
            _ => JsonValue::String(text.into()),
        },
        _ => JsonValue::String(text.into()),
    }
}

/// Shown in place of binary data.
fn binary_label(len: usize) -> JsonValue {
    JsonValue::String(format!("<binary, {len} bytes>"))
}

/// Builds the table list from the rows of a catalog query: `(schema?, name, type)`.
fn tables_from(result: RemoteResult, with_schema: bool) -> Vec<RemoteTable> {
    let text = |v: &JsonValue| match v {
        JsonValue::String(s) => s.clone(),
        other => other.to_string(),
    };
    result
        .rows
        .into_iter()
        .filter_map(|row| {
            let (schema, rest) = if with_schema { (row.first().map(text), row.get(1..)?) } else { (None, &row[..]) };
            let name = text(rest.first()?);
            let kind = if rest.get(1).map(text).is_some_and(|k| k.to_ascii_uppercase().contains("VIEW")) {
                crate::models::ObjectKind::View
            } else {
                crate::models::ObjectKind::Table
            };
            Some(RemoteTable { schema, name, kind })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn typed_text_values() {
        assert_eq!(text_value("INT4", "42"), json!(42));
        assert_eq!(text_value("BIGINT UNSIGNED", "18446744073709551615"), json!("18446744073709551615"));
        assert_eq!(text_value("FLOAT8", "1.5"), json!(1.5));
        assert_eq!(text_value("FLOAT8", "NaN"), json!("NaN"));
        assert_eq!(text_value("bool", "t"), json!(true));
        assert_eq!(text_value("NUMERIC", "12.50"), json!("12.50"));
        assert_eq!(text_value("INTERVAL", "1 day"), json!("1 day"));
    }

    #[test]
    fn quoting() {
        assert_eq!(quote(Engine::Postgres, "a\"b"), "\"a\"\"b\"");
        assert_eq!(quote(Engine::Mysql, "a`b"), "`a``b`");
        assert_eq!(quote(Engine::SqlServer, "a]b"), "[a]]b]");
    }

    #[test]
    fn catalog_rows() {
        let r = RemoteResult {
            rows: vec![vec![json!("public"), json!("orders"), json!("BASE TABLE")], vec![json!("public"), json!("totals"), json!("VIEW")]],
            ..Default::default()
        };
        let t = tables_from(r, true);
        assert_eq!(t[0].schema.as_deref(), Some("public"));
        assert_eq!(t[1].kind, crate::models::ObjectKind::View);
    }
}

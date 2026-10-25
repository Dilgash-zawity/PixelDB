use sqlx::Connection as _;
use sqlx::postgres::{PgConnectOptions, PgConnection, PgSslMode};

use super::sqlx_rows::{collect, remote_error};
use super::{CONNECT_TIMEOUT, tables_from, within};
use crate::errors::AppResult;
use crate::models::{Connection, RemoteResult, RemoteTable, TlsMode};

async fn connect(c: &Connection) -> AppResult<PgConnection> {
    let options = PgConnectOptions::new()
        .host(&c.host)
        .port(c.port.unwrap_or(5432))
        .username(&c.user)
        .password(&c.password)
        .database(&c.database)
        .application_name("SlateDB")
        .ssl_mode(match c.tls {
            TlsMode::Disable => PgSslMode::Disable,
            TlsMode::Prefer => PgSslMode::Prefer,
            TlsMode::Require => PgSslMode::Require,
        });
    within(CONNECT_TIMEOUT, async { PgConnection::connect_with(&options).await.map_err(remote_error) }).await
}

/// Runs `sql` in a read-only transaction that is rolled back afterwards.
/// Statements are sent unprepared, which also works through PgBouncer
/// (Supabase's transaction pooler).
pub async fn run(c: &Connection, sql: &str, limit: usize) -> AppResult<RemoteResult> {
    let mut conn = connect(c).await?;
    sqlx::raw_sql("BEGIN READ ONLY").execute(&mut conn).await.map_err(remote_error)?;
    let result = collect::<sqlx::Postgres, _>(sqlx::raw_sql(sql).fetch(&mut conn), limit).await;
    let _ = sqlx::raw_sql("ROLLBACK").execute(&mut conn).await;
    let _ = conn.close().await;
    result
}

pub async fn version(c: &Connection) -> AppResult<String> {
    let r = run(c, "SHOW server_version", 1).await?;
    let v = r.rows.first().and_then(|row| row.first()).and_then(|v| v.as_str()).unwrap_or("");
    Ok(format!("PostgreSQL {v}").trim().to_string())
}

pub async fn tables(c: &Connection) -> AppResult<Vec<RemoteTable>> {
    let r = run(
        c,
        "SELECT table_schema, table_name, table_type FROM information_schema.tables \
         WHERE table_schema NOT IN ('pg_catalog', 'information_schema') AND table_schema NOT LIKE 'pg\\_%' \
         ORDER BY table_schema <> 'public', table_schema, table_name",
        100_000,
    )
    .await?;
    Ok(tables_from(r, true))
}

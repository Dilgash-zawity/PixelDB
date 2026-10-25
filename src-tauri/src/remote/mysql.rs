use sqlx::Connection as _;
use sqlx::mysql::{MySqlConnectOptions, MySqlConnection, MySqlSslMode};

use super::sqlx_rows::{collect, remote_error};
use super::{CONNECT_TIMEOUT, tables_from, within};
use crate::errors::AppResult;
use crate::models::{Connection, RemoteResult, RemoteTable, TlsMode};

async fn connect(c: &Connection) -> AppResult<MySqlConnection> {
    let mut options = MySqlConnectOptions::new()
        .host(&c.host)
        .port(c.port.unwrap_or(3306))
        .username(&c.user)
        .password(&c.password)
        .ssl_mode(match c.tls {
            TlsMode::Disable => MySqlSslMode::Disabled,
            TlsMode::Prefer => MySqlSslMode::Preferred,
            TlsMode::Require => MySqlSslMode::Required,
        });
    if !c.database.is_empty() {
        options = options.database(&c.database);
    }
    within(CONNECT_TIMEOUT, async { MySqlConnection::connect_with(&options).await.map_err(remote_error) }).await
}

/// Runs `sql` in a read-only transaction that is rolled back afterwards.
pub async fn run(c: &Connection, sql: &str, limit: usize) -> AppResult<RemoteResult> {
    let mut conn = connect(c).await?;
    sqlx::raw_sql("START TRANSACTION READ ONLY").execute(&mut conn).await.map_err(remote_error)?;
    let result = collect::<sqlx::MySql, _>(sqlx::raw_sql(sql).fetch(&mut conn), limit).await;
    let _ = sqlx::raw_sql("ROLLBACK").execute(&mut conn).await;
    let _ = conn.close().await;
    result
}

pub async fn version(c: &Connection) -> AppResult<String> {
    let r = run(c, "SELECT VERSION()", 1).await?;
    let v = r.rows.first().and_then(|row| row.first()).and_then(|v| v.as_str()).unwrap_or("");
    Ok(if v.to_ascii_lowercase().contains("mariadb") { format!("MariaDB {v}") } else { format!("MySQL {v}") })
}

/// Tables of the chosen database, or of every user database when none was named.
pub async fn tables(c: &Connection) -> AppResult<Vec<RemoteTable>> {
    if c.database.is_empty() {
        let r = run(
            c,
            "SELECT table_schema, table_name, table_type FROM information_schema.tables \
             WHERE table_schema NOT IN ('mysql', 'information_schema', 'performance_schema', 'sys') \
             ORDER BY table_schema, table_name",
            100_000,
        )
        .await?;
        return Ok(tables_from(r, true));
    }
    let r = run(
        c,
        "SELECT table_name, table_type FROM information_schema.tables WHERE table_schema = DATABASE() ORDER BY table_name",
        100_000,
    )
    .await?;
    Ok(tables_from(r, false))
}

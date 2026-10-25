use futures_util::TryStreamExt;
use serde_json::Value as JsonValue;
use tiberius::time::chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};
use tiberius::{AuthMethod, Client, ColumnData, Config, EncryptionLevel, FromSql};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use super::{CONNECT_TIMEOUT, SAFE_INTEGER, binary_label, tables_from, within};
use crate::errors::{AppError, AppResult};
use crate::models::{Connection, RemoteResult, RemoteTable, TlsMode};

type MsClient = Client<Compat<TcpStream>>;

fn remote_error(e: tiberius::error::Error) -> AppError {
    match e {
        tiberius::error::Error::Server(t) => AppError::Remote(t.message().to_string()),
        tiberius::error::Error::Io { message, .. } => AppError::Remote(format!("couldn't reach the server: {message}")),
        tiberius::error::Error::Tls(m) => AppError::Remote(format!("secure connection failed: {m}")),
        other => AppError::Remote(other.to_string()),
    }
}

fn config(c: &Connection, host: &str, port: u16) -> Config {
    let mut config = Config::new();
    config.host(host);
    config.port(port);
    config.authentication(AuthMethod::sql_server(&c.user, &c.password));
    config.application_name("SlateDB");
    if !c.database.is_empty() {
        config.database(&c.database);
    }
    config.encryption(match c.tls {
        TlsMode::Disable => EncryptionLevel::Off,
        TlsMode::Prefer => EncryptionLevel::On,
        TlsMode::Require => EncryptionLevel::Required,
    });
    if c.trust_certificate {
        config.trust_cert();
    }
    config
}

async fn open(config: Config) -> Result<MsClient, tiberius::error::Error> {
    let tcp = TcpStream::connect(config.get_addr()).await?;
    tcp.set_nodelay(true)?;
    Client::connect(config, tcp.compat_write()).await
}

async fn connect(c: &Connection) -> AppResult<MsClient> {
    within(CONNECT_TIMEOUT, async {
        match open(config(c, &c.host, c.port.unwrap_or(1433))).await {
            // Azure SQL sends the client on to the server that holds the database.
            Err(tiberius::error::Error::Routing { host, port }) => open(config(c, &host, port)).await.map_err(remote_error),
            other => other.map_err(remote_error),
        }
    })
    .await
}

/// Runs `sql` inside a transaction that is always rolled back.
pub async fn run(c: &Connection, sql: &str, limit: usize) -> AppResult<RemoteResult> {
    let mut client = connect(c).await?;
    client.simple_query("BEGIN TRANSACTION").await.map_err(remote_error)?.into_results().await.map_err(remote_error)?;
    let result = collect(&mut client, sql, limit).await;
    if let Ok(s) = client.simple_query("IF @@TRANCOUNT > 0 ROLLBACK").await {
        let _ = s.into_results().await;
    }
    let _ = client.close().await;
    result
}

async fn collect(client: &mut MsClient, sql: &str, limit: usize) -> AppResult<RemoteResult> {
    let mut stream = client.simple_query(sql).await.map_err(remote_error)?;
    let mut out = RemoteResult::default();
    if let Some(columns) = stream.columns().await.map_err(remote_error)? {
        out.columns = columns.iter().map(|c| c.name().to_string()).collect();
    }
    let mut rows = stream.into_row_stream();
    while let Some(row) = rows.try_next().await.map_err(remote_error)? {
        if out.rows.len() == limit {
            out.truncated = true;
            break;
        }
        out.rows.push(row.into_iter().map(|d| value(&d)).collect());
    }
    Ok(out)
}

fn value(d: &ColumnData<'static>) -> JsonValue {
    let text = |s: Option<String>| s.map_or(JsonValue::Null, JsonValue::String);
    match d {
        ColumnData::U8(v) => v.map_or(JsonValue::Null, JsonValue::from),
        ColumnData::I16(v) => v.map_or(JsonValue::Null, JsonValue::from),
        ColumnData::I32(v) => v.map_or(JsonValue::Null, JsonValue::from),
        ColumnData::I64(v) => v.map_or(JsonValue::Null, |n| if n.abs() <= SAFE_INTEGER { n.into() } else { n.to_string().into() }),
        ColumnData::F32(v) => v.map_or(JsonValue::Null, |n| serde_json::Number::from_f64(n as f64).map_or(JsonValue::Null, JsonValue::Number)),
        ColumnData::F64(v) => v.map_or(JsonValue::Null, |n| serde_json::Number::from_f64(n).map_or(JsonValue::Null, JsonValue::Number)),
        ColumnData::Bit(v) => v.map_or(JsonValue::Null, JsonValue::Bool),
        ColumnData::String(v) => text(v.as_ref().map(|s| s.to_string())),
        ColumnData::Guid(v) => text(v.map(|g| g.to_string())),
        ColumnData::Numeric(v) => text(v.map(|n| n.to_string())),
        ColumnData::Binary(v) => v.as_ref().map_or(JsonValue::Null, |b| binary_label(b.len())),
        ColumnData::Xml(v) => text(v.as_ref().map(|x| x.to_string())),
        ColumnData::Date(_) => text(NaiveDate::from_sql(d).ok().flatten().map(|v| v.to_string())),
        ColumnData::Time(_) => text(NaiveTime::from_sql(d).ok().flatten().map(|v| v.to_string())),
        ColumnData::DateTime(_) | ColumnData::SmallDateTime(_) | ColumnData::DateTime2(_) => {
            text(NaiveDateTime::from_sql(d).ok().flatten().map(|v| v.to_string()))
        }
        ColumnData::DateTimeOffset(_) => text(DateTime::<FixedOffset>::from_sql(d).ok().flatten().map(|v| v.to_string())),
    }
}

pub async fn version(c: &Connection) -> AppResult<String> {
    let r = run(c, "SELECT @@VERSION", 1).await?;
    let v = r.rows.first().and_then(|row| row.first()).and_then(|v| v.as_str()).unwrap_or("SQL Server");
    Ok(v.lines().next().unwrap_or(v).trim().to_string())
}

pub async fn tables(c: &Connection) -> AppResult<Vec<RemoteTable>> {
    let r = run(
        c,
        "SELECT TABLE_SCHEMA, TABLE_NAME, TABLE_TYPE FROM INFORMATION_SCHEMA.TABLES \
         WHERE TABLE_SCHEMA NOT IN ('sys', 'INFORMATION_SCHEMA') ORDER BY CASE WHEN TABLE_SCHEMA = 'dbo' THEN 0 ELSE 1 END, TABLE_SCHEMA, TABLE_NAME",
        100_000,
    )
    .await?;
    Ok(tables_from(r, true))
}

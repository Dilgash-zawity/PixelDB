//! Reading online databases: the tables and views of a saved connection, a
//! page of a table's rows, and read-only queries. Nothing here writes.
//!
//! Values cross the API as JSON: numbers and booleans when the server type
//! says so, everything else (decimals, dates, JSON, UUIDs…) as the text the
//! server prints for it, so exact values are never rounded.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};

use crate::database::queries::{MAX_QUERY_ROWS, QUERY_TIMEOUT, normalize};
use crate::errors::{AppError, AppResult};
use crate::online::{ConnectionSettings, Engine, connection_error};

/// Longest a page may be when browsing a table.
pub const MAX_PAGE_ROWS: u32 = 500;
/// Time allowed for connecting on top of the query timeout.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Connections kept per server. Opening a table sends several requests at
/// once (its description, a page of rows, the linked tables' labels), and
/// each holds a connection for a few round trips.
const POOL_SIZE: u32 = 6;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnlineObjectKind {
    Table,
    View,
}

/// A table or view on the server. `schema` is empty where the engine has
/// none (libSQL); in MySQL it is the database the table lives in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnlineObject {
    pub schema: String,
    pub name: String,
    pub kind: OnlineObjectKind,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OnlineSort {
    pub column: String,
    #[serde(default)]
    pub descending: bool,
}

/// Which page of which table to read.
#[derive(Debug, Clone, Deserialize)]
pub struct BrowseRequest {
    pub object: OnlineObject,
    #[serde(default)]
    pub sort: Option<OnlineSort>,
    #[serde(default)]
    pub offset: u64,
    pub limit: u32,
}

/// Rows read from an online database.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OnlineRows {
    pub columns: Vec<String>,
    /// The server's name for each column's type, lower-case (`int4`, `varchar`…).
    pub types: Vec<String>,
    pub rows: Vec<Vec<JsonValue>>,
    /// More rows exist than were returned (queries only).
    pub truncated: bool,
    /// Rows in the whole table (browsing only).
    pub total: Option<u64>,
    pub elapsed_ms: u64,
}

/// Connection pools, one per connection (settings and secret), so paging
/// through a table does not connect again for every page. Idle connections
/// close on their own. Cloning shares the same pools.
#[derive(Clone, Default)]
pub struct OnlinePools {
    pg: Arc<Mutex<HashMap<String, sqlx::PgPool>>>,
    mysql: Arc<Mutex<HashMap<String, sqlx::MySqlPool>>>,
    /// Table descriptions read recently, per connection (`pool_key`).
    pub(crate) catalog: Arc<Mutex<HashMap<String, crate::online_records::Catalog>>>,
    /// One gate per catalog entry being read, so requests arriving together
    /// wait for the first one's answer instead of each asking the server.
    gates: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}

pub(crate) fn pool_key(s: &ConnectionSettings, secret: &str) -> String {
    format!("{}\u{0}{}\u{0}{:?}\u{0}{}\u{0}{}\u{0}{}\u{0}{secret}", s.provider, s.host, s.port, s.database, s.user, s.ssl)
}

pub(crate) fn lock<T>(m: &Mutex<T>) -> AppResult<std::sync::MutexGuard<'_, T>> {
    m.lock().map_err(|_| AppError::Internal("connection pool lock poisoned".into()))
}

impl OnlinePools {
    /// The gate for catalog entry `key`; hold its lock while reading it.
    pub(crate) fn gate(&self, key: String) -> AppResult<Arc<tokio::sync::Mutex<()>>> {
        Ok(lock(&self.gates)?.entry(key).or_default().clone())
    }

    pub(crate) fn pg(&self, s: &ConnectionSettings, secret: &str) -> AppResult<sqlx::PgPool> {
        use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
        let mut pools = lock(&self.pg)?;
        let key = pool_key(s, secret);
        if let Some(p) = pools.get(&key) {
            return Ok(p.clone());
        }
        let mut opts = PgConnectOptions::new()
            .host(&s.host)
            .port(s.port.unwrap_or(5432))
            .username(&s.user)
            .ssl_mode(if s.ssl { PgSslMode::Require } else { PgSslMode::Prefer })
            .application_name("SlateDB");
        if !s.database.is_empty() {
            opts = opts.database(&s.database);
        }
        // An empty secret means no password; sending one fails where none is set.
        if !secret.is_empty() {
            opts = opts.password(secret);
        }
        let pool = PgPoolOptions::new()
            .max_connections(POOL_SIZE)
            .min_connections(0)
            .idle_timeout(Duration::from_secs(120))
            .acquire_timeout(CONNECT_TIMEOUT)
            .connect_lazy_with(opts);
        pools.insert(key, pool.clone());
        Ok(pool)
    }

    pub(crate) fn mysql(&self, s: &ConnectionSettings, secret: &str) -> AppResult<sqlx::MySqlPool> {
        use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions, MySqlSslMode};
        let mut pools = lock(&self.mysql)?;
        let key = pool_key(s, secret);
        if let Some(p) = pools.get(&key) {
            return Ok(p.clone());
        }
        let mut opts = MySqlConnectOptions::new()
            .host(&s.host)
            .port(s.port.unwrap_or(3306))
            .username(&s.user)
            .ssl_mode(if s.ssl { MySqlSslMode::Required } else { MySqlSslMode::Preferred });
        if !s.database.is_empty() {
            opts = opts.database(&s.database);
        }
        // An empty secret means no password; sending one fails where none is set.
        if !secret.is_empty() {
            opts = opts.password(secret);
        }
        let pool = MySqlPoolOptions::new()
            .max_connections(POOL_SIZE)
            .min_connections(0)
            .idle_timeout(Duration::from_secs(120))
            .acquire_timeout(CONNECT_TIMEOUT)
            .connect_lazy_with(opts);
        pools.insert(key, pool.clone());
        Ok(pool)
    }
}

/// Errors the server reports about the SQL (syntax, unknown table,
/// permission) are the user's to fix; anything else is the connection.
pub(crate) fn sqlx_error(e: sqlx::Error) -> AppError {
    match e {
        // SQLSTATE 08 (connection) and 28 (authorization): the connection settings are wrong.
        sqlx::Error::Database(d) if d.code().is_some_and(|c| c.starts_with("08") || c.starts_with("28")) => {
            AppError::Connection(d.message().to_string())
        }
        sqlx::Error::Database(d) => match d.kind() {
            sqlx::error::ErrorKind::Other => AppError::Validation(d.message().to_string()),
            // Unique, foreign key, not-null and check violations.
            _ => AppError::Constraint(d.message().to_string()),
        },
        sqlx::Error::PoolTimedOut => AppError::Connection("the server did not answer in time".into()),
        e => connection_error(e),
    }
}

pub(crate) async fn within<T>(host: &str, work: impl std::future::Future<Output = AppResult<T>>) -> AppResult<T> {
    let limit = QUERY_TIMEOUT + CONNECT_TIMEOUT;
    tokio::time::timeout(limit, work)
        .await
        .map_err(|_| AppError::Connection(format!("{host} did not answer within {} seconds", limit.as_secs())))?
}

pub(crate) fn elapsed(started: Instant) -> u64 {
    started.elapsed().as_millis() as u64
}

/// A value bound to a statement.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Param {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

impl Param {
    /// The text Postgres reads the value from inside `CAST($n::text AS …)`.
    pub(crate) fn into_text(self) -> Param {
        match self {
            Param::Null | Param::Text(_) => self,
            Param::Bool(b) => Param::Text(b.to_string()),
            Param::Int(n) => Param::Text(n.to_string()),
            Param::Float(f) => Param::Text(f.to_string()),
        }
    }

    pub(crate) fn json(&self) -> JsonValue {
        match self {
            Param::Null => JsonValue::Null,
            // SQLite has no booleans.
            Param::Bool(b) => json!(i64::from(*b)),
            Param::Int(n) => json!(n),
            Param::Float(f) => json!(f),
            Param::Text(t) => json!(t),
        }
    }
}

pub(crate) fn pg_bind<'q>(
    q: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    p: &Param,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    match p {
        Param::Null => q.bind(None::<String>),
        Param::Bool(b) => q.bind(*b),
        Param::Int(n) => q.bind(*n),
        Param::Float(f) => q.bind(*f),
        Param::Text(t) => q.bind(t.clone()),
    }
}

pub(crate) fn mysql_bind<'q>(
    q: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
    p: &Param,
) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
    match p {
        Param::Null => q.bind(None::<String>),
        Param::Bool(b) => q.bind(*b),
        Param::Int(n) => q.bind(*n),
        Param::Float(f) => q.bind(*f),
        Param::Text(t) => q.bind(t.clone()),
    }
}

// ── Identifiers ──

/// Quotes an identifier for `engine`, doubling the quote character.
pub(crate) fn quote(engine: Engine, name: &str) -> AppResult<String> {
    if name.is_empty() || name.contains('\0') {
        return Err(AppError::InvalidIdentifier(format!("{name:?}")));
    }
    Ok(match engine {
        Engine::Mysql => format!("`{}`", name.replace('`', "``")),
        Engine::Postgres | Engine::Libsql => format!("\"{}\"", name.replace('"', "\"\"")),
    })
}

/// `schema.name`, or just `name` when there is no schema.
pub(crate) fn qualified(engine: Engine, o: &OnlineObject) -> AppResult<String> {
    let name = quote(engine, &o.name)?;
    Ok(if o.schema.is_empty() { name } else { format!("{}.{name}", quote(engine, &o.schema)?) })
}

/// Index of the sort column among `columns` and the direction keyword.
fn sort_target(columns: &[String], sort: Option<&OnlineSort>) -> AppResult<Option<(usize, &'static str)>> {
    let Some(sort) = sort else { return Ok(None) };
    let i = columns
        .iter()
        .position(|c| *c == sort.column)
        .ok_or_else(|| AppError::NotFound(format!("column {:?}", sort.column)))?;
    Ok(Some((i, if sort.descending { "DESC" } else { "ASC" })))
}

/// ` ORDER BY <column> <dir>` for MySQL and libSQL, which sort by name.
fn order_clause(engine: Engine, columns: &[String], sort: Option<&OnlineSort>) -> AppResult<String> {
    Ok(match sort_target(columns, sort)? {
        Some((i, dir)) => format!(" ORDER BY {} {dir}", quote(engine, &columns[i])?),
        None => String::new(),
    })
}

/// Postgres: selects every column of `from` as text, renamed positionally
/// so duplicate or odd column names in a query do not matter.
fn pg_text_select(count: usize, from: &str) -> String {
    let aliases: Vec<String> = (1..=count).map(|i| format!("\"c{i}\"")).collect();
    let cols: Vec<String> = aliases.iter().map(|a| format!("q.{a}::text")).collect();
    format!("SELECT {} FROM ({from}) AS q({})", cols.join(", "), aliases.join(", "))
}

// ── Postgres values ──

/// A type as sqlx names it (`INT4`, `INT4[]`, a custom type's own name),
/// written so it can follow `AS` in a cast.
pub(crate) fn pg_cast_type(name: &str) -> String {
    let (base, array) = match name.strip_suffix("[]") {
        Some(b) => (b, "[]"),
        None => (name, ""),
    };
    // Built-in types are reported in capitals; their real names are lower-case.
    let base = if base.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) { base.to_ascii_lowercase() } else { base.to_string() };
    if base.is_empty() || base == "unknown" || base == "void" {
        return "text".into();
    }
    format!("\"{}\"{array}", base.replace('"', "\"\""))
}

/// Turns the text Postgres prints for a value into JSON by its type.
pub(crate) fn pg_value(type_name: &str, text: Option<String>) -> JsonValue {
    let Some(t) = text else { return JsonValue::Null };
    match type_name {
        "bool" => match t.as_str() {
            "t" | "true" => JsonValue::Bool(true),
            "f" | "false" => JsonValue::Bool(false),
            _ => JsonValue::String(t),
        },
        "int2" | "int4" | "int8" | "oid" => t.parse::<i64>().map(JsonValue::from).unwrap_or(JsonValue::String(t)),
        "float4" | "float8" => t
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::String(t)),
        _ => JsonValue::String(t),
    }
}

/// Columns, their types and the rows read.
pub(crate) type Read = (Vec<String>, Vec<String>, Vec<Vec<JsonValue>>);

/// Prepares `sql` to learn its columns, then reads it with every value as
/// text. `limit` is a page (rows, offset); without it, one row more than
/// `MAX_QUERY_ROWS` is read so the caller can tell the result was cut off.
/// `params` are `$1`… in `sql`.
pub(crate) async fn pg_read(
    conn: &mut sqlx::PgConnection,
    sql: &str,
    limit: Option<(u32, u64)>,
    sort: Option<&OnlineSort>,
    params: &[Param],
) -> AppResult<Read> {
    use sqlx::{Column, Executor, Row, Statement, TypeInfo};
    let stmt = (&mut *conn).prepare(sql).await.map_err(sqlx_error)?;
    let columns: Vec<String> = stmt.columns().iter().map(|c| c.name().to_string()).collect();
    let types: Vec<String> = stmt.columns().iter().map(|c| c.type_info().name().to_ascii_lowercase()).collect();
    if columns.is_empty() {
        return Err(AppError::Unsupported("the query returns no columns".into()));
    }
    let mut text = pg_text_select(columns.len(), sql);
    if let Some((i, dir)) = sort_target(&columns, sort)? {
        // Empty values last either way, as on the Datasheet.
        text.push_str(&format!(" ORDER BY q.\"c{}\" {dir} NULLS LAST", i + 1));
    }
    match limit {
        Some((n, offset)) => text.push_str(&format!(" LIMIT {n} OFFSET {offset}")),
        None => text.push_str(&format!(" LIMIT {}", MAX_QUERY_ROWS + 1)),
    }
    let rows = params
        .iter()
        .fold(sqlx::query(&text), pg_bind)
        .fetch_all(&mut *conn)
        .await
        .map_err(sqlx_error)?;
    let rows = rows
        .iter()
        .map(|r| {
            (0..columns.len())
                .map(|i| r.try_get::<Option<String>, _>(i).map(|t| pg_value(&types[i], t)))
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(sqlx_error)?;
    Ok((columns, types, rows))
}

/// A read-only transaction that gives up after the query timeout.
// Boxed with an explicit `Send` for the same reason as `mysql_begin`.
pub(crate) fn pg_begin(pool: &sqlx::PgPool) -> BoxFuture<'_, AppResult<sqlx::Transaction<'static, sqlx::Postgres>>> {
    Box::pin(async move {
        use sqlx::Executor;
        let mut tx = pool.begin().await.map_err(sqlx_error)?;
        // A plain string runs unprepared, so both settings go in one round trip.
        let settings = format!("SET TRANSACTION READ ONLY; SET LOCAL statement_timeout = {}", QUERY_TIMEOUT.as_millis());
        (&mut *tx).execute(settings.as_str()).await.map_err(sqlx_error)?;
        Ok(tx)
    })
}

// ── MySQL values ──

/// Reads one value of a MySQL row (binary protocol) as JSON by its type.
pub(crate) fn mysql_value(row: &sqlx::mysql::MySqlRow, i: usize, type_name: &str) -> Result<JsonValue, sqlx::Error> {
    use sqlx::{Row, ValueRef};
    if row.try_get_raw(i)?.is_null() {
        return Ok(JsonValue::Null);
    }
    let bytes = || row.try_get_unchecked::<Vec<u8>, _>(i);
    Ok(match type_name {
        "BOOLEAN" => JsonValue::Bool(row.try_get_unchecked::<i64, _>(i)? != 0),
        "TINYINT" | "SMALLINT" | "MEDIUMINT" | "INT" | "BIGINT" | "YEAR" => json!(row.try_get_unchecked::<i64, _>(i)?),
        t if t.ends_with(" UNSIGNED") => json!(row.try_get_unchecked::<u64, _>(i)?),
        "FLOAT" => json!(row.try_get_unchecked::<f32, _>(i)? as f64),
        "DOUBLE" => serde_json::Number::from_f64(row.try_get_unchecked::<f64, _>(i)?).map_or(JsonValue::Null, JsonValue::Number),
        "DATE" | "DATETIME" | "TIMESTAMP" => JsonValue::String(mysql_datetime(&bytes()?, type_name == "DATE")),
        "TIME" => JsonValue::String(row.try_get_unchecked::<sqlx::mysql::types::MySqlTime, _>(i)?.to_string()),
        "BIT" => json!(bytes()?.iter().fold(0u64, |n, b| (n << 8) | u64::from(*b))),
        _ => {
            let b = bytes()?;
            match String::from_utf8(b) {
                Ok(s) => JsonValue::String(s),
                Err(e) => json!({ "blob": { "size": e.as_bytes().len() } }),
            }
        }
    })
}

/// Formats a binary-protocol DATE/DATETIME/TIMESTAMP:
/// `[len, year lo, year hi, month, day, (hour, minute, second, (µs×4))]`.
fn mysql_datetime(buf: &[u8], date_only: bool) -> String {
    let len = buf.first().copied().unwrap_or(0) as usize;
    let b = &buf[1.min(buf.len())..];
    if len < 4 || b.len() < 4 {
        return if date_only { "0000-00-00".into() } else { "0000-00-00 00:00:00".into() };
    }
    let date = format!("{:04}-{:02}-{:02}", u16::from_le_bytes([b[0], b[1]]), b[2], b[3]);
    if date_only {
        return date;
    }
    let (h, m, s) = if len >= 7 && b.len() >= 7 { (b[4], b[5], b[6]) } else { (0, 0, 0) };
    let mut out = format!("{date} {h:02}:{m:02}:{s:02}");
    if len >= 11 && b.len() >= 11 {
        let micros = u32::from_le_bytes([b[7], b[8], b[9], b[10]]);
        if micros != 0 {
            out.push_str(&format!(".{micros:06}"));
        }
    }
    out
}

/// Reads `sql` (prepared, so a single statement), stopping after
/// `MAX_QUERY_ROWS` rows or the page size; the flag says rows were left over.
/// `params` are the `?` in `sql`, then the page's two when `limit` is given.
pub(crate) async fn mysql_read(
    conn: &mut sqlx::MySqlConnection,
    sql: &str,
    limit: Option<(u32, u64)>,
    params: &[Param],
) -> AppResult<(Vec<String>, Vec<String>, Vec<Vec<JsonValue>>, bool)> {
    use futures_util::TryStreamExt;
    use sqlx::{Column, Executor, Statement, TypeInfo};
    let stmt = (&mut *conn).prepare(sql).await.map_err(sqlx_error)?;
    let columns: Vec<String> = stmt.columns().iter().map(|c| c.name().to_string()).collect();
    let types: Vec<String> = stmt.columns().iter().map(|c| c.type_info().name().to_string()).collect();
    if columns.is_empty() {
        return Err(AppError::Unsupported("the query returns no columns".into()));
    }
    let max = limit.map_or(MAX_QUERY_ROWS, |(n, _)| n as usize);
    let mut out = Vec::new();
    let mut truncated = false;
    {
        let query = params.iter().fold(sqlx::query(sql), mysql_bind);
        let query = match limit {
            Some((n, offset)) => query.bind(n).bind(offset),
            None => query,
        };
        let mut stream = query.fetch(&mut *conn);
        while let Some(row) = stream.try_next().await.map_err(sqlx_error)? {
            if out.len() == max {
                truncated = true;
                break;
            }
            out.push((0..columns.len()).map(|i| mysql_value(&row, i, &types[i])).collect::<Result<Vec<_>, _>>().map_err(sqlx_error)?);
        }
    }
    Ok((columns, types.iter().map(|t| t.to_ascii_lowercase()).collect(), out, truncated))
}

// The MySQL transaction helpers are boxed with an explicit `Send`: without
// it rustc can't prove the command futures that hold them are `Send`
// ("implementation of Executor is not general enough").

/// A connection in a read-only transaction; the caller rolls it back.
pub(crate) fn mysql_begin(pool: &sqlx::MySqlPool) -> BoxFuture<'_, AppResult<sqlx::pool::PoolConnection<sqlx::MySql>>> {
    Box::pin(async move {
        use sqlx::Executor;
        let mut conn = pool.acquire().await.map_err(sqlx_error)?;
        // A plain string runs unprepared, which transaction statements need.
        (&mut *conn).execute("START TRANSACTION READ ONLY").await.map_err(sqlx_error)?;
        Ok(conn)
    })
}

pub(crate) fn mysql_end(mut conn: sqlx::pool::PoolConnection<sqlx::MySql>) -> BoxFuture<'static, ()> {
    Box::pin(async move {
        use sqlx::Executor;
        if (&mut *conn).execute("ROLLBACK").await.is_err() {
            // Don't hand a connection in an unknown state back to the pool.
            let _ = conn.detach();
        }
    })
}

// ── libSQL (Hrana over HTTP) ──

pub(crate) struct LibsqlResult {
    pub columns: Vec<String>,
    pub types: Vec<String>,
    pub rows: Vec<Vec<JsonValue>>,
}

fn hrana_arg(v: &JsonValue) -> JsonValue {
    match v {
        JsonValue::Null => json!({ "type": "null" }),
        JsonValue::Bool(b) => json!({ "type": "integer", "value": i64::from(*b).to_string() }),
        JsonValue::Number(n) if n.is_i64() => json!({ "type": "integer", "value": n.to_string() }),
        JsonValue::Number(n) => json!({ "type": "float", "value": n.as_f64() }),
        JsonValue::String(s) => json!({ "type": "text", "value": s }),
        other => json!({ "type": "text", "value": other.to_string() }),
    }
}

pub(crate) fn hrana_value(v: &JsonValue) -> JsonValue {
    match v["type"].as_str() {
        Some("integer") => {
            let s = v["value"].as_str().unwrap_or_default();
            s.parse::<i64>().map(JsonValue::from).unwrap_or_else(|_| JsonValue::String(s.to_string()))
        }
        Some("float") => v["value"].clone(),
        Some("text") => v["value"].clone(),
        Some("blob") => {
            let b64 = v["base64"].as_str().unwrap_or_default().trim_end_matches('=');
            json!({ "blob": { "size": b64.len() * 3 / 4 } })
        }
        _ => JsonValue::Null,
    }
}

/// A statement in Hrana's shape.
pub(crate) fn hrana_stmt(sql: &str, args: &[JsonValue]) -> JsonValue {
    let args: Vec<JsonValue> = args.iter().map(hrana_arg).collect();
    json!({ "sql": sql, "args": args, "want_rows": true })
}

/// Sends one request (and a close) to the server's pipeline endpoint and
/// returns that request's response; a failed request is a `Validation` error.
pub(crate) async fn libsql_request(s: &ConnectionSettings, token: &str, request: JsonValue) -> AppResult<JsonValue> {
    let url = match s.port {
        Some(p) => format!("https://{}:{p}/v2/pipeline", s.host),
        None => format!("https://{}/v2/pipeline", s.host),
    };
    let body = json!({ "requests": [request, { "type": "close" }] });
    let mut req = reqwest::Client::new().post(&url).timeout(QUERY_TIMEOUT).json(&body);
    if !token.is_empty() {
        req = req.bearer_auth(token);
    }
    let res = req.send().await.map_err(connection_error)?;
    let status = res.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(AppError::Connection("the server rejected the token".into()));
    }
    let text = res.text().await.map_err(connection_error)?;
    if !status.is_success() {
        return Err(AppError::Connection(format!("the server answered {status}: {}", text.chars().take(200).collect::<String>())));
    }
    let mut v: JsonValue = serde_json::from_str(&text).map_err(connection_error)?;
    let first = v["results"][0].take();
    if first["type"] == "error" {
        return Err(AppError::Validation(first["error"]["message"].as_str().unwrap_or("the query failed").to_string()));
    }
    Ok(first["response"].clone())
}

/// Reads a Hrana statement result.
pub(crate) fn libsql_result(result: &JsonValue) -> LibsqlResult {
    let cols = result["cols"].as_array().cloned().unwrap_or_default();
    LibsqlResult {
        columns: cols.iter().map(|c| c["name"].as_str().unwrap_or_default().to_string()).collect(),
        types: cols.iter().map(|c| c["decltype"].as_str().unwrap_or_default().to_ascii_lowercase()).collect(),
        rows: result["rows"]
            .as_array()
            .map(|rows| rows.iter().map(|r| r.as_array().map(|r| r.iter().map(hrana_value).collect()).unwrap_or_default()).collect())
            .unwrap_or_default(),
    }
}

pub(crate) async fn libsql_execute(s: &ConnectionSettings, token: &str, sql: &str, args: &[JsonValue]) -> AppResult<LibsqlResult> {
    let response = libsql_request(s, token, json!({ "type": "execute", "stmt": hrana_stmt(sql, args) })).await?;
    Ok(libsql_result(&response["result"]))
}

// ── Listing ──

/// Schemas that belong to the server, not the user.
const PG_SYSTEM_SCHEMAS: &[&str] = &["pg_catalog", "information_schema", "pg_toast", "crdb_internal", "pg_extension"];

pub async fn list_objects(pools: &OnlinePools, s: &ConnectionSettings, secret: &str) -> AppResult<Vec<OnlineObject>> {
    s.validate()?;
    let kind = |t: &str| if t.to_ascii_uppercase().contains("VIEW") { OnlineObjectKind::View } else { OnlineObjectKind::Table };
    within(&s.host, async {
        let mut out: Vec<OnlineObject> = match s.engine {
            Engine::Postgres => {
                let pool = pools.pg(s, secret)?;
                let rows: Vec<(String, String, String)> = sqlx::query_as(
                    "SELECT table_schema::text, table_name::text, table_type::text FROM information_schema.tables \
                     WHERE table_schema::text <> ALL($1) AND table_schema::text NOT LIKE 'pg\\_temp%' \
                     AND table_type::text IN ('BASE TABLE', 'VIEW', 'FOREIGN', 'FOREIGN TABLE')",
                )
                .bind(PG_SYSTEM_SCHEMAS)
                .fetch_all(&pool)
                .await
                .map_err(sqlx_error)?;
                rows.into_iter().map(|(schema, name, t)| OnlineObject { schema, name, kind: kind(&t) }).collect()
            }
            Engine::Mysql => {
                use sqlx::Row;
                let pool = pools.mysql(s, secret)?;
                let rows = sqlx::query(
                    "SELECT CAST(TABLE_SCHEMA AS CHAR), CAST(TABLE_NAME AS CHAR), CAST(TABLE_TYPE AS CHAR) \
                     FROM information_schema.TABLES \
                     WHERE (DATABASE() IS NULL OR TABLE_SCHEMA = DATABASE()) \
                     AND TABLE_SCHEMA NOT IN ('mysql', 'information_schema', 'performance_schema', 'sys')",
                )
                .fetch_all(&pool)
                .await
                .map_err(sqlx_error)?;
                let text = |r: &sqlx::mysql::MySqlRow, i: usize| -> Result<String, sqlx::Error> {
                    Ok(String::from_utf8_lossy(&r.try_get_unchecked::<Vec<u8>, _>(i)?).into_owned())
                };
                rows.iter()
                    .map(|r| Ok(OnlineObject { schema: text(r, 0)?, name: text(r, 1)?, kind: kind(&text(r, 2)?) }))
                    .collect::<Result<_, sqlx::Error>>()
                    .map_err(sqlx_error)?
            }
            Engine::Libsql => {
                let r = libsql_execute(
                    s,
                    secret,
                    "SELECT name, type FROM sqlite_schema WHERE type IN ('table', 'view') \
                     AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' AND name NOT LIKE '\\_\\_app\\_%' ESCAPE '\\' \
                     AND name NOT LIKE '\\_litestream%' ESCAPE '\\' AND name NOT LIKE 'libsql\\_%' ESCAPE '\\'",
                    &[],
                )
                .await?;
                r.rows
                    .iter()
                    .map(|row| OnlineObject {
                        schema: String::new(),
                        name: row.first().and_then(JsonValue::as_str).unwrap_or_default().to_string(),
                        kind: kind(row.get(1).and_then(JsonValue::as_str).unwrap_or_default()),
                    })
                    .collect()
            }
        };
        sort_objects(&mut out);
        Ok(out)
    })
    .await
}

/// `public` (Postgres' default schema) first, then by schema and name.
fn sort_objects(objects: &mut [OnlineObject]) {
    objects.sort_by(|a, b| {
        (a.schema != "public", a.schema.to_lowercase(), a.name.to_lowercase())
            .cmp(&(b.schema != "public", b.schema.to_lowercase(), b.name.to_lowercase()))
    });
}

// ── Browsing and queries ──

/// A page of a table's or view's rows, with the number of rows in all.
pub async fn browse(pools: &OnlinePools, s: &ConnectionSettings, secret: &str, request: &BrowseRequest) -> AppResult<OnlineRows> {
    s.validate()?;
    let (object, sort, offset) = (&request.object, request.sort.as_ref(), request.offset);
    let limit = request.limit.clamp(1, MAX_PAGE_ROWS);
    let from = qualified(s.engine, object)?;
    let started = Instant::now();
    within(&s.host, async {
        let base = format!("SELECT * FROM {from}");
        let count_sql = format!("SELECT count(*) FROM {from}");
        let (columns, types, rows, total) = match s.engine {
            Engine::Postgres => {
                let pool = pools.pg(s, secret)?;
                let mut tx = pg_begin(&pool).await?;
                let (c, t, r) = pg_read(&mut tx, &base, Some((limit, offset)), sort, &[]).await?;
                let (total,): (i64,) = sqlx::query_as(&count_sql).fetch_one(&mut *tx).await.map_err(sqlx_error)?;
                (c, t, r, total)
            }
            Engine::Mysql => {
                let pool = pools.mysql(s, secret)?;
                let mut conn = mysql_begin(&pool).await?;
                let result = async {
                    use sqlx::{Column, Executor, Statement};
                    let stmt = (&mut *conn).prepare(&base).await.map_err(sqlx_error)?;
                    let names: Vec<String> = stmt.columns().iter().map(|c| c.name().to_string()).collect();
                    let sql = format!("{base}{} LIMIT ? OFFSET ?", order_clause(s.engine, &names, sort)?);
                    let (c, t, r, _) = mysql_read(&mut conn, &sql, Some((limit, offset)), &[]).await?;
                    let (total,): (i64,) = sqlx::query_as(&count_sql).fetch_one(&mut *conn).await.map_err(sqlx_error)?;
                    Ok::<_, AppError>((c, t, r, total))
                }
                .await;
                mysql_end(conn).await;
                result?
            }
            Engine::Libsql => {
                // Column names first, so the sort column can be checked.
                let head = libsql_execute(s, secret, &format!("{base} LIMIT 0"), &[]).await?;
                let sql = format!("{base}{} LIMIT ? OFFSET ?", order_clause(s.engine, &head.columns, sort)?);
                let r = libsql_execute(s, secret, &sql, &[json!(limit), json!(offset)]).await?;
                let count = libsql_execute(s, secret, &count_sql, &[]).await?;
                let total = count.rows.first().and_then(|r| r.first()).and_then(JsonValue::as_i64).unwrap_or(0);
                (r.columns, head.types, r.rows, total)
            }
        };
        Ok(OnlineRows { columns, types, rows, truncated: false, total: Some(total.max(0) as u64), elapsed_ms: elapsed(started) })
    })
    .await
}

/// Runs one read-only SELECT/WITH/VALUES statement, returning at most
/// `MAX_QUERY_ROWS` rows. The server enforces read-only where it can (a
/// read-only transaction); the statement is also checked to be a single query.
/// `:name` in the SQL (outside quotes and comments) is bound from `params`;
/// names missing there are bound as null.
pub async fn query(
    pools: &OnlinePools,
    s: &ConnectionSettings,
    secret: &str,
    sql: &str,
    params: &serde_json::Map<String, JsonValue>,
) -> AppResult<OnlineRows> {
    use crate::online_records::{param, rewrite_params};
    s.validate()?;
    let body = normalize(sql)?.to_string();
    let value = |name: &str| param(params.get(name).unwrap_or(&JsonValue::Null));
    let started = Instant::now();
    within(&s.host, async {
        let (columns, types, mut rows, mut truncated) = match s.engine {
            Engine::Postgres => {
                // Each name is one `$n`; the server works out its type, and the
                // value is sent as text cast to that type.
                let mut names: Vec<String> = Vec::new();
                let numbered = rewrite_params(s.engine, &body, |name| {
                    let i = names.iter().position(|n| n == name).unwrap_or_else(|| {
                        names.push(name.to_string());
                        names.len() - 1
                    });
                    format!("${}", i + 1)
                });
                let pool = pools.pg(s, secret)?;
                let mut tx = pg_begin(&pool).await?;
                let (sql, values) = if names.is_empty() {
                    (body, Vec::new())
                } else {
                    use sqlx::{Executor, Statement};
                    let stmt = (&mut *tx).prepare(&numbered).await.map_err(sqlx_error)?;
                    let types: Vec<String> = match stmt.parameters() {
                        Some(sqlx::Either::Left(t)) => t.iter().map(|t| pg_cast_type(sqlx::TypeInfo::name(t))).collect(),
                        _ => Vec::new(),
                    };
                    let mut seen: Vec<String> = Vec::new();
                    let cast = rewrite_params(s.engine, &body, |name| {
                        let i = seen.iter().position(|n| n == name).unwrap_or_else(|| {
                            seen.push(name.to_string());
                            seen.len() - 1
                        });
                        let t = types.get(i).cloned().unwrap_or_else(|| "text".into());
                        format!("CAST(${}::text AS {t})", i + 1)
                    });
                    let values = names.iter().map(|n| value(n).map(Param::into_text)).collect::<AppResult<Vec<_>>>()?;
                    (cast, values)
                };
                let (c, t, r) = pg_read(&mut tx, &sql, None, None, &values).await?;
                (c, t, r, false)
            }
            Engine::Mysql => {
                let mut names: Vec<String> = Vec::new();
                let sql = rewrite_params(s.engine, &body, |name| {
                    names.push(name.to_string());
                    "?".into()
                });
                let values = names.iter().map(|n| value(n)).collect::<AppResult<Vec<_>>>()?;
                let pool = pools.mysql(s, secret)?;
                let mut conn = mysql_begin(&pool).await?;
                let result = mysql_read(&mut conn, &sql, None, &values).await;
                mysql_end(conn).await;
                result?
            }
            Engine::Libsql => {
                let mut names: Vec<String> = Vec::new();
                let sql = rewrite_params(s.engine, &body, |name| {
                    names.push(name.to_string());
                    "?".into()
                });
                let args = names.iter().map(|n| value(n).map(|p| p.json())).collect::<AppResult<Vec<_>>>()?;
                // A subquery can only read, and holds exactly one statement.
                let r = libsql_execute(s, secret, &format!("SELECT * FROM ({sql}) LIMIT {}", MAX_QUERY_ROWS + 1), &args).await?;
                (r.columns, r.types, r.rows, false)
            }
        };
        if rows.len() > MAX_QUERY_ROWS {
            rows.truncate(MAX_QUERY_ROWS);
            truncated = true;
        }
        Ok(OnlineRows { columns, types, rows, truncated, total: None, elapsed_ms: elapsed(started) })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_identifiers_per_engine() {
        assert_eq!(quote(Engine::Postgres, "a\"b").unwrap(), "\"a\"\"b\"");
        assert_eq!(quote(Engine::Mysql, "a`b").unwrap(), "`a``b`");
        assert!(quote(Engine::Libsql, "").is_err());
        let o = OnlineObject { schema: "sales".into(), name: "Order Lines".into(), kind: OnlineObjectKind::Table };
        assert_eq!(qualified(Engine::Postgres, &o).unwrap(), "\"sales\".\"Order Lines\"");
        let bare = OnlineObject { schema: String::new(), ..o };
        assert_eq!(qualified(Engine::Libsql, &bare).unwrap(), "\"Order Lines\"");
    }

    #[test]
    fn sorting_only_by_known_columns() {
        let cols = vec!["id".to_string(), "na`me".to_string()];
        let sort = OnlineSort { column: "na`me".into(), descending: true };
        assert_eq!(sort_target(&cols, Some(&sort)).unwrap(), Some((1, "DESC")));
        assert_eq!(order_clause(Engine::Mysql, &cols, Some(&sort)).unwrap(), " ORDER BY `na``me` DESC");
        let unknown = OnlineSort { column: "1; DROP TABLE x".into(), descending: false };
        assert!(order_clause(Engine::Mysql, &cols, Some(&unknown)).is_err());
        assert_eq!(order_clause(Engine::Libsql, &cols, None).unwrap(), "");
    }

    #[test]
    fn postgres_text_wrapper_and_values() {
        assert_eq!(
            pg_text_select(2, "SELECT 1, 1"),
            "SELECT q.\"c1\"::text, q.\"c2\"::text FROM (SELECT 1, 1) AS q(\"c1\", \"c2\")"
        );
        assert_eq!(pg_value("bool", Some("t".into())), json!(true));
        assert_eq!(pg_value("int8", Some("9007199254740993".into())), json!(9007199254740993i64));
        assert_eq!(pg_value("numeric", Some("12.50".into())), json!("12.50"), "decimals stay exact");
        assert_eq!(pg_value("float8", Some("NaN".into())), json!("NaN"));
        assert_eq!(pg_value("text", None), JsonValue::Null);
    }

    #[test]
    fn mysql_dates_from_binary() {
        assert_eq!(mysql_datetime(&[4, 0xE8, 0x07, 3, 9], true), "2024-03-09");
        assert_eq!(mysql_datetime(&[7, 0xE8, 0x07, 3, 9, 14, 5, 7], false), "2024-03-09 14:05:07");
        assert_eq!(mysql_datetime(&[11, 0xE8, 0x07, 3, 9, 0, 0, 0, 0x40, 0xE2, 0x01, 0], false), "2024-03-09 00:00:00.123456");
        assert_eq!(mysql_datetime(&[0], false), "0000-00-00 00:00:00");
    }

    #[test]
    fn hrana_values() {
        assert_eq!(hrana_value(&json!({"type": "integer", "value": "42"})), json!(42));
        assert_eq!(hrana_value(&json!({"type": "text", "value": "hi"})), json!("hi"));
        assert_eq!(hrana_value(&json!({"type": "null"})), JsonValue::Null);
        assert_eq!(hrana_arg(&json!(5)), json!({"type": "integer", "value": "5"}));
    }

    #[test]
    fn public_schema_first() {
        let o = |schema: &str, name: &str| OnlineObject { schema: schema.into(), name: name.into(), kind: OnlineObjectKind::Table };
        let mut list = vec![o("auth", "users"), o("public", "orders"), o("public", "Customers")];
        sort_objects(&mut list);
        let names: Vec<_> = list.iter().map(|x| format!("{}.{}", x.schema, x.name)).collect();
        assert_eq!(names, ["public.Customers", "public.orders", "auth.users"]);
    }
}

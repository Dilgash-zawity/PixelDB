//! Online databases behind the record, table and query commands, so apps
//! and the online database page use them like workspace databases.
//!
//! Tables and views are named as `list_objects` lists them: `orders`, or
//! `sales.orders` outside the default schema (`public` in Postgres, the
//! database connected to in MySQL). Rows are addressed by primary key, so
//! tables without one are read-only. In Postgres every value is sent as text
//! and cast to the column's own type on the server; MySQL and libSQL convert
//! bound values themselves. Nothing here changes the structure of a table.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::future::BoxFuture;
use serde::Serialize;
use serde_json::{Map, Value as JsonValue, json};

use crate::database::queries::{QUERY_TIMEOUT, normalize};
use crate::database::records::{DEFAULT_PAGE_SIZE, MAX_BATCH_OPS, MAX_IN_LIST, MAX_PAGE_SIZE, MAX_SERIES};
use crate::errors::{AppError, AppResult};
use crate::models::{
    Aggregate, AggregateOrder, AggregateQuery, AggregateResult, BatchOp, BatchResult, CalculatedResult, DateBucket,
    FieldDef, FieldType, FilterOp, FilterSpec, FkAction, ForeignKeyDef, LookupValueType, ObjectKind, ObjectSummary,
    QueryResult, RecordKey, RecordPage, RecordQuery, RecordRow, RecordValues, SavedQuery, TableDef, TableInfo,
};
use crate::online::{ConnectionSettings, Engine};
use crate::online_data::{
    self, OnlineObject, OnlineObjectKind, OnlinePools, OnlineRows, Param, elapsed, hrana_stmt, libsql_execute,
    libsql_request, libsql_result, lock, mysql_begin, mysql_bind, mysql_end, mysql_read, pg_begin, pg_bind, pg_read,
    pg_value, pool_key, qualified, quote, sqlx_error, within,
};

/// How long a table's description is reused before it is read again.
const CATALOG_TTL: Duration = Duration::from_secs(20);

/// Recently read objects and table descriptions of one connection.
#[derive(Default)]
pub struct Catalog {
    objects: Option<(Instant, Arc<Vec<OnlineObject>>)>,
    tables: HashMap<String, (Instant, Arc<OnlineTable>)>,
}

/// A column as the server describes it.
#[derive(Debug, Clone, Serialize)]
pub struct OnlineColumn {
    pub name: String,
    /// The server's type, as it would be written in SQL (`varchar(80)`).
    pub server_type: String,
    /// The closest Access-style type, used by forms and formatting.
    pub field_type: FieldType,
    pub nullable: bool,
    /// The default expression, as the server prints it.
    pub default: Option<String>,
    pub primary_key: bool,
    pub unique: bool,
    /// Filled in by the server (identity, serial, auto_increment).
    pub auto: bool,
    /// Computed by the server; can't be written.
    pub generated: bool,
    pub description: Option<String>,
    /// Postgres: the type values are cast to.
    #[serde(skip)]
    cast: String,
    /// The type name values are decoded by (`int4`, `bool`…).
    #[serde(skip)]
    udt: String,
    #[serde(skip)]
    searchable: bool,
}

/// A table or view of an online database.
#[derive(Debug, Clone, Serialize)]
pub struct OnlineTable {
    /// The name it is listed and addressed by.
    pub name: String,
    pub schema: String,
    pub kind: OnlineObjectKind,
    pub columns: Vec<OnlineColumn>,
    /// Primary key columns; empty when rows can't be addressed.
    pub primary_key: Vec<String>,
    /// `ref_table` is the referenced table's listed name.
    pub foreign_keys: Vec<ForeignKeyDef>,
    pub unique_constraints: Vec<Vec<String>>,
    /// Rows can be added, changed and deleted.
    pub editable: bool,
    /// The server's estimate (or count) of rows; None when unknown.
    pub row_estimate: Option<i64>,
    #[serde(skip)]
    object: Option<OnlineObject>,
}

impl OnlineTable {
    fn column(&self, name: &str) -> AppResult<&OnlineColumn> {
        self.columns
            .iter()
            .find(|c| c.name == name)
            .or_else(|| self.columns.iter().find(|c| c.name.eq_ignore_ascii_case(name)))
            .ok_or_else(|| AppError::Validation(format!("unknown field {name:?} in {:?}", self.name)))
    }

    fn object(&self) -> &OnlineObject {
        self.object.as_ref().expect("described tables keep their object")
    }

    fn require_editable(&self) -> AppResult<()> {
        if self.editable {
            return Ok(());
        }
        Err(AppError::Unsupported(match self.kind {
            OnlineObjectKind::View => format!("{:?} is a view and is read-only", self.name),
            OnlineObjectKind::Table => format!("{:?} has no primary key, so its rows can only be read", self.name),
        }))
    }
}

/// Server facts shown on the online database page.
#[derive(Debug, Clone, Serialize)]
pub struct OnlineInfo {
    pub engine: Engine,
    pub version: String,
    /// Bytes on disk, when the server tells.
    pub size: Option<i64>,
    pub schemas: Vec<String>,
    pub tables: usize,
    pub views: usize,
}

// ── Statements ──

/// SQL text with its bound values, in placeholder order.
struct Sql {
    engine: Engine,
    text: String,
    params: Vec<Param>,
}

impl Sql {
    fn new(engine: Engine, text: impl Into<String>) -> Self {
        Sql { engine, text: text.into(), params: Vec::new() }
    }

    fn push(&mut self, s: &str) -> &mut Self {
        self.text.push_str(s);
        self
    }

    /// Appends a placeholder for `p`. In Postgres a value for a column is
    /// sent as text and cast to the column's type (`cast`).
    fn bind(&mut self, p: Param, cast: Option<&str>) -> &mut Self {
        match self.engine {
            Engine::Postgres => {
                let p = if cast.is_some() { p.into_text() } else { p };
                self.params.push(p);
                let n = self.params.len();
                match cast {
                    Some(c) => self.text.push_str(&format!("CAST(${n}::text AS {c})")),
                    None => self.text.push_str(&format!("${n}")),
                }
            }
            Engine::Mysql | Engine::Libsql => {
                self.params.push(p);
                self.text.push('?');
            }
        }
        self
    }

    /// Appends a value for `col`.
    fn value(&mut self, col: &OnlineColumn, p: Param) -> &mut Self {
        let cast = col.cast.clone();
        self.bind(p, Some(&cast))
    }
}

/// A statement that changes rows; `returning` are column indexes to read back.
struct Write {
    sql: Sql,
    returning: Vec<usize>,
}

#[derive(Debug, Default)]
struct Written {
    row: Option<Vec<JsonValue>>,
    affected: u64,
    last_id: Option<i64>,
}

/// A JSON value from the frontend as a bound value.
pub(crate) fn param(v: &JsonValue) -> AppResult<Param> {
    Ok(match v {
        JsonValue::Null => Param::Null,
        JsonValue::Bool(b) => Param::Bool(*b),
        JsonValue::Number(n) => match n.as_i64() {
            Some(i) => Param::Int(i),
            None => n.as_f64().map(Param::Float).unwrap_or_else(|| Param::Text(n.to_string())),
        },
        JsonValue::String(s) => Param::Text(s.clone()),
        JsonValue::Object(o) if o.contains_key("base64") => {
            return Err(AppError::Unsupported("files can't be saved to online databases yet".into()));
        }
        other => Param::Text(other.to_string()),
    })
}

/// Replaces each `:name` parameter outside quotes, comments and Postgres
/// `::` casts with what `render` returns for the name.
pub(crate) fn rewrite_params(engine: Engine, sql: &str, mut render: impl FnMut(&str) -> String) -> String {
    let chars: Vec<char> = sql.chars().collect();
    let mut out = String::with_capacity(sql.len());
    let mut i = 0;
    let copy_until = |out: &mut String, i: &mut usize, end: &dyn Fn(&[char], usize) -> Option<usize>| {
        let start = *i;
        let stop = end(&chars, start).unwrap_or(chars.len());
        out.extend(&chars[start..stop]);
        *i = stop;
    };
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match c {
            '\'' | '"' | '`' => {
                let backslash = engine == Engine::Mysql;
                copy_until(&mut out, &mut i, &|ch, s| {
                    let mut j = s + 1;
                    while j < ch.len() {
                        if backslash && ch[j] == '\\' {
                            j += 2;
                            continue;
                        }
                        if ch[j] == c {
                            if ch.get(j + 1) == Some(&c) {
                                j += 2;
                                continue;
                            }
                            return Some(j + 1);
                        }
                        j += 1;
                    }
                    None
                });
            }
            '-' if next == Some('-') => copy_until(&mut out, &mut i, &|ch, s| ch[s..].iter().position(|&x| x == '\n').map(|p| s + p)),
            '#' if engine == Engine::Mysql => {
                copy_until(&mut out, &mut i, &|ch, s| ch[s..].iter().position(|&x| x == '\n').map(|p| s + p))
            }
            '/' if next == Some('*') => copy_until(&mut out, &mut i, &|ch, s| {
                (s + 2..ch.len().saturating_sub(1)).find(|&j| ch[j] == '*' && ch[j + 1] == '/').map(|j| j + 2)
            }),
            '$' if engine == Engine::Postgres => {
                // Dollar quoting: $$…$$ or $tag$…$tag$.
                let tag_end = (i + 1..chars.len()).find(|&j| !(chars[j].is_alphanumeric() || chars[j] == '_'));
                match tag_end {
                    Some(t) if chars[t] == '$' && !chars.get(i + 1).is_some_and(char::is_ascii_digit) => {
                        let tag: String = chars[i..=t].iter().collect();
                        let tag_chars: Vec<char> = tag.chars().collect();
                        copy_until(&mut out, &mut i, &|ch, s| {
                            let from = s + tag_chars.len();
                            (from..=ch.len().saturating_sub(tag_chars.len()))
                                .find(|&j| ch[j..j + tag_chars.len()] == tag_chars[..])
                                .map(|j| j + tag_chars.len())
                        });
                    }
                    _ => {
                        out.push(c);
                        i += 1;
                    }
                }
            }
            ':' if next == Some(':') => {
                out.push_str("::");
                i += 2;
            }
            ':' if next.is_some_and(|n| n.is_alphabetic() || n == '_') => {
                let end = (i + 1..chars.len()).find(|&j| !(chars[j].is_alphanumeric() || chars[j] == '_')).unwrap_or(chars.len());
                let name: String = chars[i + 1..end].iter().collect();
                out.push_str(&render(&name));
                i = end;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// A LIKE pattern for `s`, lower-cased, with `\` escaping.
fn like_pattern(s: &str, prefix: bool, suffix: bool) -> String {
    let s = s.to_lowercase();
    let escaped = s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    format!("{}{escaped}{}", if prefix { "%" } else { "" }, if suffix { "%" } else { "" })
}

// ── Types ──

/// The Access-style type closest to a server type. `name` is the type's
/// short name (`int4`, `varchar`, `tinyint`), `full` the full one (`tinyint(1)`).
fn field_type(engine: Engine, name: &str, full: &str, max_length: Option<i64>) -> FieldType {
    let name = name.to_ascii_lowercase();
    let full_lower = full.to_ascii_lowercase();
    let short = |n: Option<i64>| FieldType::ShortText { max_length: n.and_then(|n| u32::try_from(n).ok()) };
    let unknown = || FieldType::Unknown { declared: full.to_string() };
    match engine {
        Engine::Postgres => match name.as_str() {
            "int2" | "int4" | "smallint" | "integer" | "oid" => FieldType::Number { real: false },
            "int8" | "bigint" => FieldType::LargeNumber,
            "float4" | "float8" | "numeric" | "real" | "double precision" => FieldType::Number { real: true },
            "bool" | "boolean" => FieldType::YesNo,
            "date" | "timestamp" | "timestamptz" | "time" | "timetz" => FieldType::DateTime,
            "varchar" | "bpchar" | "char" | "name" | "citext" | "text" => short(max_length),
            _ => unknown(),
        },
        Engine::Mysql => {
            let base = name.trim_end_matches(" unsigned");
            if full_lower.starts_with("tinyint(1)") || base == "boolean" {
                return FieldType::YesNo;
            }
            match base {
                "tinyint" | "smallint" | "mediumint" | "int" | "integer" | "year" => FieldType::Number { real: false },
                "bigint" => FieldType::LargeNumber,
                "decimal" | "numeric" | "float" | "double" | "real" => FieldType::Number { real: true },
                "date" | "datetime" | "timestamp" | "time" => FieldType::DateTime,
                "char" | "varchar" => short(max_length),
                "tinytext" | "text" | "mediumtext" | "longtext" => FieldType::LongText,
                _ => unknown(),
            }
        }
        // SQLite's type affinity rules, on the declared type.
        Engine::Libsql => {
            let t = full_lower.as_str();
            if t.contains("bool") {
                FieldType::YesNo
            } else if t.contains("int") {
                FieldType::Number { real: false }
            } else if t.contains("char") || t.contains("clob") || t.contains("text") {
                short(None)
            } else if t.contains("date") || t.contains("time") {
                FieldType::DateTime
            } else if t.contains("real") || t.contains("floa") || t.contains("doub") || t.contains("num") || t.contains("dec") {
                FieldType::Number { real: true }
            } else {
                unknown()
            }
        }
    }
}

/// Field types of a query's result columns, from the server's type names.
fn result_types(engine: Engine, types: &[String]) -> Vec<Option<FieldType>> {
    types
        .iter()
        .map(|t| match field_type(engine, t, t, None) {
            FieldType::Unknown { .. } => None,
            ft => Some(ft),
        })
        .collect()
}

fn is_binary(engine: Engine, name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    match engine {
        Engine::Postgres => n == "bytea",
        Engine::Mysql => n.contains("blob") || n.contains("binary"),
        Engine::Libsql => n.contains("blob"),
    }
}

fn fk_action(s: &str) -> FkAction {
    match s {
        "r" => FkAction::Restrict,
        "c" => FkAction::Cascade,
        "n" => FkAction::SetNull,
        "d" => FkAction::SetDefault,
        "a" | "" => FkAction::NoAction,
        other => FkAction::parse(other),
    }
}

/// A column as read from the catalog, before keys are known.
struct RawColumn {
    name: String,
    type_name: String,
    full_type: String,
    nullable: bool,
    default: Option<String>,
    max_length: Option<i64>,
    auto: bool,
    generated: bool,
    description: Option<String>,
    cast: String,
}

/// What a catalog read returns: columns, key columns and the row estimate.
type CatalogRows = (Vec<RawColumn>, Vec<RawConstraint>, Option<i64>);

/// A key or foreign key column: kind `p`, `u` or `f`.
struct RawConstraint {
    kind: String,
    name: String,
    column: String,
    ref_schema: String,
    ref_table: String,
    ref_column: String,
    on_update: String,
    on_delete: String,
}

// ── Talking to a connection ──

/// One saved connection and its secret.
pub struct Online<'a> {
    pools: &'a OnlinePools,
    s: &'a ConnectionSettings,
    secret: &'a str,
}

impl<'a> Online<'a> {
    pub fn new(pools: &'a OnlinePools, s: &'a ConnectionSettings, secret: &'a str) -> Self {
        Online { pools, s, secret }
    }

    fn engine(&self) -> Engine {
        self.s.engine
    }

    fn key(&self) -> String {
        pool_key(self.s, self.secret)
    }

    /// The name an object is listed and addressed by.
    pub fn display(&self, o: &OnlineObject) -> String {
        let default = match self.engine() {
            Engine::Postgres => o.schema == "public",
            Engine::Mysql => !self.s.database.is_empty() && o.schema == self.s.database,
            Engine::Libsql => true,
        };
        if o.schema.is_empty() || default { o.name.clone() } else { format!("{}.{}", o.schema, o.name) }
    }

    /// Tables and views; `fresh` reads them again and forgets descriptions.
    async fn objects(&self, fresh: bool) -> AppResult<Arc<Vec<OnlineObject>>> {
        if !fresh {
            let cached = lock(&self.pools.catalog)?.get(&self.key()).and_then(|c| c.objects.clone());
            if let Some((at, list)) = cached
                && at.elapsed() < CATALOG_TTL
            {
                return Ok(list);
            }
        }
        let list = Arc::new(online_data::list_objects(self.pools, self.s, self.secret).await?);
        let mut catalog = lock(&self.pools.catalog)?;
        let entry = catalog.entry(self.key()).or_default();
        entry.objects = Some((Instant::now(), list.clone()));
        if fresh {
            entry.tables.clear();
        }
        Ok(list)
    }

    async fn resolve(&self, name: &str) -> AppResult<OnlineObject> {
        let objects = self.objects(false).await?;
        let found = objects
            .iter()
            .find(|o| self.display(o) == name)
            .or_else(|| objects.iter().find(|o| self.display(o).eq_ignore_ascii_case(name)));
        match found {
            Some(o) => Ok(o.clone()),
            None => Err(AppError::NotFound(format!("table {name:?}"))),
        }
    }

    /// The columns and keys of the table or view `name`; cached briefly.
    pub async fn describe(&self, name: &str) -> AppResult<Arc<OnlineTable>> {
        let cached = lock(&self.pools.catalog)?.get(&self.key()).and_then(|c| c.tables.get(name).cloned());
        if let Some((at, t)) = cached
            && at.elapsed() < CATALOG_TTL
        {
            return Ok(t);
        }
        let object = self.resolve(name).await?;
        let table = Arc::new(self.describe_object(&object).await?);
        lock(&self.pools.catalog)?
            .entry(self.key())
            .or_default()
            .tables
            .insert(name.to_string(), (Instant::now(), table.clone()));
        Ok(table)
    }

    async fn describe_object(&self, o: &OnlineObject) -> AppResult<OnlineTable> {
        let (raw, constraints, rows) = within(&self.s.host, async {
            match self.engine() {
                Engine::Postgres => self.pg_catalog(o).await,
                Engine::Mysql => self.mysql_catalog(o).await,
                Engine::Libsql => self.libsql_catalog(o).await,
            }
        })
        .await?;
        if raw.is_empty() {
            return Err(AppError::NotFound(format!("columns of {:?} (or permission to read them)", self.display(o))));
        }
        let primary_key: Vec<String> = constraints.iter().filter(|c| c.kind == "p").map(|c| c.column.clone()).collect();

        // Constraints come as one row per column; group them by name.
        let mut groups: Vec<(&str, &str, Vec<&RawConstraint>)> = Vec::new();
        for c in constraints.iter().filter(|c| c.kind != "p") {
            match groups.iter_mut().find(|(k, n, _)| *k == c.kind && *n == c.name) {
                Some(g) => g.2.push(c),
                None => groups.push((&c.kind, &c.name, vec![c])),
            }
        }
        let unique_single = |col: &str| groups.iter().any(|(k, _, cs)| *k == "u" && cs.len() == 1 && cs[0].column == col);
        let unique_constraints: Vec<Vec<String>> = groups
            .iter()
            .filter(|(k, _, cs)| *k == "u" && cs.len() > 1)
            .map(|(_, _, cs)| cs.iter().map(|c| c.column.clone()).collect())
            .collect();
        let foreign_keys: Vec<ForeignKeyDef> = groups
            .iter()
            .filter(|(k, _, _)| *k == "f")
            .map(|(_, _, cs)| ForeignKeyDef {
                columns: cs.iter().map(|c| c.column.clone()).collect(),
                ref_table: self.display(&OnlineObject {
                    schema: cs[0].ref_schema.clone(),
                    name: cs[0].ref_table.clone(),
                    kind: OnlineObjectKind::Table,
                }),
                ref_columns: if cs.iter().all(|c| c.ref_column.is_empty()) {
                    Vec::new()
                } else {
                    cs.iter().map(|c| c.ref_column.clone()).collect()
                },
                on_update: fk_action(&cs[0].on_update),
                on_delete: fk_action(&cs[0].on_delete),
            })
            .collect();

        let engine = self.engine();
        let columns = raw
            .into_iter()
            .map(|r| {
                let pk = primary_key.contains(&r.name);
                // SQLite: a lone INTEGER PRIMARY KEY is the rowid, numbered automatically.
                let auto = r.auto
                    || (engine == Engine::Libsql && pk && primary_key.len() == 1 && r.full_type.eq_ignore_ascii_case("integer"));
                let mut ft = field_type(engine, &r.type_name, &r.full_type, r.max_length);
                if auto && matches!(ft, FieldType::Number { real: false } | FieldType::LargeNumber) {
                    ft = FieldType::AutoNumber;
                } else if r.generated {
                    ft = FieldType::Calculated { expression: r.default.clone().unwrap_or_default(), stored: true, result: CalculatedResult::Any };
                } else if foreign_keys.iter().any(|k| k.columns.len() == 1 && k.columns[0] == r.name) {
                    let value_type = match ft {
                        FieldType::Number { real: false } | FieldType::LargeNumber => LookupValueType::Integer,
                        _ => LookupValueType::Text,
                    };
                    ft = FieldType::Lookup { value_type, display_column: None };
                }
                OnlineColumn {
                    searchable: !is_binary(engine, &r.type_name),
                    unique: unique_single(&r.name),
                    primary_key: pk,
                    field_type: ft,
                    server_type: r.full_type,
                    nullable: r.nullable,
                    default: if r.generated { None } else { r.default },
                    auto,
                    generated: r.generated,
                    description: r.description,
                    udt: r.type_name.to_ascii_lowercase(),
                    cast: r.cast,
                    name: r.name,
                }
            })
            .collect::<Vec<_>>();
        let editable = o.kind == OnlineObjectKind::Table && !primary_key.is_empty();
        Ok(OnlineTable {
            name: self.display(o),
            schema: o.schema.clone(),
            kind: o.kind.clone(),
            columns,
            primary_key,
            foreign_keys,
            unique_constraints,
            editable,
            row_estimate: rows,
            object: Some(o.clone()),
        })
    }

    async fn pg_catalog(&self, o: &OnlineObject) -> AppResult<CatalogRows> {
        let pool = self.pools.pg(self.s, self.secret)?;
        type Col = (String, String, String, String, bool, Option<String>, Option<i64>, bool, bool);
        let cols: Vec<Col> = sqlx::query_as(
            "SELECT column_name::text, data_type::text, udt_schema::text, udt_name::text, \
             (is_nullable::text = 'YES'), column_default::text, character_maximum_length::int8, \
             (COALESCE(is_identity::text, 'NO') = 'YES'), (COALESCE(is_generated::text, 'NEVER') <> 'NEVER') \
             FROM information_schema.columns WHERE table_schema = $1 AND table_name = $2 ORDER BY ordinal_position",
        )
        .bind(&o.schema)
        .bind(&o.name)
        .fetch_all(&pool)
        .await
        .map_err(sqlx_error)?;
        type Con = (String, String, String, String, String, String, String, String);
        let cons: Vec<Con> = sqlx::query_as(
            "SELECT con.contype::text, con.conname::text, a.attname::text, COALESCE(fn.nspname::text, ''), \
             COALESCE(fc.relname::text, ''), COALESCE(fa.attname::text, ''), con.confupdtype::text, con.confdeltype::text \
             FROM pg_catalog.pg_constraint con \
             JOIN pg_catalog.pg_class c ON c.oid = con.conrelid \
             JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
             CROSS JOIN LATERAL generate_subscripts(con.conkey, 1) AS k(i) \
             JOIN pg_catalog.pg_attribute a ON a.attrelid = con.conrelid AND a.attnum = con.conkey[k.i] \
             LEFT JOIN pg_catalog.pg_class fc ON fc.oid = con.confrelid \
             LEFT JOIN pg_catalog.pg_namespace fn ON fn.oid = fc.relnamespace \
             LEFT JOIN pg_catalog.pg_attribute fa ON fa.attrelid = con.confrelid AND fa.attnum = con.confkey[k.i] \
             WHERE n.nspname = $1 AND c.relname = $2 AND con.contype IN ('p', 'u', 'f') \
             ORDER BY con.conname, k.i",
        )
        .bind(&o.schema)
        .bind(&o.name)
        .fetch_all(&pool)
        .await
        .map_err(sqlx_error)?;
        let rows: Option<(i64,)> = sqlx::query_as(
            "SELECT c.reltuples::int8 FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
             WHERE n.nspname = $1 AND c.relname = $2 AND c.relkind IN ('r', 'p', 'f')",
        )
        .bind(&o.schema)
        .bind(&o.name)
        .fetch_optional(&pool)
        .await
        .map_err(sqlx_error)?;
        let cols = cols
            .into_iter()
            .map(|(name, data_type, udt_schema, udt, nullable, default, max_length, identity, generated)| {
                let full_type = match data_type.as_str() {
                    "USER-DEFINED" => udt.clone(),
                    "ARRAY" => format!("{}[]", udt.trim_start_matches('_')),
                    _ => match max_length {
                        Some(n) => format!("{data_type}({n})"),
                        None => data_type,
                    },
                };
                let auto = identity || default.as_deref().is_some_and(|d| d.starts_with("nextval(") || d == "unique_rowid()");
                RawColumn {
                    cast: format!("{}.{}", quote(Engine::Postgres, &udt_schema).unwrap_or_default(), quote(Engine::Postgres, &udt).unwrap_or_default()),
                    type_name: udt,
                    full_type,
                    nullable,
                    default,
                    max_length,
                    auto,
                    generated,
                    description: None,
                    name,
                }
            })
            .collect();
        let cons = cons
            .into_iter()
            .map(|(kind, name, column, ref_schema, ref_table, ref_column, on_update, on_delete)| RawConstraint {
                kind,
                name,
                column,
                ref_schema,
                ref_table,
                ref_column,
                on_update: on_update.trim().to_string(),
                on_delete: on_delete.trim().to_string(),
            })
            .collect();
        Ok((cols, cons, rows.map(|(n,)| n).filter(|n| *n >= 0)))
    }

    fn mysql_catalog<'b>(&'b self, o: &'b OnlineObject) -> BoxFuture<'b, AppResult<CatalogRows>> {
        Box::pin(async move {
            let pool = self.pools.mysql(self.s, self.secret)?;
            let text = |sql: &'static str| {
                let pool = pool.clone();
                let (schema, name) = (o.schema.clone(), o.name.clone());
                async move {
                    let rows = sqlx::query(sql).bind(schema).bind(name).fetch_all(&pool).await.map_err(sqlx_error)?;
                    mysql_texts(&rows).map_err(sqlx_error)
                }
            };
            let cols = text(
                "SELECT CAST(COLUMN_NAME AS CHAR), CAST(DATA_TYPE AS CHAR), CAST(COLUMN_TYPE AS CHAR), CAST(IS_NULLABLE AS CHAR), \
                 CAST(COLUMN_DEFAULT AS CHAR), CAST(EXTRA AS CHAR), CAST(CHARACTER_MAXIMUM_LENGTH AS CHAR), CAST(COLUMN_COMMENT AS CHAR) \
                 FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? ORDER BY ORDINAL_POSITION",
            )
            .await?;
            let cons = text(
                "SELECT CAST(CASE c.CONSTRAINT_TYPE WHEN 'PRIMARY KEY' THEN 'p' WHEN 'UNIQUE' THEN 'u' ELSE 'f' END AS CHAR), \
                 CAST(k.CONSTRAINT_NAME AS CHAR), CAST(k.COLUMN_NAME AS CHAR), CAST(k.REFERENCED_TABLE_SCHEMA AS CHAR), \
                 CAST(k.REFERENCED_TABLE_NAME AS CHAR), CAST(k.REFERENCED_COLUMN_NAME AS CHAR), CAST(r.UPDATE_RULE AS CHAR), \
                 CAST(r.DELETE_RULE AS CHAR) \
                 FROM information_schema.KEY_COLUMN_USAGE k \
                 JOIN information_schema.TABLE_CONSTRAINTS c ON c.CONSTRAINT_SCHEMA = k.CONSTRAINT_SCHEMA \
                 AND c.CONSTRAINT_NAME = k.CONSTRAINT_NAME AND c.TABLE_NAME = k.TABLE_NAME \
                 LEFT JOIN information_schema.REFERENTIAL_CONSTRAINTS r ON r.CONSTRAINT_SCHEMA = k.CONSTRAINT_SCHEMA \
                 AND r.CONSTRAINT_NAME = k.CONSTRAINT_NAME \
                 WHERE k.TABLE_SCHEMA = ? AND k.TABLE_NAME = ? AND c.CONSTRAINT_TYPE IN ('PRIMARY KEY', 'UNIQUE', 'FOREIGN KEY') \
                 ORDER BY k.CONSTRAINT_NAME, k.ORDINAL_POSITION",
            )
            .await?;
            let rows = text(
                "SELECT CAST(TABLE_ROWS AS CHAR) FROM information_schema.TABLES WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? \
                 AND TABLE_TYPE = 'BASE TABLE'",
            )
            .await?;
            let get = |r: &Vec<Option<String>>, i: usize| r.get(i).cloned().flatten().unwrap_or_default();
            let cols = cols
                .iter()
                .map(|r| {
                    let extra = get(r, 5).to_ascii_lowercase();
                    let comment = get(r, 7);
                    RawColumn {
                        name: get(r, 0),
                        type_name: get(r, 1),
                        full_type: get(r, 2),
                        nullable: get(r, 3) == "YES",
                        default: r.get(4).cloned().flatten(),
                        max_length: get(r, 6).parse().ok(),
                        auto: extra.contains("auto_increment"),
                        generated: extra.contains("generated") && !extra.contains("default_generated"),
                        description: (!comment.is_empty()).then_some(comment),
                        cast: String::new(),
                    }
                })
                .collect();
            let cons = cons
                .iter()
                .map(|r| RawConstraint {
                    kind: get(r, 0),
                    name: get(r, 1),
                    column: get(r, 2),
                    ref_schema: get(r, 3),
                    ref_table: get(r, 4),
                    ref_column: get(r, 5),
                    on_update: get(r, 6),
                    on_delete: get(r, 7),
                })
                .collect();
            let rows = rows.first().map(|r| get(r, 0)).and_then(|n| n.parse().ok());
            Ok((cols, cons, rows))
        })
    }

    async fn libsql_catalog(&self, o: &OnlineObject) -> AppResult<CatalogRows> {
        let name = json!(o.name);
        let cols = libsql_execute(
            self.s,
            self.secret,
            "SELECT name, type, \"notnull\", dflt_value, pk, hidden FROM pragma_table_xinfo(?) ORDER BY cid",
            std::slice::from_ref(&name),
        )
        .await?;
        let fks = libsql_execute(
            self.s,
            self.secret,
            "SELECT id, \"table\", \"from\", \"to\", on_update, on_delete FROM pragma_foreign_key_list(?) ORDER BY id, seq",
            std::slice::from_ref(&name),
        )
        .await?;
        let text = |v: Option<&JsonValue>| v.and_then(JsonValue::as_str).unwrap_or_default().to_string();
        let int = |v: Option<&JsonValue>| v.and_then(JsonValue::as_i64).unwrap_or(0);
        let mut pk: Vec<(i64, String)> = Vec::new();
        let mut columns = Vec::new();
        for r in &cols.rows {
            let hidden = int(r.get(5));
            if hidden == 1 {
                continue;
            }
            let name = text(r.first());
            if int(r.get(4)) > 0 {
                pk.push((int(r.get(4)), name.clone()));
            }
            let declared = text(r.get(1));
            columns.push(RawColumn {
                type_name: declared.clone(),
                full_type: declared,
                nullable: int(r.get(2)) == 0,
                default: r.get(3).and_then(JsonValue::as_str).map(String::from),
                max_length: None,
                auto: false,
                generated: hidden >= 2,
                description: None,
                cast: String::new(),
                name,
            });
        }
        pk.sort();
        let mut cons: Vec<RawConstraint> = pk
            .into_iter()
            .map(|(_, column)| RawConstraint {
                kind: "p".into(),
                name: "pk".into(),
                column,
                ref_schema: String::new(),
                ref_table: String::new(),
                ref_column: String::new(),
                on_update: String::new(),
                on_delete: String::new(),
            })
            .collect();
        for r in &fks.rows {
            cons.push(RawConstraint {
                kind: "f".into(),
                name: format!("fk{}", int(r.first())),
                ref_table: text(r.get(1)),
                column: text(r.get(2)),
                ref_column: text(r.get(3)),
                on_update: text(r.get(4)),
                on_delete: text(r.get(5)),
                ref_schema: String::new(),
            });
        }
        let rows = if o.kind == OnlineObjectKind::Table {
            let count = libsql_execute(self.s, self.secret, &format!("SELECT count(*) FROM {}", quote(Engine::Libsql, &o.name)?), &[]).await?;
            count.rows.first().and_then(|r| r.first()).and_then(JsonValue::as_i64)
        } else {
            None
        };
        Ok((columns, cons, rows))
    }

    /// Runs a read in a read-only transaction; returns column names and rows.
    async fn read(&self, sql: &Sql) -> AppResult<(Vec<String>, Vec<String>, Vec<Vec<JsonValue>>)> {
        within(&self.s.host, async {
            match self.engine() {
                Engine::Postgres => {
                    let pool = self.pools.pg(self.s, self.secret)?;
                    let mut tx = pg_begin(&pool).await?;
                    pg_read(&mut tx, &sql.text, None, None, &sql.params).await
                }
                Engine::Mysql => {
                    let pool = self.pools.mysql(self.s, self.secret)?;
                    let mut conn = mysql_begin(&pool).await?;
                    let result = mysql_read(&mut conn, &sql.text, None, &sql.params).await;
                    mysql_end(conn).await;
                    result.map(|(c, t, r, _)| (c, t, r))
                }
                Engine::Libsql => {
                    let args: Vec<JsonValue> = sql.params.iter().map(Param::json).collect();
                    let r = libsql_execute(self.s, self.secret, &sql.text, &args).await?;
                    Ok((r.columns, r.types, r.rows))
                }
            }
        })
        .await
    }

    /// Runs `writes` in one transaction: all of them or none.
    async fn write(&self, table: &OnlineTable, writes: Vec<Write>) -> AppResult<Vec<Written>> {
        let ms = QUERY_TIMEOUT.as_millis();
        within(&self.s.host, async {
            match self.engine() {
                Engine::Postgres => {
                    let pool = self.pools.pg(self.s, self.secret)?;
                    let mut tx = pool.begin().await.map_err(sqlx_error)?;
                    sqlx::query(&format!("SET LOCAL statement_timeout = {ms}")).execute(&mut *tx).await.map_err(sqlx_error)?;
                    let mut out = Vec::new();
                    for w in &writes {
                        let q = w.sql.params.iter().fold(sqlx::query(&w.sql.text), pg_bind);
                        if w.returning.is_empty() {
                            let r = q.execute(&mut *tx).await.map_err(sqlx_error)?;
                            out.push(Written { affected: r.rows_affected(), ..Default::default() });
                        } else {
                            use sqlx::Row;
                            let rows = q.fetch_all(&mut *tx).await.map_err(sqlx_error)?;
                            let row = match rows.first() {
                                Some(r) => Some(
                                    w.returning
                                        .iter()
                                        .enumerate()
                                        .map(|(i, &c)| r.try_get::<Option<String>, _>(i).map(|t| pg_value(&table.columns[c].udt, t)))
                                        .collect::<Result<Vec<_>, _>>()
                                        .map_err(sqlx_error)?,
                                ),
                                None => None,
                            };
                            out.push(Written { row, affected: rows.len() as u64, last_id: None });
                        }
                    }
                    tx.commit().await.map_err(sqlx_error)?;
                    Ok(out)
                }
                Engine::Mysql => mysql_write(self.pools.mysql(self.s, self.secret)?, writes).await,
                Engine::Libsql => {
                    let n = writes.len();
                    let mut steps = vec![json!({ "stmt": { "sql": "BEGIN" } })];
                    for (i, w) in writes.iter().enumerate() {
                        let args: Vec<JsonValue> = w.sql.params.iter().map(Param::json).collect();
                        steps.push(json!({ "condition": { "type": "ok", "step": i }, "stmt": hrana_stmt(&w.sql.text, &args) }));
                    }
                    steps.push(json!({ "condition": { "type": "ok", "step": n }, "stmt": { "sql": "COMMIT" } }));
                    steps.push(json!({
                        "condition": { "type": "not", "cond": { "type": "ok", "step": n + 1 } },
                        "stmt": { "sql": "ROLLBACK" }
                    }));
                    let response =
                        libsql_request(self.s, self.secret, json!({ "type": "batch", "batch": { "steps": steps } })).await?;
                    let result = &response["result"];
                    for i in 1..=n + 1 {
                        let e = &result["step_errors"][i];
                        if !e.is_null() {
                            return Err(AppError::Validation(e["message"].as_str().unwrap_or("the change failed").to_string()));
                        }
                    }
                    Ok((1..=n)
                        .map(|i| {
                            let step = &result["step_results"][i];
                            let r = libsql_result(step);
                            Written {
                                row: r.rows.into_iter().next(),
                                affected: step["affected_row_count"].as_u64().unwrap_or(0),
                                last_id: step["last_insert_rowid"].as_str().and_then(|s| s.parse().ok()),
                            }
                        })
                        .collect())
                }
            }
        })
        .await
    }
}

/// Reads every column of MySQL rows as optional text.
fn mysql_texts(rows: &[sqlx::mysql::MySqlRow]) -> Result<Vec<Vec<Option<String>>>, sqlx::Error> {
    use sqlx::{Row, ValueRef};
    rows.iter()
        .map(|r| {
            (0..r.len())
                .map(|i| {
                    if r.try_get_raw(i)?.is_null() {
                        return Ok(None);
                    }
                    Ok(Some(String::from_utf8_lossy(&r.try_get_unchecked::<Vec<u8>, _>(i)?).into_owned()))
                })
                .collect()
        })
        .collect()
}

// Boxed with an explicit `Send` for the same reason as `mysql_begin`.
fn mysql_write(pool: sqlx::MySqlPool, writes: Vec<Write>) -> BoxFuture<'static, AppResult<Vec<Written>>> {
    Box::pin(async move {
        let mut tx = pool.begin().await.map_err(sqlx_error)?;
        let mut out = Vec::new();
        for w in &writes {
            let r = w.sql.params.iter().fold(sqlx::query(&w.sql.text), mysql_bind).execute(&mut *tx).await.map_err(sqlx_error)?;
            let id = r.last_insert_id();
            out.push(Written { row: None, affected: r.rows_affected(), last_id: (id != 0).then_some(id as i64) });
        }
        tx.commit().await.map_err(sqlx_error)?;
        Ok(out)
    })
}

// ── Building statements ──

/// Appends ` WHERE …` for filters and search.
fn push_where(sql: &mut Sql, t: &OnlineTable, filters: &[FilterSpec], search: Option<&str>) -> AppResult<()> {
    let engine = sql.engine;
    let text_of = |qc: &str| match engine {
        Engine::Postgres => format!("CAST({qc} AS text)"),
        Engine::Mysql => format!("LOWER(CAST({qc} AS CHAR))"),
        Engine::Libsql => format!("lower(CAST({qc} AS TEXT))"),
    };
    let like = match engine {
        Engine::Postgres => " ILIKE ",
        Engine::Mysql => " LIKE ",
        Engine::Libsql => " LIKE ",
    };
    let escape = if engine == Engine::Libsql { " ESCAPE '\\'" } else { "" };
    let mut first = true;
    let mut and = |sql: &mut Sql| {
        sql.push(if first { " WHERE " } else { " AND " });
        first = false;
    };
    for f in filters {
        let col = t.column(&f.column)?;
        let qc = quote(engine, &col.name)?;
        match f.op {
            FilterOp::IsNull => {
                and(sql);
                sql.push(&format!("{qc} IS NULL"));
            }
            FilterOp::IsNotNull => {
                and(sql);
                sql.push(&format!("{qc} IS NOT NULL"));
            }
            FilterOp::Contains | FilterOp::StartsWith | FilterOp::EndsWith => {
                let s = f
                    .value
                    .as_str()
                    .ok_or_else(|| AppError::Validation(format!("filter on {:?} needs a text value", col.name)))?;
                let pattern = match f.op {
                    FilterOp::Contains => like_pattern(s, true, true),
                    FilterOp::StartsWith => like_pattern(s, false, true),
                    _ => like_pattern(s, true, false),
                };
                and(sql);
                sql.push(&text_of(&qc)).push(like).bind(Param::Text(pattern), None).push(escape);
            }
            FilterOp::In => {
                let items = f
                    .value
                    .as_array()
                    .ok_or_else(|| AppError::Validation(format!("filter on {:?} needs a list of values", col.name)))?;
                if items.len() > MAX_IN_LIST {
                    return Err(AppError::Validation(format!("at most {MAX_IN_LIST} values per list")));
                }
                and(sql);
                if items.is_empty() {
                    sql.push("1 = 0");
                    continue;
                }
                sql.push(&format!("{qc} IN ("));
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        sql.push(", ");
                    }
                    sql.value(col, param(item)?);
                }
                sql.push(")");
            }
            op => {
                let v = param(&f.value)?;
                and(sql);
                if v == Param::Null {
                    match op {
                        FilterOp::Eq => sql.push(&format!("{qc} IS NULL")),
                        FilterOp::Ne => sql.push(&format!("{qc} IS NOT NULL")),
                        _ => return Err(AppError::Validation(format!("cannot compare {:?} with null", col.name))),
                    };
                    continue;
                }
                if op == FilterOp::Ne {
                    // Rows with no value count as different, as in the Datasheet.
                    match engine {
                        Engine::Postgres => sql.push(&format!("{qc} IS DISTINCT FROM ")).value(col, v),
                        Engine::Mysql => sql.push(&format!("NOT ({qc} <=> ")).value(col, v).push(")"),
                        Engine::Libsql => sql.push(&format!("{qc} IS NOT ")).value(col, v),
                    };
                    continue;
                }
                let sym = match op {
                    FilterOp::Eq => "=",
                    FilterOp::Lt => "<",
                    FilterOp::Le => "<=",
                    FilterOp::Gt => ">",
                    _ => ">=",
                };
                sql.push(&format!("{qc} {sym} ")).value(col, v);
            }
        }
    }
    if let Some(term) = search.map(str::trim).filter(|s| !s.is_empty()) {
        let cols: Vec<&OnlineColumn> = t.columns.iter().filter(|c| c.searchable).collect();
        if !cols.is_empty() {
            let pattern = like_pattern(term, true, true);
            and(sql);
            sql.push("(");
            for (i, c) in cols.iter().enumerate() {
                if i > 0 {
                    sql.push(" OR ");
                }
                sql.push(&text_of(&quote(engine, &c.name)?)).push(like).bind(Param::Text(pattern.clone()), None).push(escape);
            }
            sql.push(")");
        }
    }
    Ok(())
}

/// Appends `<pk> = … AND …` for `key`.
fn push_key(sql: &mut Sql, t: &OnlineTable, key: &RecordKey) -> AppResult<()> {
    t.require_editable()?;
    let RecordKey::PrimaryKey(map) = key else {
        return Err(AppError::Validation("record key does not match the table's key type".into()));
    };
    for (i, name) in t.primary_key.iter().enumerate() {
        let v = map
            .iter()
            .find(|(k, _)| *k == name)
            .or_else(|| map.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)))
            .map(|(_, v)| v)
            .ok_or_else(|| AppError::Validation(format!("record key is missing {name:?}")))?;
        let col = t.column(name)?;
        let qc = quote(sql.engine, name)?;
        sql.push(if i == 0 { "" } else { " AND " }).push(&format!("{qc} = ")).value(col, param(v)?);
    }
    Ok(())
}

fn select_list(engine: Engine, t: &OnlineTable) -> AppResult<String> {
    Ok(t.columns.iter().map(|c| quote(engine, &c.name)).collect::<AppResult<Vec<_>>>()?.join(", "))
}

fn row_key(t: &OnlineTable, values: &[JsonValue]) -> Option<RecordKey> {
    if !t.editable {
        return None;
    }
    let mut map = Map::new();
    for k in &t.primary_key {
        let i = t.columns.iter().position(|c| &c.name == k)?;
        map.insert(k.clone(), values.get(i)?.clone());
    }
    Some(RecordKey::PrimaryKey(map))
}

/// ` RETURNING <pk>` where the server supports it, and the columns read back.
fn returning(engine: Engine, t: &OnlineTable) -> AppResult<(String, Vec<usize>)> {
    if engine == Engine::Mysql {
        return Ok((String::new(), Vec::new()));
    }
    let idx: Vec<usize> = t.primary_key.iter().filter_map(|k| t.columns.iter().position(|c| &c.name == k)).collect();
    let cols = idx
        .iter()
        .map(|&i| {
            let q = quote(engine, &t.columns[i].name)?;
            Ok(if engine == Engine::Postgres { format!("CAST({q} AS text)") } else { q })
        })
        .collect::<AppResult<Vec<_>>>()?;
    Ok((format!(" RETURNING {}", cols.join(", ")), idx))
}

/// The columns and values to write, checked against the table.
fn assignments<'t>(t: &'t OnlineTable, values: &RecordValues, inserting: bool) -> AppResult<Vec<(&'t OnlineColumn, Param)>> {
    let mut out = Vec::new();
    for (k, v) in values {
        let col = t.column(k)?;
        if col.generated {
            return Err(AppError::Validation(format!("{:?} is calculated by the server", col.name)));
        }
        if inserting && col.auto && v.is_null() {
            continue;
        }
        out.push((col, param(v)?));
    }
    Ok(out)
}

fn insert_write(engine: Engine, t: &OnlineTable, values: &RecordValues) -> AppResult<Write> {
    t.require_editable()?;
    let from = qualified(engine, t.object())?;
    let cols = assignments(t, values, true)?;
    let mut sql = Sql::new(engine, format!("INSERT INTO {from} "));
    if cols.is_empty() {
        sql.push(if engine == Engine::Mysql { "() VALUES ()" } else { "DEFAULT VALUES" });
    } else {
        let names = cols.iter().map(|(c, _)| quote(engine, &c.name)).collect::<AppResult<Vec<_>>>()?;
        sql.push(&format!("({}) VALUES (", names.join(", ")));
        for (i, (c, p)) in cols.into_iter().enumerate() {
            if i > 0 {
                sql.push(", ");
            }
            sql.value(c, p);
        }
        sql.push(")");
    }
    let (ret, returning) = returning(engine, t)?;
    sql.push(&ret);
    Ok(Write { sql, returning })
}

fn update_write(engine: Engine, t: &OnlineTable, key: &RecordKey, values: &RecordValues) -> AppResult<Write> {
    t.require_editable()?;
    let from = qualified(engine, t.object())?;
    let mut sql = Sql::new(engine, format!("UPDATE {from} SET "));
    for (i, (c, p)) in assignments(t, values, false)?.into_iter().enumerate() {
        if i > 0 {
            sql.push(", ");
        }
        sql.push(&format!("{} = ", quote(engine, &c.name)?)).value(c, p);
    }
    sql.push(" WHERE ");
    push_key(&mut sql, t, key)?;
    let (ret, returning) = returning(engine, t)?;
    sql.push(&ret);
    Ok(Write { sql, returning })
}

fn delete_write(engine: Engine, t: &OnlineTable, key: &RecordKey) -> AppResult<Write> {
    let from = qualified(engine, t.object())?;
    let mut sql = Sql::new(engine, format!("DELETE FROM {from} WHERE "));
    push_key(&mut sql, t, key)?;
    Ok(Write { sql, returning: Vec::new() })
}

/// The key of a row just inserted or updated: read back, or (MySQL) from
/// the values written and the generated ID.
fn written_key(t: &OnlineTable, w: &Written, values: &RecordValues, old: Option<&RecordKey>) -> AppResult<RecordKey> {
    let mut map = Map::new();
    for (i, k) in t.primary_key.iter().enumerate() {
        let v = if let Some(row) = &w.row {
            row.get(i).cloned()
        } else if let Some((_, v)) = values.iter().find(|(n, _)| n.eq_ignore_ascii_case(k)).filter(|(_, v)| !v.is_null()) {
            Some(v.clone())
        } else if let Some(RecordKey::PrimaryKey(m)) = old {
            m.iter().find(|(n, _)| n.eq_ignore_ascii_case(k)).map(|(_, v)| v.clone())
        } else if t.column(k)?.auto {
            w.last_id.map(JsonValue::from)
        } else {
            None
        };
        map.insert(k.clone(), v.ok_or_else(|| AppError::Internal(format!("the saved record's {k:?} could not be read")))?);
    }
    Ok(RecordKey::PrimaryKey(map))
}

// ── Commands ──

pub async fn list_objects(o: &Online<'_>) -> AppResult<Vec<ObjectSummary>> {
    let objects = o.objects(true).await?;
    Ok(objects
        .iter()
        .map(|x| ObjectSummary {
            name: o.display(x),
            kind: match x.kind {
                OnlineObjectKind::Table => ObjectKind::Table,
                OnlineObjectKind::View => ObjectKind::View,
            },
            managed: false,
        })
        .collect())
}

/// A table's definition in the shape of a workspace table's: relations
/// become lookup fields shown by the linked table's first text column.
pub async fn table_info(o: &Online<'_>, name: &str) -> AppResult<TableInfo> {
    let t = o.describe(name).await?;
    // Links that name no columns point at the linked table's primary key.
    let mut foreign_keys = t.foreign_keys.clone();
    for fk in foreign_keys.iter_mut().filter(|k| k.ref_columns.is_empty()) {
        if let Ok(parent) = o.describe(&fk.ref_table).await {
            fk.ref_columns = parent.primary_key.clone();
        }
    }
    let mut fields = Vec::new();
    for c in &t.columns {
        let mut ft = c.field_type.clone();
        if let FieldType::Lookup { display_column, .. } = &mut ft
            && let Some(fk) = foreign_keys.iter().find(|k| k.columns.len() == 1 && k.columns[0] == c.name)
            && let Ok(parent) = o.describe(&fk.ref_table).await
        {
            *display_column = parent
                .columns
                .iter()
                .find(|p| !p.primary_key && matches!(p.field_type, FieldType::ShortText { .. } | FieldType::LongText))
                .map(|p| p.name.clone());
        }
        fields.push(FieldDef {
            name: c.name.clone(),
            field_type: ft,
            required: !c.nullable && c.default.is_none() && !c.auto && !c.generated,
            unique: c.unique,
            primary_key: c.primary_key,
            default: None,
            check: None,
            description: c.description.clone(),
        });
    }
    Ok(TableInfo {
        definition: TableDef {
            name: t.name.clone(),
            fields,
            foreign_keys,
            unique_constraints: t.unique_constraints.clone(),
            checks: Vec::new(),
            description: None,
        },
        managed: false,
        unsupported_features: Vec::new(),
        schema_sql: String::new(),
        without_rowid: false,
        row_count: t.row_estimate.unwrap_or(0),
        indexes: Vec::new(),
    })
}

pub async fn list_records(o: &Online<'_>, table: &str, q: &RecordQuery) -> AppResult<RecordPage> {
    let t = o.describe(table).await?;
    let engine = o.engine();
    let from = qualified(engine, t.object())?;
    let search = q.search.as_deref();

    let mut count = Sql::new(engine, format!("SELECT count(*) FROM {from}"));
    push_where(&mut count, &t, &q.filters, search)?;
    let mut page = Sql::new(engine, format!("SELECT {} FROM {from}", select_list(engine, &t)?));
    push_where(&mut page, &t, &q.filters, search)?;
    let mut order = Vec::new();
    for s in &q.sort {
        let col = t.column(&s.column)?;
        order.push(format!("{} {}", quote(engine, &col.name)?, if s.descending { "DESC" } else { "ASC" }));
    }
    for k in &t.primary_key {
        order.push(quote(engine, k)?);
    }
    if !order.is_empty() {
        page.push(&format!(" ORDER BY {}", order.join(", ")));
    }
    let limit = match q.limit {
        0 => DEFAULT_PAGE_SIZE,
        n => n.min(MAX_PAGE_SIZE),
    };
    page.push(&format!(" LIMIT {limit} OFFSET {}", q.offset.min(i64::MAX as u64)));

    let (_, _, totals) = o.read(&count).await?;
    let total = totals.first().and_then(|r| r.first()).and_then(JsonValue::as_i64).unwrap_or(0);
    let (_, _, rows) = o.read(&page).await?;
    Ok(RecordPage {
        columns: t.columns.iter().map(|c| c.name.clone()).collect(),
        types: t.columns.iter().map(|c| Some(c.field_type.clone())).collect(),
        rows: rows.into_iter().map(|values| RecordRow { key: row_key(&t, &values), values }).collect(),
        total,
        offset: q.offset,
        limit,
    })
}

pub async fn aggregate(o: &Online<'_>, table: &str, q: &AggregateQuery) -> AppResult<AggregateResult> {
    let t = o.describe(table).await?;
    let engine = o.engine();
    if q.series.is_empty() && q.group_by.is_none() {
        return Err(AppError::Validation("choose at least one value to total".into()));
    }
    if q.series.len() > MAX_SERIES {
        return Err(AppError::Validation(format!("at most {MAX_SERIES} values per chart")));
    }
    let mut select = Vec::new();
    let mut columns = Vec::new();
    let mut types = Vec::new();
    if let Some(g) = &q.group_by {
        let col = t.column(&g.column)?;
        let qc = quote(engine, &col.name)?;
        let expr = match (engine, g.bucket) {
            (_, None) => qc,
            (Engine::Postgres, Some(b)) => {
                let f = match b {
                    DateBucket::Day => "YYYY-MM-DD",
                    DateBucket::Month => "YYYY-MM",
                    DateBucket::Year => "YYYY",
                };
                format!("to_char(CAST({qc} AS timestamp), '{f}')")
            }
            (Engine::Mysql, Some(b)) => {
                let f = match b {
                    DateBucket::Day => "%Y-%m-%d",
                    DateBucket::Month => "%Y-%m",
                    DateBucket::Year => "%Y",
                };
                format!("DATE_FORMAT({qc}, '{f}')")
            }
            (Engine::Libsql, Some(DateBucket::Day)) => format!("date({qc})"),
            (Engine::Libsql, Some(DateBucket::Month)) => format!("strftime('%Y-%m', {qc})"),
            (Engine::Libsql, Some(DateBucket::Year)) => format!("strftime('%Y', {qc})"),
        };
        select.push(expr);
        types.push(if g.bucket.is_some() { None } else { Some(col.field_type.clone()) });
        columns.push(col.name.clone());
    }
    for s in &q.series {
        let (expr, label, ty) = match (&s.column, s.agg) {
            (None, Aggregate::Count) => ("count(*)".to_string(), "Count".to_string(), None),
            (None, _) => return Err(AppError::Validation("choose a field to total".into())),
            (Some(c), agg) => {
                let col = t.column(c)?;
                let qc = quote(engine, &col.name)?;
                // Sums and averages come back as plain numbers for charts.
                let number = |e: String| match engine {
                    Engine::Postgres => format!("CAST({e} AS float8)"),
                    Engine::Mysql => format!("({e} + 0e0)"),
                    Engine::Libsql => e,
                };
                let (f, name) = match agg {
                    Aggregate::Count => (format!("count({qc})"), "Count of"),
                    Aggregate::CountDistinct => (format!("count(DISTINCT {qc})"), "Distinct"),
                    Aggregate::Sum => (number(format!("sum({qc})")), "Sum of"),
                    Aggregate::Avg => (number(format!("avg({qc})")), "Average"),
                    Aggregate::Min => (format!("min({qc})"), "Min"),
                    Aggregate::Max => (format!("max({qc})"), "Max"),
                };
                let ty = matches!(agg, Aggregate::Min | Aggregate::Max).then(|| col.field_type.clone());
                (f, format!("{name} {}", col.name), ty)
            }
        };
        select.push(expr);
        columns.push(label);
        types.push(ty);
    }
    let from = qualified(engine, t.object())?;
    let mut sql = Sql::new(engine, format!("SELECT {} FROM {from}", select.join(", ")));
    push_where(&mut sql, &t, &q.filters, q.search.as_deref())?;
    if q.group_by.is_some() {
        let order = match (q.order, q.series.is_empty()) {
            (AggregateOrder::ValueAsc, false) => "2 ASC",
            (AggregateOrder::ValueDesc, false) => "2 DESC",
            _ => "1 ASC",
        };
        sql.push(&format!(" GROUP BY 1 ORDER BY {order}"));
    }
    let limit = match q.limit {
        0 => MAX_PAGE_SIZE,
        n => n.min(MAX_PAGE_SIZE),
    } as usize;
    sql.push(&format!(" LIMIT {}", limit + 1));
    let (_, _, mut rows) = o.read(&sql).await?;
    let truncated = rows.len() > limit;
    rows.truncate(limit);
    Ok(AggregateResult { columns, rows, types, truncated })
}

pub async fn get_record(o: &Online<'_>, table: &str, key: &RecordKey) -> AppResult<RecordRow> {
    let t = o.describe(table).await?;
    let engine = o.engine();
    let mut sql = Sql::new(engine, format!("SELECT {} FROM {} WHERE ", select_list(engine, &t)?, qualified(engine, t.object())?));
    push_key(&mut sql, &t, key)?;
    sql.push(" LIMIT 1");
    let (_, _, rows) = o.read(&sql).await?;
    let values = rows.into_iter().next().ok_or_else(|| AppError::NotFound("record".into()))?;
    Ok(RecordRow { key: row_key(&t, &values), values })
}

pub async fn insert_record(o: &Online<'_>, table: &str, values: &RecordValues) -> AppResult<RecordKey> {
    let t = o.describe(table).await?;
    let w = insert_write(o.engine(), &t, values)?;
    let done = o.write(&t, vec![w]).await?;
    written_key(&t, &done[0], values, None)
}

pub async fn update_record(o: &Online<'_>, table: &str, key: &RecordKey, values: &RecordValues) -> AppResult<RecordKey> {
    let t = o.describe(table).await?;
    if values.is_empty() {
        return Ok(key.clone());
    }
    let w = update_write(o.engine(), &t, key, values)?;
    let done = o.write(&t, vec![w]).await?;
    if done[0].affected == 0 {
        return Err(AppError::NotFound("record (it may have been deleted)".into()));
    }
    written_key(&t, &done[0], values, Some(key))
}

pub async fn delete_records(o: &Online<'_>, table: &str, keys: &[RecordKey], confirm: bool) -> AppResult<usize> {
    if !confirm {
        return Err(AppError::ConfirmationRequired(format!("deleting {} record(s) cannot be undone", keys.len())));
    }
    let t = o.describe(table).await?;
    let writes = keys.iter().map(|k| delete_write(o.engine(), &t, k)).collect::<AppResult<Vec<_>>>()?;
    let done = o.write(&t, writes).await?;
    Ok(done.iter().map(|w| w.affected as usize).sum())
}

pub async fn batch(o: &Online<'_>, table: &str, ops: &[BatchOp], confirm_deletes: bool) -> AppResult<BatchResult> {
    if ops.len() > MAX_BATCH_OPS {
        return Err(AppError::Validation(format!("at most {MAX_BATCH_OPS} operations per batch")));
    }
    let deletes = ops.iter().filter(|o| matches!(o, BatchOp::Delete { .. })).count();
    if deletes > 0 && !confirm_deletes {
        return Err(AppError::ConfirmationRequired(format!("the batch deletes {deletes} record(s), which cannot be undone")));
    }
    let t = o.describe(table).await?;
    let engine = o.engine();
    let writes = ops
        .iter()
        .map(|op| match op {
            BatchOp::Insert { values } => insert_write(engine, &t, values),
            BatchOp::Update { key, values } => update_write(engine, &t, key, values),
            BatchOp::Delete { key } => delete_write(engine, &t, key),
        })
        .collect::<AppResult<Vec<_>>>()?;
    let done = o.write(&t, writes).await?;
    let mut result = BatchResult { inserted: Vec::new(), updated: 0, deleted: 0 };
    for (op, w) in ops.iter().zip(&done) {
        match op {
            BatchOp::Insert { values } => result.inserted.push(written_key(&t, w, values, None)?),
            BatchOp::Update { .. } => result.updated += 1,
            BatchOp::Delete { .. } => result.deleted += 1,
        }
    }
    Ok(result)
}

/// A read-only query, with `:name` parameters bound from `params`.
pub async fn run_query(o: &Online<'_>, sql: &str, params: &Map<String, JsonValue>) -> AppResult<QueryResult> {
    let r = online_data::query(o.pools, o.s, o.secret, sql, params).await?;
    Ok(QueryResult {
        types: result_types(o.engine(), &r.types),
        columns: r.columns,
        rows: r.rows,
        truncated: r.truncated,
        elapsed_ms: r.elapsed_ms,
    })
}

/// Online views open as a query reading them.
pub async fn saved_query(o: &Online<'_>, name: &str) -> AppResult<SavedQuery> {
    let object = o.resolve(name).await?;
    Ok(SavedQuery { sql: format!("SELECT * FROM {}", qualified(o.engine(), &object)?), design: None })
}

pub async fn info(o: &Online<'_>) -> AppResult<OnlineInfo> {
    let engine = o.engine();
    let objects = o.objects(true).await?;
    let (version_sql, size_sql) = match engine {
        Engine::Postgres => ("SELECT version()", "SELECT pg_database_size(current_database())::int8"),
        Engine::Mysql => (
            "SELECT VERSION()",
            "SELECT CAST(SUM(DATA_LENGTH + INDEX_LENGTH) AS SIGNED) FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE()",
        ),
        Engine::Libsql => ("SELECT sqlite_version()", "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()"),
    };
    let (_, _, v) = o.read(&Sql::new(engine, version_sql)).await?;
    let mut version = v.first().and_then(|r| r.first()).and_then(JsonValue::as_str).unwrap_or_default().to_string();
    if engine == Engine::Libsql {
        version = format!("SQLite {version}");
    }
    // Not every server lets every user see its size.
    let size = match o.read(&Sql::new(engine, size_sql)).await {
        Ok((_, _, r)) => r.first().and_then(|r| r.first()).and_then(|v| v.as_i64().or_else(|| v.as_str()?.parse().ok())),
        Err(_) => None,
    };
    let mut schemas: Vec<String> = objects.iter().map(|x| x.schema.clone()).filter(|s| !s.is_empty()).collect();
    schemas.dedup();
    schemas.sort();
    schemas.dedup();
    Ok(OnlineInfo {
        engine,
        version,
        size,
        schemas,
        tables: objects.iter().filter(|x| x.kind == OnlineObjectKind::Table).count(),
        views: objects.iter().filter(|x| x.kind == OnlineObjectKind::View).count(),
    })
}

/// How the server would run a query. `analyze` (Postgres) also runs it,
/// inside a read-only transaction, and reports real timings.
pub async fn explain(o: &Online<'_>, sql: &str, analyze: bool) -> AppResult<OnlineRows> {
    let body = normalize(sql)?.to_string();
    let engine = o.engine();
    let started = Instant::now();
    let (columns, types, rows) = match engine {
        Engine::Postgres => {
            let text = format!("EXPLAIN (ANALYZE {analyze}, FORMAT TEXT) {body}");
            within(&o.s.host, async {
                use sqlx::Row;
                let pool = o.pools.pg(o.s, o.secret)?;
                let mut tx = pg_begin(&pool).await?;
                let rows = sqlx::query(&text).fetch_all(&mut *tx).await.map_err(sqlx_error)?;
                let lines = rows
                    .iter()
                    .map(|r| r.try_get::<String, _>(0).map(|l| vec![JsonValue::String(l)]))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(sqlx_error)?;
                Ok((vec!["QUERY PLAN".to_string()], vec!["text".to_string()], lines))
            })
            .await?
        }
        Engine::Mysql => {
            if analyze {
                return Err(AppError::Unsupported("timing a query is available for PostgreSQL databases".into()));
            }
            o.read(&Sql::new(engine, format!("EXPLAIN {body}"))).await?
        }
        Engine::Libsql => {
            if analyze {
                return Err(AppError::Unsupported("timing a query is available for PostgreSQL databases".into()));
            }
            o.read(&Sql::new(engine, format!("EXPLAIN QUERY PLAN {body}"))).await?
        }
    };
    Ok(OnlineRows { columns, types, rows, truncated: false, total: None, elapsed_ms: elapsed(started) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(engine: Engine, sql: &str) -> (String, Vec<String>) {
        let mut found = Vec::new();
        let out = rewrite_params(engine, sql, |n| {
            found.push(n.to_string());
            "?".into()
        });
        (out, found)
    }

    #[test]
    fn named_parameters_outside_quotes_and_casts() {
        let (sql, found) = names(Engine::Postgres, "SELECT ':no', \"a:b\", x::int FROM t WHERE id = :id AND n > :min -- :c\n/* :d */");
        assert_eq!(found, ["id", "min"]);
        assert_eq!(sql, "SELECT ':no', \"a:b\", x::int FROM t WHERE id = ? AND n > ? -- :c\n/* :d */");
        let (_, found) = names(Engine::Postgres, "SELECT $$ :x $$, $tag$ :y $tag$, $1 FROM t WHERE a = :z");
        assert_eq!(found, ["z"]);
        let (_, found) = names(Engine::Mysql, "SELECT 'it\\'s :x', `:y` FROM t # :c\nWHERE a = :a");
        assert_eq!(found, ["a"]);
        let (_, found) = names(Engine::Libsql, "SELECT 'it''s :x' WHERE a = :a_1");
        assert_eq!(found, ["a_1"]);
    }

    #[test]
    fn server_types_to_field_types() {
        assert_eq!(field_type(Engine::Postgres, "int4", "integer", None), FieldType::Number { real: false });
        assert_eq!(field_type(Engine::Postgres, "numeric", "numeric", None), FieldType::Number { real: true });
        assert_eq!(field_type(Engine::Postgres, "varchar", "character varying(20)", Some(20)), FieldType::ShortText { max_length: Some(20) });
        assert_eq!(field_type(Engine::Postgres, "jsonb", "jsonb", None), FieldType::Unknown { declared: "jsonb".into() });
        assert_eq!(field_type(Engine::Mysql, "tinyint", "tinyint(1)", None), FieldType::YesNo);
        assert_eq!(field_type(Engine::Mysql, "int unsigned", "int unsigned", None), FieldType::Number { real: false });
        assert_eq!(field_type(Engine::Mysql, "longtext", "longtext", None), FieldType::LongText);
        assert_eq!(field_type(Engine::Libsql, "", "VARCHAR(10)", None), FieldType::ShortText { max_length: None });
        assert_eq!(field_type(Engine::Libsql, "", "DATETIME", None), FieldType::DateTime);
        assert_eq!(result_types(Engine::Postgres, &["bool".into(), "uuid".into()]), vec![Some(FieldType::YesNo), None]);
    }

    fn table(engine: Engine) -> OnlineTable {
        let col = |name: &str, ft: FieldType, cast: &str, pk: bool| OnlineColumn {
            name: name.into(),
            server_type: String::new(),
            field_type: ft,
            nullable: !pk,
            default: None,
            primary_key: pk,
            unique: false,
            auto: pk,
            generated: false,
            description: None,
            cast: cast.into(),
            udt: String::new(),
            searchable: true,
        };
        let _ = engine;
        OnlineTable {
            name: "orders".into(),
            schema: "public".into(),
            kind: OnlineObjectKind::Table,
            columns: vec![
                col("id", FieldType::AutoNumber, "\"pg_catalog\".\"int4\"", true),
                col("Name", FieldType::ShortText { max_length: None }, "\"pg_catalog\".\"text\"", false),
            ],
            primary_key: vec!["id".into()],
            foreign_keys: Vec::new(),
            unique_constraints: Vec::new(),
            editable: true,
            row_estimate: None,
            object: Some(OnlineObject { schema: "public".into(), name: "orders".into(), kind: OnlineObjectKind::Table }),
        }
    }

    #[test]
    fn filters_cast_values_in_postgres() {
        let t = table(Engine::Postgres);
        let mut sql = Sql::new(Engine::Postgres, "SELECT 1");
        let filters = vec![
            FilterSpec { column: "id".into(), op: FilterOp::Ge, value: json!(3) },
            FilterSpec { column: "name".into(), op: FilterOp::Ne, value: json!("x") },
            FilterSpec { column: "Name".into(), op: FilterOp::In, value: json!([]) },
        ];
        push_where(&mut sql, &t, &filters, Some("50%")).unwrap();
        assert_eq!(
            sql.text,
            "SELECT 1 WHERE \"id\" >= CAST($1::text AS \"pg_catalog\".\"int4\") AND \"Name\" IS DISTINCT FROM \
             CAST($2::text AS \"pg_catalog\".\"text\") AND 1 = 0 AND (CAST(\"id\" AS text) ILIKE $3 OR CAST(\"Name\" AS text) ILIKE $4)"
        );
        assert_eq!(sql.params[0], Param::Text("3".into()));
        assert_eq!(sql.params[2], Param::Text("%50\\%%".into()));
    }

    #[test]
    fn writes_by_primary_key() {
        let t = table(Engine::Mysql);
        let mut values = Map::new();
        values.insert("name".into(), json!("Ada"));
        values.insert("id".into(), JsonValue::Null);
        let w = insert_write(Engine::Mysql, &t, &values).unwrap();
        assert_eq!(w.sql.text, "INSERT INTO `public`.`orders` (`Name`) VALUES (?)");
        let key = written_key(&t, &Written { last_id: Some(7), ..Default::default() }, &values, None).unwrap();
        assert_eq!(key, RecordKey::PrimaryKey(Map::from_iter([("id".to_string(), json!(7))])));

        let w = update_write(Engine::Libsql, &t, &key, &values).unwrap();
        assert_eq!(w.sql.text, "UPDATE \"public\".\"orders\" SET \"id\" = ?, \"Name\" = ? WHERE \"id\" = ? RETURNING \"id\"");
        assert!(update_write(Engine::Libsql, &t, &RecordKey::RowId(1), &values).is_err());

        let mut view = table(Engine::Postgres);
        view.editable = false;
        assert!(delete_write(Engine::Postgres, &view, &key).is_err());
    }
}

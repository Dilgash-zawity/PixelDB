//! Record CRUD. All values are bound as parameters; identifiers come from
//! the table definition and are quoted.

use rusqlite::types::Value;
use rusqlite::{Connection, params_from_iter};
use serde_json::Value as JsonValue;

use crate::errors::{AppError, AppResult};
use crate::models::{
    BatchOp, BatchResult, FieldDef, FieldType, FilterOp, RecordKey, RecordPage, RecordQuery,
    RecordRow, RecordValues, TableDef,
};
use crate::validation::identifiers::quote_ident;
use crate::validation::values::{coerce_value, sql_to_json, untyped_value};

use super::schema::{load_table, require_object, rowid_alias, rowid_name};
use super::transactions::with_transaction;

pub const DEFAULT_PAGE_SIZE: u64 = 100;
pub const MAX_PAGE_SIZE: u64 = 1000;
pub const MAX_IN_LIST: usize = 1000;
pub const MAX_BATCH_OPS: usize = 10_000;

enum KeyKind {
    RowId(&'static str),
    PrimaryKey(Vec<String>),
    /// Views: rows cannot be addressed.
    None,
}

/// A table or view prepared for record access.
struct Source {
    name: String,
    def: Option<TableDef>,
    columns: Vec<String>,
    key: KeyKind,
}

impl Source {
    fn load(conn: &Connection, name: &str) -> AppResult<Self> {
        let obj = require_object(conn, name)?;
        if obj.kind == "view" {
            let stmt = conn.prepare(&format!("SELECT * FROM {} LIMIT 0", quote_ident(&obj.name)))?;
            let columns = stmt.column_names().into_iter().map(String::from).collect();
            return Ok(Source { name: obj.name, def: None, columns, key: KeyKind::None });
        }
        let loaded = load_table(conn, &obj.name)?;
        let def = loaded.definition;
        let key = if loaded.without_rowid {
            KeyKind::PrimaryKey(def.primary_key().iter().map(|f| f.name.clone()).collect())
        } else {
            KeyKind::RowId(rowid_name(&def)?)
        };
        let columns = def.fields.iter().map(|f| f.name.clone()).collect();
        Ok(Source { name: obj.name, def: Some(def), columns, key })
    }

    fn table_def(&self) -> AppResult<&TableDef> {
        self.def
            .as_ref()
            .ok_or_else(|| AppError::Unsupported(format!("{:?} is a view and is read-only", self.name)))
    }

    fn column(&self, name: &str) -> AppResult<String> {
        self.columns
            .iter()
            .find(|c| c.eq_ignore_ascii_case(name))
            .cloned()
            .ok_or_else(|| AppError::Validation(format!("unknown field {name:?} in {:?}", self.name)))
    }

    fn field(&self, name: &str) -> Option<&FieldDef> {
        self.def.as_ref().and_then(|d| d.field(name))
    }

    /// Value used to compare against a column in a filter.
    fn filter_value(&self, column: &str, value: &JsonValue) -> AppResult<Value> {
        match self.field(column) {
            Some(f) if !matches!(f.field_type, FieldType::Calculated { .. } | FieldType::Unknown { .. }) => {
                coerce_value(f, value)
            }
            _ => untyped_value(value),
        }
    }

    /// WHERE clause and parameters identifying one row.
    fn key_clause(&self, key: &RecordKey) -> AppResult<(String, Vec<Value>)> {
        match (&self.key, key) {
            (KeyKind::RowId(r), RecordKey::RowId(id)) => Ok((format!("{r} = ?"), vec![Value::Integer(*id)])),
            (KeyKind::PrimaryKey(cols), RecordKey::PrimaryKey(map)) => {
                let mut clauses = Vec::new();
                let mut params = Vec::new();
                for col in cols {
                    let v = map
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case(col))
                        .map(|(_, v)| v)
                        .ok_or_else(|| AppError::Validation(format!("record key is missing {col:?}")))?;
                    clauses.push(format!("{} = ?", quote_ident(col)));
                    params.push(self.filter_value(col, v)?);
                }
                Ok((clauses.join(" AND "), params))
            }
            (KeyKind::None, _) => Err(AppError::Unsupported(format!("{:?} is a view and is read-only", self.name))),
            _ => Err(AppError::Validation("record key does not match the table's key type".into())),
        }
    }
}

fn like_pattern(s: &str, prefix: bool, suffix: bool) -> String {
    let escaped = s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    format!("{}{escaped}{}", if prefix { "%" } else { "" }, if suffix { "%" } else { "" })
}

fn build_where(src: &Source, q: &RecordQuery) -> AppResult<(String, Vec<Value>)> {
    let mut clauses: Vec<String> = Vec::new();
    let mut params: Vec<Value> = Vec::new();
    for f in &q.filters {
        let col = src.column(&f.column)?;
        let qc = quote_ident(&col);
        match f.op {
            FilterOp::IsNull => clauses.push(format!("{qc} IS NULL")),
            FilterOp::IsNotNull => clauses.push(format!("{qc} IS NOT NULL")),
            FilterOp::Contains | FilterOp::StartsWith | FilterOp::EndsWith => {
                let s = f.value.as_str().ok_or_else(|| {
                    AppError::Validation(format!("filter on {col:?} needs a text value"))
                })?;
                let pattern = match f.op {
                    FilterOp::Contains => like_pattern(s, true, true),
                    FilterOp::StartsWith => like_pattern(s, false, true),
                    _ => like_pattern(s, true, false),
                };
                clauses.push(format!("CAST({qc} AS TEXT) LIKE ? ESCAPE '\\'"));
                params.push(Value::Text(pattern));
            }
            FilterOp::In => {
                let items = f.value.as_array().ok_or_else(|| {
                    AppError::Validation(format!("filter on {col:?} needs a list of values"))
                })?;
                if items.len() > MAX_IN_LIST {
                    return Err(AppError::Validation(format!("at most {MAX_IN_LIST} values per list")));
                }
                if items.is_empty() {
                    clauses.push("0".into());
                } else {
                    clauses.push(format!("{qc} IN ({})", vec!["?"; items.len()].join(", ")));
                    for item in items {
                        params.push(src.filter_value(&col, item)?);
                    }
                }
            }
            op => {
                let v = src.filter_value(&col, &f.value)?;
                if v == Value::Null {
                    match op {
                        FilterOp::Eq => clauses.push(format!("{qc} IS NULL")),
                        FilterOp::Ne => clauses.push(format!("{qc} IS NOT NULL")),
                        _ => return Err(AppError::Validation(format!("cannot compare {col:?} with null"))),
                    }
                    continue;
                }
                let sym = match op {
                    FilterOp::Eq => "=",
                    FilterOp::Ne => "IS NOT",
                    FilterOp::Lt => "<",
                    FilterOp::Le => "<=",
                    FilterOp::Gt => ">",
                    _ => ">=",
                };
                clauses.push(format!("{qc} {sym} ?"));
                params.push(v);
            }
        }
    }
    if let Some(term) = q.search.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        let searchable: Vec<&String> = src
            .columns
            .iter()
            .filter(|c| !src.field(c).is_some_and(|f| f.field_type.is_blob()))
            .collect();
        if !searchable.is_empty() {
            let pattern = like_pattern(term, true, true);
            let ors: Vec<String> = searchable
                .iter()
                .map(|c| format!("CAST({} AS TEXT) LIKE ? ESCAPE '\\'", quote_ident(c)))
                .collect();
            params.extend(std::iter::repeat_n(Value::Text(pattern), ors.len()));
            clauses.push(format!("({})", ors.join(" OR ")));
        }
    }
    let sql = if clauses.is_empty() { String::new() } else { format!(" WHERE {}", clauses.join(" AND ")) };
    Ok((sql, params))
}

fn row_key(src: &Source, rowid: Option<i64>, values: &[JsonValue]) -> Option<RecordKey> {
    match &src.key {
        KeyKind::RowId(_) => rowid.map(RecordKey::RowId),
        KeyKind::PrimaryKey(cols) => {
            let mut map = serde_json::Map::new();
            for c in cols {
                let idx = src.columns.iter().position(|x| x == c)?;
                map.insert(c.clone(), values[idx].clone());
            }
            Some(RecordKey::PrimaryKey(map))
        }
        KeyKind::None => None,
    }
}

fn select_list(src: &Source) -> String {
    let cols: Vec<String> = src.columns.iter().map(|c| quote_ident(c)).collect();
    match &src.key {
        KeyKind::RowId(r) => format!("{r}, {}", cols.join(", ")),
        _ => cols.join(", "),
    }
}

fn read_row(src: &Source, row: &rusqlite::Row<'_>) -> rusqlite::Result<RecordRow> {
    let offset = matches!(src.key, KeyKind::RowId(_)) as usize;
    let rowid = if offset == 1 { Some(row.get::<_, i64>(0)?) } else { None };
    let mut values = Vec::with_capacity(src.columns.len());
    for i in 0..src.columns.len() {
        values.push(sql_to_json(row.get_ref(i + offset)?));
    }
    Ok(RecordRow { key: row_key(src, rowid, &values), values })
}

pub fn list_records(conn: &Connection, table: &str, q: &RecordQuery) -> AppResult<RecordPage> {
    let src = Source::load(conn, table)?;
    let (where_sql, params) = build_where(&src, q)?;
    let mut order: Vec<String> = Vec::new();
    for s in &q.sort {
        let col = src.column(&s.column)?;
        order.push(format!("{} {}", quote_ident(&col), if s.descending { "DESC" } else { "ASC" }));
    }
    match &src.key {
        KeyKind::RowId(r) => order.push(r.to_string()),
        KeyKind::PrimaryKey(cols) => order.extend(cols.iter().map(|c| quote_ident(c))),
        KeyKind::None => {}
    }
    let order_sql = if order.is_empty() { String::new() } else { format!(" ORDER BY {}", order.join(", ")) };
    let limit = match q.limit {
        0 => DEFAULT_PAGE_SIZE,
        n => n.min(MAX_PAGE_SIZE),
    };
    let from = quote_ident(&src.name);

    let total: i64 = conn.query_row(
        &format!("SELECT count(*) FROM {from}{where_sql}"),
        params_from_iter(params.iter()),
        |r| r.get(0),
    )?;
    let mut page_params = params.clone();
    page_params.push(Value::Integer(limit as i64));
    page_params.push(Value::Integer(q.offset.min(i64::MAX as u64) as i64));
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM {from}{where_sql}{order_sql} LIMIT ? OFFSET ?",
        select_list(&src)
    ))?;
    let rows = stmt
        .query_map(params_from_iter(page_params.iter()), |row| read_row(&src, row))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(RecordPage { columns: src.columns.clone(), rows, total, offset: q.offset, limit })
}

pub fn get_record(conn: &Connection, table: &str, key: &RecordKey) -> AppResult<RecordRow> {
    let src = Source::load(conn, table)?;
    let (clause, params) = src.key_clause(key)?;
    conn.query_row(
        &format!("SELECT {} FROM {} WHERE {clause}", select_list(&src), quote_ident(&src.name)),
        params_from_iter(params.iter()),
        |row| read_row(&src, row),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("record {key:?} in {:?}", src.name)),
        e => e.into(),
    })
}

/// Returns the content of an attachment field as `{ "base64": ... }`.
pub fn get_attachment(conn: &Connection, table: &str, key: &RecordKey, column: &str) -> AppResult<JsonValue> {
    use base64::Engine;
    let src = Source::load(conn, table)?;
    let col = src.column(column)?;
    let (clause, params) = src.key_clause(key)?;
    let data: Option<Vec<u8>> = conn
        .query_row(
            &format!("SELECT {} FROM {} WHERE {clause}", quote_ident(&col), quote_ident(&src.name)),
            params_from_iter(params.iter()),
            |r| r.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("record {key:?}")),
            rusqlite::Error::InvalidColumnType(..) => AppError::Validation(format!("{col:?} does not hold binary data")),
            e => e.into(),
        })?;
    Ok(match data {
        Some(bytes) => serde_json::json!({ "base64": base64::engine::general_purpose::STANDARD.encode(bytes) }),
        None => JsonValue::Null,
    })
}

/// Validates and converts input values. Returns (column, value) pairs.
fn prepare_values(def: &TableDef, values: &RecordValues) -> AppResult<Vec<(String, Value)>> {
    let mut out = Vec::with_capacity(values.len());
    let mut errors = Vec::new();
    for (name, raw) in values {
        let Some(field) = def.field(name) else {
            errors.push(format!("unknown field {name:?}"));
            continue;
        };
        match coerce_value(field, raw) {
            Ok(v) => out.push((field.name.clone(), v)),
            Err(e) => errors.push(match e {
                AppError::Validation(m) => m,
                other => other.to_string(),
            }),
        }
    }
    if !errors.is_empty() {
        return Err(AppError::Validation(errors.join("; ")));
    }
    Ok(out)
}

fn needs_value(f: &FieldDef) -> bool {
    (f.required || f.primary_key)
        && f.default.is_none()
        && !matches!(f.field_type, FieldType::AutoNumber | FieldType::Calculated { .. })
}

fn insert_one(conn: &Connection, src: &Source, values: &RecordValues) -> AppResult<RecordKey> {
    let def = src.table_def()?;
    let prepared = prepare_values(def, values)?;
    let missing: Vec<&str> = def
        .fields
        .iter()
        .filter(|f| needs_value(f))
        .filter(|f| !prepared.iter().any(|(c, v)| c == &f.name && *v != Value::Null))
        .map(|f| f.name.as_str())
        .collect();
    if !missing.is_empty() {
        return Err(AppError::Validation(format!("required fields are missing: {missing:?}")));
    }
    let table = quote_ident(&src.name);
    if prepared.is_empty() {
        conn.execute(&format!("INSERT INTO {table} DEFAULT VALUES"), [])?;
    } else {
        let cols: Vec<String> = prepared.iter().map(|(c, _)| quote_ident(c)).collect();
        conn.execute(
            &format!(
                "INSERT INTO {table} ({}) VALUES ({})",
                cols.join(", "),
                vec!["?"; cols.len()].join(", ")
            ),
            params_from_iter(prepared.iter().map(|(_, v)| v)),
        )?;
    }
    match &src.key {
        KeyKind::RowId(_) => Ok(RecordKey::RowId(conn.last_insert_rowid())),
        KeyKind::PrimaryKey(cols) => {
            let mut map = serde_json::Map::new();
            for c in cols {
                let v = prepared.iter().find(|(n, _)| n == c).map(|(_, v)| v.clone()).unwrap_or(Value::Null);
                map.insert(c.clone(), sql_to_json((&v).into()));
            }
            Ok(RecordKey::PrimaryKey(map))
        }
        KeyKind::None => unreachable!("views are rejected by table_def"),
    }
}

fn update_one(conn: &Connection, src: &Source, key: &RecordKey, values: &RecordValues) -> AppResult<RecordKey> {
    let def = src.table_def()?;
    if values.is_empty() {
        return Err(AppError::Validation("no values to update".into()));
    }
    let prepared = prepare_values(def, values)?;
    let nulled: Vec<&str> = prepared
        .iter()
        .filter(|(c, v)| *v == Value::Null && def.field(c).is_some_and(|f| f.required || f.primary_key))
        .map(|(c, _)| c.as_str())
        .collect();
    if !nulled.is_empty() {
        return Err(AppError::Validation(format!("required fields cannot be empty: {nulled:?}")));
    }
    let (clause, key_params) = src.key_clause(key)?;
    let sets: Vec<String> = prepared.iter().map(|(c, _)| format!("{} = ?", quote_ident(c))).collect();
    let params: Vec<&Value> = prepared.iter().map(|(_, v)| v).chain(key_params.iter()).collect();
    let changed = conn.execute(
        &format!("UPDATE {} SET {} WHERE {clause}", quote_ident(&src.name), sets.join(", ")),
        params_from_iter(params),
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("record {key:?} in {:?}", src.name)));
    }
    // The key changes when the primary key itself was updated.
    Ok(match (key, &src.key) {
        (RecordKey::RowId(id), _) => match rowid_alias(def).and_then(|pk| prepared.iter().find(|(c, _)| *c == pk.name)) {
            Some((_, Value::Integer(new_id))) => RecordKey::RowId(*new_id),
            _ => RecordKey::RowId(*id),
        },
        (RecordKey::PrimaryKey(map), KeyKind::PrimaryKey(cols)) => {
            let mut map = map.clone();
            for c in cols {
                if let Some((_, v)) = prepared.iter().find(|(n, _)| n == c) {
                    map.retain(|k, _| !k.eq_ignore_ascii_case(c));
                    map.insert(c.clone(), sql_to_json(v.into()));
                }
            }
            RecordKey::PrimaryKey(map)
        }
        (k, _) => k.clone(),
    })
}

fn delete_one(conn: &Connection, src: &Source, key: &RecordKey) -> AppResult<()> {
    src.table_def()?;
    let (clause, params) = src.key_clause(key)?;
    let changed = conn.execute(
        &format!("DELETE FROM {} WHERE {clause}", quote_ident(&src.name)),
        params_from_iter(params.iter()),
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("record {key:?} in {:?}", src.name)));
    }
    Ok(())
}

pub fn insert_record(conn: &Connection, table: &str, values: &RecordValues) -> AppResult<RecordKey> {
    let src = Source::load(conn, table)?;
    with_transaction(conn, |tx| insert_one(tx, &src, values))
}

pub fn update_record(conn: &Connection, table: &str, key: &RecordKey, values: &RecordValues) -> AppResult<RecordKey> {
    let src = Source::load(conn, table)?;
    with_transaction(conn, |tx| update_one(tx, &src, key, values))
}

/// Deletes all given records or none of them.
pub fn delete_records(conn: &Connection, table: &str, keys: &[RecordKey], confirm: bool) -> AppResult<usize> {
    if !confirm {
        return Err(AppError::ConfirmationRequired(format!(
            "deleting {} record(s) cannot be undone",
            keys.len()
        )));
    }
    let src = Source::load(conn, table)?;
    with_transaction(conn, |tx| {
        for (i, key) in keys.iter().enumerate() {
            delete_one(tx, &src, key).map_err(|e| e.context(format!("record #{}", i + 1)))?;
        }
        Ok(keys.len())
    })
}

/// Applies a list of operations atomically: if any fails, none is applied.
pub fn batch(conn: &Connection, table: &str, ops: &[BatchOp], confirm_deletes: bool) -> AppResult<BatchResult> {
    if ops.len() > MAX_BATCH_OPS {
        return Err(AppError::Validation(format!("at most {MAX_BATCH_OPS} operations per batch")));
    }
    let deletes = ops.iter().filter(|o| matches!(o, BatchOp::Delete { .. })).count();
    if deletes > 0 && !confirm_deletes {
        return Err(AppError::ConfirmationRequired(format!(
            "the batch deletes {deletes} record(s), which cannot be undone"
        )));
    }
    let src = Source::load(conn, table)?;
    with_transaction(conn, |tx| {
        let mut result = BatchResult { inserted: Vec::new(), updated: 0, deleted: 0 };
        for (i, op) in ops.iter().enumerate() {
            let ctx = format!("operation #{}", i + 1);
            match op {
                BatchOp::Insert { values } => {
                    result.inserted.push(insert_one(tx, &src, values).map_err(|e| e.context(&ctx))?)
                }
                BatchOp::Update { key, values } => {
                    update_one(tx, &src, key, values).map_err(|e| e.context(&ctx))?;
                    result.updated += 1;
                }
                BatchOp::Delete { key } => {
                    delete_one(tx, &src, key).map_err(|e| e.context(&ctx))?;
                    result.deleted += 1;
                }
            }
        }
        Ok(result)
    })
}

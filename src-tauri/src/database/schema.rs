//! Schema introspection and DDL generation.

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension};

use crate::errors::{AppError, AppResult};
use crate::models::{
    CalculatedResult, DefaultValue, FieldDef, FieldType, FkAction, ForeignKeyDef, IndexInfo,
    ObjectKind, ObjectSummary, TableDef, TableInfo,
};
use crate::validation::expressions::bare_words_upper;
use crate::validation::identifiers::{is_reserved_name, quote_ident};
use crate::validation::values::{coerce_value, render_literal};

use super::metadata;

#[derive(Debug, Clone)]
pub struct SchemaObject {
    pub kind: String,
    pub name: String,
    pub sql: Option<String>,
}

/// Looks up any schema object by name (case-insensitive, like SQLite).
pub fn find_object(conn: &Connection, name: &str) -> AppResult<Option<SchemaObject>> {
    Ok(conn
        .query_row(
            "SELECT type, name, sql FROM sqlite_schema WHERE name = ?1 COLLATE NOCASE",
            [name],
            |r| Ok(SchemaObject { kind: r.get(0)?, name: r.get(1)?, sql: r.get(2)? }),
        )
        .optional()?)
}

/// Resolves a user table or view by name.
pub fn require_object(conn: &Connection, name: &str) -> AppResult<SchemaObject> {
    match find_object(conn, name)? {
        Some(o) if (o.kind == "table" || o.kind == "view") && !is_reserved_name(&o.name) => Ok(o),
        _ => Err(AppError::NotFound(format!("table or view {name:?}"))),
    }
}

/// Resolves a user table (not a view) by name.
pub fn require_table(conn: &Connection, name: &str) -> AppResult<SchemaObject> {
    let o = require_object(conn, name)?;
    if o.kind != "table" {
        return Err(AppError::Unsupported(format!("{:?} is a view, not a table", o.name)));
    }
    Ok(o)
}

pub fn list_objects(conn: &Connection) -> AppResult<Vec<ObjectSummary>> {
    let managed: Vec<String> = metadata::stored_names(conn)?;
    let mut stmt = conn.prepare(
        "SELECT type, name, sql FROM sqlite_schema
         WHERE type IN ('table', 'view') ORDER BY type, name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (kind, name, sql) = row?;
        if is_reserved_name(&name) {
            continue;
        }
        let is_managed = kind == "table"
            && managed.iter().any(|m| m.eq_ignore_ascii_case(&name))
            && metadata::load(conn, &name)?
                .is_some_and(|s| Some(&s.schema_sql) == sql.as_ref());
        out.push(ObjectSummary {
            name,
            kind: if kind == "view" { ObjectKind::View } else { ObjectKind::Table },
            managed: is_managed,
        });
    }
    Ok(out)
}

/// The column that aliases the rowid (`INTEGER PRIMARY KEY`), if any.
pub fn rowid_alias(def: &TableDef) -> Option<&FieldDef> {
    let pk = def.primary_key();
    match pk.as_slice() {
        [f] if f.field_type.sql_type().eq_ignore_ascii_case("INTEGER") => Some(f),
        _ => None,
    }
}

/// A name that refers to the rowid and is not shadowed by a real column.
pub fn rowid_name(def: &TableDef) -> AppResult<&'static str> {
    ["rowid", "_rowid_", "oid"]
        .into_iter()
        .find(|n| def.field(n).is_none())
        .ok_or_else(|| {
            AppError::Unsupported(format!(
                "table {:?} shadows every rowid alias with a column",
                def.name
            ))
        })
}

fn column_sql(field: &FieldDef, pk_count: usize) -> AppResult<String> {
    let name = quote_ident(&field.name);
    let mut parts = vec![name.clone()];
    let ty = field.field_type.sql_type();
    if !ty.is_empty() {
        parts.push(ty);
    }
    let mut checks: Vec<String> = Vec::new();
    match &field.field_type {
        FieldType::AutoNumber => parts.push("PRIMARY KEY AUTOINCREMENT".into()),
        FieldType::Calculated { expression, stored, .. } => parts.push(format!(
            "GENERATED ALWAYS AS ({expression}) {}",
            if *stored { "STORED" } else { "VIRTUAL" }
        )),
        FieldType::ShortText { max_length: Some(n) } => checks.push(format!("length({name}) <= {n}")),
        FieldType::Currency { .. } => checks.push(format!("typeof({name}) IN ('integer', 'null')")),
        FieldType::DateTime => checks.push(format!("{name} IS NULL OR datetime({name}) IS NOT NULL")),
        FieldType::YesNo => checks.push(format!("{name} IN (0, 1)")),
        _ => {}
    }
    let is_auto = matches!(field.field_type, FieldType::AutoNumber);
    if field.required || (field.primary_key && !is_auto) {
        parts.push("NOT NULL".into());
    }
    if field.unique && !(field.primary_key && pk_count == 1) {
        parts.push("UNIQUE".into());
    }
    if let Some(default) = &field.default {
        let rendered = match default {
            DefaultValue::Literal(v) => render_literal(&coerce_value(field, v)?)?,
            DefaultValue::CurrentTimestamp => "CURRENT_TIMESTAMP".into(),
            DefaultValue::CurrentDate => "CURRENT_DATE".into(),
            DefaultValue::CurrentTime => "CURRENT_TIME".into(),
            DefaultValue::Expression(e) => format!("({e})"),
        };
        parts.push(format!("DEFAULT {rendered}"));
    }
    if let Some(check) = &field.check {
        checks.push(check.clone());
    }
    for c in checks {
        parts.push(format!("CHECK ({c})"));
    }
    Ok(parts.join(" "))
}

fn column_list(cols: &[String]) -> String {
    cols.iter().map(|c| quote_ident(c)).collect::<Vec<_>>().join(", ")
}

/// Generates CREATE TABLE for `def` under `table_name`. The definition must
/// already be validated.
pub fn create_table_sql(def: &TableDef, table_name: &str) -> AppResult<String> {
    let pk: Vec<String> = def.primary_key().iter().map(|f| f.name.clone()).collect();
    let mut lines = Vec::new();
    for field in &def.fields {
        lines.push(column_sql(field, pk.len())?);
    }
    let has_auto = def.fields.iter().any(|f| matches!(f.field_type, FieldType::AutoNumber));
    if !pk.is_empty() && !has_auto {
        lines.push(format!("PRIMARY KEY ({})", column_list(&pk)));
    }
    for cols in &def.unique_constraints {
        lines.push(format!("UNIQUE ({})", column_list(cols)));
    }
    for fk in &def.foreign_keys {
        let mut s = format!(
            "FOREIGN KEY ({}) REFERENCES {}",
            column_list(&fk.columns),
            quote_ident(&fk.ref_table)
        );
        if !fk.ref_columns.is_empty() {
            s.push_str(&format!(" ({})", column_list(&fk.ref_columns)));
        }
        s.push_str(&format!(" ON DELETE {} ON UPDATE {}", fk.on_delete.sql(), fk.on_update.sql()));
        lines.push(s);
    }
    for check in &def.checks {
        lines.push(format!("CHECK ({check})"));
    }
    Ok(format!("CREATE TABLE {} (\n  {}\n)", quote_ident(table_name), lines.join(",\n  ")))
}

pub struct LoadedTable {
    pub definition: TableDef,
    pub managed: bool,
    pub unsupported_features: Vec<String>,
    pub schema_sql: String,
    pub without_rowid: bool,
}

/// Loads a table definition: from metadata when it matches the live schema,
/// otherwise inferred from SQLite's schema pragmas.
pub fn load_table(conn: &Connection, name: &str) -> AppResult<LoadedTable> {
    let obj = require_table(conn, name)?;
    let sql = obj.sql.clone().unwrap_or_default();
    let words = bare_words_upper(&sql)?;
    let without_rowid = words.contains("WITHOUT");
    let stored = metadata::load(conn, &obj.name)?;
    if let Some(s) = &stored
        && s.schema_sql == sql {
            let mut definition = s.definition.clone();
            definition.name = obj.name.clone();
            return Ok(LoadedTable {
                definition,
                managed: true,
                unsupported_features: Vec::new(),
                schema_sql: sql,
                without_rowid,
            });
        }
    let (mut definition, unsupported_features) = infer_table(conn, &obj.name, &sql)?;
    // Keep descriptions from stale metadata where the field still exists.
    if let Some(s) = stored {
        definition.description = s.definition.description.clone();
        for f in &mut definition.fields {
            if let Some(old) = s.definition.field(&f.name) {
                f.description = old.description.clone();
            }
        }
    }
    Ok(LoadedTable { definition, managed: false, unsupported_features, schema_sql: sql, without_rowid })
}

fn infer_type(declared: &str, sole_integer_pk: bool) -> FieldType {
    match declared.trim().to_ascii_uppercase().as_str() {
        "INTEGER" if sole_integer_pk => FieldType::AutoNumber,
        "INTEGER" => FieldType::LargeNumber,
        "TEXT" => FieldType::LongText,
        "REAL" => FieldType::Number { real: true },
        "BLOB" => FieldType::Attachment,
        _ => FieldType::Unknown { declared: declared.trim().to_string() },
    }
}

fn infer_table(conn: &Connection, name: &str, sql: &str) -> AppResult<(TableDef, Vec<String>)> {
    let words = bare_words_upper(sql)?;
    let mut unsupported = Vec::new();
    for (word, feature) in [
        ("CHECK", "CHECK constraints"),
        ("COLLATE", "column collations"),
        ("WITHOUT", "WITHOUT ROWID"),
        ("STRICT", "STRICT tables"),
        ("CONFLICT", "ON CONFLICT clauses"),
        ("DEFERRABLE", "deferrable foreign keys"),
        ("ASC", "ordered primary keys"),
        ("DESC", "ordered primary keys"),
    ] {
        if words.contains(word) && !unsupported.contains(&feature.to_string()) {
            unsupported.push(feature.to_string());
        }
    }

    struct Col {
        name: String,
        declared: String,
        notnull: bool,
        default: Option<String>,
        pk: i64,
        hidden: i64,
    }
    let mut stmt = conn.prepare(
        "SELECT name, type, \"notnull\", dflt_value, pk, hidden FROM pragma_table_xinfo(?1) ORDER BY cid",
    )?;
    let cols: Vec<Col> = stmt
        .query_map([name], |r| {
            Ok(Col {
                name: r.get(0)?,
                declared: r.get(1)?,
                notnull: r.get(2)?,
                default: r.get(3)?,
                pk: r.get(4)?,
                hidden: r.get(5)?,
            })
        })?
        .collect::<Result<_, _>>()?;

    let pk_cols: Vec<&Col> = cols.iter().filter(|c| c.pk > 0).collect();
    if pk_cols.windows(2).any(|w| w[0].pk > w[1].pk) {
        unsupported.push("primary key column order".into());
    }
    let mut fields = Vec::new();
    for c in &cols {
        let sole_integer_pk = c.pk > 0 && pk_cols.len() == 1;
        let mut field_type = infer_type(&c.declared, sole_integer_pk);
        if c.hidden == 2 || c.hidden == 3 {
            unsupported.push(format!("generated column {:?}", c.name));
            field_type = FieldType::Calculated {
                expression: String::new(),
                stored: c.hidden == 3,
                result: CalculatedResult::Any,
            };
        } else if c.hidden != 0 {
            continue;
        }
        let mut f = FieldDef::new(&c.name, field_type);
        f.primary_key = c.pk > 0;
        f.required = c.notnull;
        f.default = c.default.clone().map(DefaultValue::Expression);
        fields.push(f);
    }

    let mut fk_stmt = conn.prepare(
        "SELECT id, \"table\", \"from\", \"to\", on_update, on_delete
         FROM pragma_foreign_key_list(?1) ORDER BY id, seq",
    )?;
    let mut fks: Vec<(i64, ForeignKeyDef)> = Vec::new();
    let rows = fk_stmt.query_map([name], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, Option<String>>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
        ))
    })?;
    for row in rows {
        let (id, table, from, to, on_update, on_delete) = row?;
        match fks.last_mut() {
            Some((last, fk)) if *last == id => {
                fk.columns.push(from);
                if let Some(t) = to {
                    fk.ref_columns.push(t);
                }
            }
            _ => fks.push((
                id,
                ForeignKeyDef {
                    columns: vec![from],
                    ref_table: table,
                    ref_columns: to.into_iter().collect(),
                    on_delete: FkAction::parse(&on_delete),
                    on_update: FkAction::parse(&on_update),
                },
            )),
        }
    }
    // pragma_foreign_key_list lists constraints in reverse declaration order.
    let foreign_keys: Vec<ForeignKeyDef> = fks.into_iter().rev().map(|(_, fk)| fk).collect();

    let mut unique_constraints = Vec::new();
    for idx in indexes(conn, name)? {
        if idx.origin != "u" {
            continue;
        }
        if let [col] = idx.columns.as_slice()
            && let Some(f) = fields.iter_mut().find(|f| f.name.eq_ignore_ascii_case(col)) {
                f.unique = true;
                continue;
            }
        unique_constraints.push(idx.columns.clone());
    }
    unique_constraints.reverse();

    Ok((
        TableDef {
            name: name.to_string(),
            fields,
            foreign_keys,
            unique_constraints,
            checks: Vec::new(),
            description: None,
        },
        unsupported,
    ))
}

pub fn indexes(conn: &Connection, table: &str) -> AppResult<Vec<IndexInfo>> {
    let mut stmt =
        conn.prepare("SELECT name, \"unique\", origin, partial FROM pragma_index_list(?1)")?;
    let list: Vec<(String, bool, String, bool)> = stmt
        .query_map([table], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<_, _>>()?;
    let mut col_stmt =
        conn.prepare("SELECT name FROM pragma_index_info(?1) ORDER BY seqno")?;
    let mut out = Vec::new();
    for (name, unique, origin, partial) in list {
        let columns: Vec<String> = col_stmt
            .query_map([&name], |r| r.get::<_, Option<String>>(0))?
            .map(|c| c.map(|c| c.unwrap_or_else(|| "<expression>".into())))
            .collect::<Result<_, _>>()?;
        out.push(IndexInfo { name, unique, origin, partial, columns });
    }
    Ok(out)
}

pub fn row_count(conn: &Connection, table: &str) -> AppResult<i64> {
    Ok(conn.query_row(&format!("SELECT count(*) FROM {}", quote_ident(table)), [], |r| r.get(0))?)
}

pub fn table_info(conn: &Connection, name: &str) -> AppResult<TableInfo> {
    let loaded = load_table(conn, name)?;
    let table = loaded.definition.name.clone();
    Ok(TableInfo {
        row_count: row_count(conn, &table)?,
        indexes: indexes(conn, &table)?,
        definition: loaded.definition,
        managed: loaded.managed,
        unsupported_features: loaded.unsupported_features,
        schema_sql: loaded.schema_sql,
        without_rowid: loaded.without_rowid,
    })
}

/// Names of tables whose foreign keys reference `table`, excluding itself.
pub fn referencing_tables(conn: &Connection, table: &str) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT m.name FROM sqlite_schema m, pragma_foreign_key_list(m.name) f
         WHERE m.type = 'table' AND f.\"table\" = ?1 COLLATE NOCASE
           AND m.name <> ?1 COLLATE NOCASE",
    )?;
    let names = stmt.query_map([table], |r| r.get(0))?.collect::<Result<_, _>>()?;
    Ok(names)
}

/// Columns of each index on `table` declared with CREATE INDEX or triggers,
/// keyed by object name, with their SQL. Used to recreate them on rebuild.
pub struct DependentObject {
    pub kind: String,
    pub name: String,
    pub sql: String,
    /// Index columns; empty for triggers.
    pub columns: Vec<String>,
}

pub fn dependent_objects(conn: &Connection, table: &str) -> AppResult<Vec<DependentObject>> {
    let mut stmt = conn.prepare(
        "SELECT type, name, sql FROM sqlite_schema
         WHERE tbl_name = ?1 COLLATE NOCASE AND type IN ('index', 'trigger') AND sql IS NOT NULL",
    )?;
    let objs: Vec<(String, String, String)> = stmt
        .query_map([table], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let info: HashMap<String, Vec<String>> =
        indexes(conn, table)?.into_iter().map(|i| (i.name, i.columns)).collect();
    Ok(objs
        .into_iter()
        .map(|(kind, name, sql)| DependentObject {
            columns: info.get(&name).cloned().unwrap_or_default(),
            kind,
            name,
            sql,
        })
        .collect())
}

/// Views that currently compile. Used to detect views broken by a change.
pub fn working_views(conn: &Connection) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM sqlite_schema WHERE type = 'view'")?;
    let views: Vec<String> = stmt.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
    Ok(views
        .into_iter()
        .filter(|v| conn.prepare(&format!("SELECT * FROM {} LIMIT 0", quote_ident(v))).is_ok())
        .collect())
}

/// Rows violating foreign keys, as (child table, rowid, parent table).
pub fn foreign_key_violations(
    conn: &Connection,
) -> AppResult<std::collections::HashSet<(String, Option<i64>, String)>> {
    let mut stmt = conn.prepare("PRAGMA foreign_key_check")?;
    let rows = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

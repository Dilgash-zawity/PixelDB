//! Indexes the user adds to speed up searching and sorting, or to keep a
//! combination of fields unique. Only indexes made with CREATE INDEX can be
//! dropped here; the ones behind keys and UNIQUE rules belong to the table.

use crate::errors::{AppError, AppResult};
use crate::models::IndexInfo;
use crate::validation::identifiers::{is_reserved_name, quote_ident, validate_identifier};

use rusqlite::Connection;

use super::schema::{find_object, indexes, load_table, require_table};

pub const MAX_INDEX_COLUMNS: usize = 8;

/// A free index name like `idx_Orders_Customer_Date`.
fn index_name(conn: &Connection, table: &str, columns: &[String]) -> AppResult<String> {
    let base: String = format!("idx_{table}_{}", columns.join("_"))
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .take(60)
        .collect();
    for n in 1.. {
        let name = if n == 1 { base.clone() } else { format!("{base}_{n}") };
        if find_object(conn, &name)?.is_none() {
            return Ok(name);
        }
    }
    unreachable!()
}

/// Adds an index on `columns` of `table` (unique when `unique`).
pub fn create_index(conn: &Connection, table: &str, columns: &[String], unique: bool) -> AppResult<IndexInfo> {
    let table = require_table(conn, table)?.name;
    if columns.is_empty() {
        return Err(AppError::Validation("choose at least one field".into()));
    }
    if columns.len() > MAX_INDEX_COLUMNS {
        return Err(AppError::Validation(format!("an index can cover at most {MAX_INDEX_COLUMNS} fields")));
    }
    let def = load_table(conn, &table)?.definition;
    let mut cols: Vec<String> = Vec::new();
    for c in columns {
        let f = def
            .field(c)
            .ok_or_else(|| AppError::Validation(format!("{table:?} has no field {c:?}")))?;
        if f.field_type.is_blob() {
            return Err(AppError::Validation(format!("{:?} holds files and can't be indexed", f.name)));
        }
        if cols.iter().any(|x| x.eq_ignore_ascii_case(&f.name)) {
            return Err(AppError::Validation(format!("{:?} is listed twice", f.name)));
        }
        cols.push(f.name.clone());
    }
    let same = indexes(conn, &table)?
        .into_iter()
        .find(|i| !i.partial && i.unique == unique && i.columns.len() == cols.len() && i.columns.iter().zip(&cols).all(|(a, b)| a.eq_ignore_ascii_case(b)));
    if let Some(i) = same {
        return Err(AppError::AlreadyExists(format!("an index on these fields ({:?})", i.name)));
    }
    let name = index_name(conn, &table, &cols)?;
    validate_identifier(&name)?;
    let list: Vec<String> = cols.iter().map(|c| quote_ident(c)).collect();
    conn.execute_batch(&format!(
        "CREATE {}INDEX {} ON {} ({})",
        if unique { "UNIQUE " } else { "" },
        quote_ident(&name),
        quote_ident(&table),
        list.join(", ")
    ))
    .map_err(|e| match AppError::from(e) {
        AppError::Constraint(_) => AppError::Constraint(format!(
            "some records already share the same {}, so they can't be made unique",
            cols.join(" + ")
        )),
        other => other,
    })?;
    indexes(conn, &table)?
        .into_iter()
        .find(|i| i.name == name)
        .ok_or_else(|| AppError::Internal("the new index was not found".into()))
}

/// Drops an index made with CREATE INDEX. No data is lost, so no confirmation is needed.
pub fn drop_index(conn: &Connection, table: &str, name: &str) -> AppResult<()> {
    let table = require_table(conn, table)?.name;
    let found = indexes(conn, &table)?
        .into_iter()
        .find(|i| i.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| AppError::NotFound(format!("index {name:?} on {table:?}")))?;
    if found.origin != "c" || is_reserved_name(&found.name) {
        return Err(AppError::Unsupported(format!(
            "{:?} belongs to the table's key or unique rules; change those on the field instead",
            found.name
        )));
    }
    conn.execute_batch(&format!("DROP INDEX {}", quote_ident(&found.name)))?;
    Ok(())
}

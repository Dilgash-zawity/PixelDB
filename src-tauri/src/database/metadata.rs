//! Application metadata stored inside the user's database in `__app_*`
//! tables: Access-style field types, descriptions and other information that
//! SQLite's schema cannot represent.

use rusqlite::{Connection, OptionalExtension, params};

use crate::errors::{AppError, AppResult};
use crate::models::{DiagramLayout, TableDef};

pub const META_TABLE: &str = "__app_meta";
pub const TABLES_TABLE: &str = "__app_tables";
pub const METADATA_VERSION: i64 = 1;
const LAYOUT_KEY: &str = "relationship_layout";
/// Positions further out than this are treated as corrupt input.
const MAX_COORDINATE: f64 = 1_000_000.0;

/// A stored definition together with the CREATE TABLE text it describes.
/// The definition is trusted only while `schema_sql` matches the live schema.
pub struct StoredTable {
    pub definition: TableDef,
    pub schema_sql: String,
}

pub fn ensure(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {META_TABLE} (
            key TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS {TABLES_TABLE} (
            name TEXT PRIMARY KEY NOT NULL COLLATE NOCASE,
            definition TEXT NOT NULL,
            schema_sql TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         );"
    ))?;
    conn.execute(
        &format!("INSERT OR IGNORE INTO {META_TABLE} (key, value) VALUES ('version', ?1)"),
        [METADATA_VERSION.to_string()],
    )?;
    Ok(())
}

fn tables_exist(conn: &Connection) -> AppResult<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = ?1",
            [TABLES_TABLE],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

pub fn load(conn: &Connection, name: &str) -> AppResult<Option<StoredTable>> {
    if !tables_exist(conn)? {
        return Ok(None);
    }
    let row: Option<(String, String)> = conn
        .query_row(
            &format!("SELECT definition, schema_sql FROM {TABLES_TABLE} WHERE name = ?1"),
            [name],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match row {
        Some((def, schema_sql)) => match serde_json::from_str::<TableDef>(&def) {
            Ok(definition) => Ok(Some(StoredTable { definition, schema_sql })),
            Err(e) => {
                log::warn!("ignoring unreadable metadata for {name:?}: {e}");
                Ok(None)
            }
        },
        None => Ok(None),
    }
}

pub fn save(conn: &Connection, def: &TableDef, schema_sql: &str) -> AppResult<()> {
    ensure(conn)?;
    conn.execute(
        &format!(
            "INSERT INTO {TABLES_TABLE} (name, definition, schema_sql, updated_at)
             VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)
             ON CONFLICT(name) DO UPDATE SET
                name = excluded.name,
                definition = excluded.definition,
                schema_sql = excluded.schema_sql,
                updated_at = excluded.updated_at"
        ),
        params![def.name, serde_json::to_string(def)?, schema_sql],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, name: &str) -> AppResult<()> {
    if tables_exist(conn)? {
        conn.execute(&format!("DELETE FROM {TABLES_TABLE} WHERE name = ?1"), [name])?;
    }
    Ok(())
}

/// Names of all tables that have stored metadata.
pub fn stored_names(conn: &Connection) -> AppResult<Vec<String>> {
    if !tables_exist(conn)? {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare(&format!("SELECT name FROM {TABLES_TABLE} ORDER BY name"))?;
    let names = stmt.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
    Ok(names)
}

fn meta_exists(conn: &Connection) -> AppResult<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = ?1",
            [META_TABLE],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

/// The saved Relationships diagram layout; empty when none was saved.
pub fn load_layout(conn: &Connection) -> AppResult<DiagramLayout> {
    if !meta_exists(conn)? {
        return Ok(DiagramLayout::new());
    }
    let raw: Option<String> = conn
        .query_row(
            &format!("SELECT value FROM {META_TABLE} WHERE key = ?1"),
            [LAYOUT_KEY],
            |r| r.get(0),
        )
        .optional()?;
    Ok(raw
        .and_then(|json| match serde_json::from_str(&json) {
            Ok(layout) => Some(layout),
            Err(e) => {
                log::warn!("ignoring unreadable diagram layout: {e}");
                None
            }
        })
        .unwrap_or_default())
}

pub fn save_layout(conn: &Connection, layout: &DiagramLayout) -> AppResult<()> {
    if layout
        .values()
        .any(|p| !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > MAX_COORDINATE || p.y.abs() > MAX_COORDINATE)
    {
        return Err(AppError::Validation("diagram positions must be finite and in range".into()));
    }
    ensure(conn)?;
    conn.execute(
        &format!(
            "INSERT INTO {META_TABLE} (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value"
        ),
        params![LAYOUT_KEY, serde_json::to_string(layout)?],
    )?;
    Ok(())
}

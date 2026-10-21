use serde::{Deserialize, Serialize};

use super::field::{FieldDef, ForeignKeyDef};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
    #[serde(default)]
    pub foreign_keys: Vec<ForeignKeyDef>,
    /// Multi-column UNIQUE constraints. Single-column ones live on the field.
    #[serde(default)]
    pub unique_constraints: Vec<Vec<String>>,
    /// Table-level CHECK expressions.
    #[serde(default)]
    pub checks: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

impl TableDef {
    pub fn field(&self, name: &str) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.name.eq_ignore_ascii_case(name))
    }

    pub fn primary_key(&self) -> Vec<&FieldDef> {
        self.fields.iter().filter(|f| f.primary_key).collect()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexInfo {
    pub name: String,
    pub unique: bool,
    /// "c" = CREATE INDEX, "u" = UNIQUE constraint, "pk" = PRIMARY KEY.
    pub origin: String,
    pub partial: bool,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableInfo {
    pub definition: TableDef,
    /// True when `definition` comes from application metadata and matches the
    /// live schema. False when it was inferred from the SQLite schema.
    pub managed: bool,
    /// Schema features that cannot be reconstructed from an inferred
    /// definition. Structural changes that require a rebuild are refused
    /// while this list is non-empty.
    pub unsupported_features: Vec<String>,
    pub schema_sql: String,
    pub without_rowid: bool,
    pub row_count: i64,
    pub indexes: Vec<IndexInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnRename {
    pub from: String,
    pub to: String,
}

/// A full table design submitted from the table designer. Fields are matched
/// to existing columns by name, after `renames` are applied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDesign {
    pub definition: TableDef,
    #[serde(default)]
    pub renames: Vec<ColumnRename>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SchemaChangeResult {
    /// True when the table had to be rebuilt to apply the change.
    pub rebuilt: bool,
    pub warnings: Vec<String>,
}

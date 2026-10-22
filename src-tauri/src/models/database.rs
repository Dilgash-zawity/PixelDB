use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct DatabaseInfo {
    pub path: String,
    pub file_size: u64,
    pub sqlite_version: String,
    pub page_size: i64,
    pub page_count: i64,
    pub encoding: String,
    pub journal_mode: String,
    pub user_version: i64,
    pub application_id: i64,
    pub foreign_keys: bool,
    pub table_count: usize,
    pub view_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Table,
    View,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObjectSummary {
    pub name: String,
    pub kind: ObjectKind,
    /// True when the table definition is owned by this application's metadata.
    pub managed: bool,
}

/// Where a table's card sits on the Relationships diagram.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TablePosition {
    pub x: f64,
    pub y: f64,
}

/// Card positions on the Relationships diagram, keyed by table name.
pub type DiagramLayout = BTreeMap<String, TablePosition>;

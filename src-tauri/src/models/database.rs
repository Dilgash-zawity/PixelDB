use serde::Serialize;

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

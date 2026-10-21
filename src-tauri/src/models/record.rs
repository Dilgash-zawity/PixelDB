use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};

/// Identifies a row: by rowid for ordinary tables, by primary key values for
/// WITHOUT ROWID tables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordKey {
    RowId(i64),
    PrimaryKey(Map<String, JsonValue>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortSpec {
    pub column: String,
    #[serde(default)]
    pub descending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Contains,
    StartsWith,
    EndsWith,
    IsNull,
    IsNotNull,
    In,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterSpec {
    pub column: String,
    pub op: FilterOp,
    #[serde(default)]
    pub value: JsonValue,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecordQuery {
    #[serde(default)]
    pub offset: u64,
    /// Clamped to `records::MAX_PAGE_SIZE`; 0 means the default page size.
    #[serde(default)]
    pub limit: u64,
    #[serde(default)]
    pub sort: Vec<SortSpec>,
    #[serde(default)]
    pub filters: Vec<FilterSpec>,
    #[serde(default)]
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordRow {
    /// None for rows of views, which are read-only.
    pub key: Option<RecordKey>,
    pub values: Vec<JsonValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordPage {
    pub columns: Vec<String>,
    pub rows: Vec<RecordRow>,
    pub total: i64,
    pub offset: u64,
    pub limit: u64,
}

pub type RecordValues = Map<String, JsonValue>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum BatchOp {
    Insert { values: RecordValues },
    Update { key: RecordKey, values: RecordValues },
    Delete { key: RecordKey },
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchResult {
    pub inserted: Vec<RecordKey>,
    pub updated: usize,
    pub deleted: usize,
}

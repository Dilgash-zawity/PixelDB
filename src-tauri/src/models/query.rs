use serde::Serialize;
use serde_json::Value as JsonValue;

use super::FieldType;

/// Rows returned by an ad-hoc query.
#[derive(Debug, Clone, Serialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<JsonValue>>,
    /// True when more rows existed than were returned.
    pub truncated: bool,
    pub elapsed_ms: u64,
    /// Field type behind each column, when it comes straight from a table.
    pub types: Vec<Option<FieldType>>,
}

/// A saved query: the SELECT behind its view and, when it was built in the
/// visual designer, that design (opaque JSON owned by the frontend).
#[derive(Debug, Clone, Serialize)]
pub struct SavedQuery {
    pub sql: String,
    pub design: Option<JsonValue>,
}

use serde::Serialize;
use serde_json::Value as JsonValue;

/// Rows returned by an ad-hoc query.
#[derive(Debug, Clone, Serialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<JsonValue>>,
    /// True when more rows existed than were returned.
    pub truncated: bool,
    pub elapsed_ms: u64,
}

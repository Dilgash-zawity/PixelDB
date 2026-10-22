use serde_json::Value as JsonValue;
use tauri::State;

use super::target;
use crate::database::{DbManager, records};
use crate::errors::AppResult;
use crate::models::{
    AggregateQuery, AggregateResult, BatchOp, BatchResult, RecordKey, RecordPage, RecordQuery, RecordRow, RecordValues,
};
use crate::workspace::WorkspaceStore;

// `database` names a workspace database (used by apps); without it a command
// acts on the database open on the database page.

#[tauri::command]
pub async fn list_records(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    database: Option<String>,
    table: String,
    query: RecordQuery,
) -> AppResult<RecordPage> {
    state.run_on(target(&store, database)?, move |db| records::list_records(&db.conn, &table, &query)).await
}

#[tauri::command]
pub async fn aggregate_records(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    database: Option<String>,
    table: String,
    query: AggregateQuery,
) -> AppResult<AggregateResult> {
    state.run_on(target(&store, database)?, move |db| records::aggregate(&db.conn, &table, &query)).await
}

#[tauri::command]
pub async fn get_record(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    database: Option<String>,
    table: String,
    key: RecordKey,
) -> AppResult<RecordRow> {
    state.run_on(target(&store, database)?, move |db| records::get_record(&db.conn, &table, &key)).await
}

#[tauri::command]
pub async fn get_attachment(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    database: Option<String>,
    table: String,
    key: RecordKey,
    column: String,
) -> AppResult<JsonValue> {
    state
        .run_on(target(&store, database)?, move |db| records::get_attachment(&db.conn, &table, &key, &column))
        .await
}

#[tauri::command]
pub async fn insert_record(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    database: Option<String>,
    table: String,
    values: RecordValues,
) -> AppResult<RecordKey> {
    state.run_on(target(&store, database)?, move |db| records::insert_record(&db.conn, &table, &values)).await
}

#[tauri::command]
pub async fn update_record(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    database: Option<String>,
    table: String,
    key: RecordKey,
    values: RecordValues,
) -> AppResult<RecordKey> {
    state
        .run_on(target(&store, database)?, move |db| records::update_record(&db.conn, &table, &key, &values))
        .await
}

#[tauri::command]
pub async fn delete_records(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    database: Option<String>,
    table: String,
    keys: Vec<RecordKey>,
    confirm: bool,
) -> AppResult<usize> {
    state
        .run_on(target(&store, database)?, move |db| records::delete_records(&db.conn, &table, &keys, confirm))
        .await
}

#[tauri::command]
pub async fn batch_records(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    database: Option<String>,
    table: String,
    ops: Vec<BatchOp>,
    confirm_deletes: bool,
) -> AppResult<BatchResult> {
    state
        .run_on(target(&store, database)?, move |db| records::batch(&db.conn, &table, &ops, confirm_deletes))
        .await
}

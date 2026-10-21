use serde_json::Value as JsonValue;
use tauri::State;

use crate::database::{DbManager, records};
use crate::errors::AppResult;
use crate::models::{BatchOp, BatchResult, RecordKey, RecordPage, RecordQuery, RecordRow, RecordValues};

#[tauri::command]
pub async fn list_records(state: State<'_, DbManager>, table: String, query: RecordQuery) -> AppResult<RecordPage> {
    state.run(move |db| records::list_records(&db.conn, &table, &query)).await
}

#[tauri::command]
pub async fn get_record(state: State<'_, DbManager>, table: String, key: RecordKey) -> AppResult<RecordRow> {
    state.run(move |db| records::get_record(&db.conn, &table, &key)).await
}

#[tauri::command]
pub async fn get_attachment(
    state: State<'_, DbManager>,
    table: String,
    key: RecordKey,
    column: String,
) -> AppResult<JsonValue> {
    state.run(move |db| records::get_attachment(&db.conn, &table, &key, &column)).await
}

#[tauri::command]
pub async fn insert_record(state: State<'_, DbManager>, table: String, values: RecordValues) -> AppResult<RecordKey> {
    state.run(move |db| records::insert_record(&db.conn, &table, &values)).await
}

#[tauri::command]
pub async fn update_record(
    state: State<'_, DbManager>,
    table: String,
    key: RecordKey,
    values: RecordValues,
) -> AppResult<RecordKey> {
    state.run(move |db| records::update_record(&db.conn, &table, &key, &values)).await
}

#[tauri::command]
pub async fn delete_records(
    state: State<'_, DbManager>,
    table: String,
    keys: Vec<RecordKey>,
    confirm: bool,
) -> AppResult<usize> {
    state.run(move |db| records::delete_records(&db.conn, &table, &keys, confirm)).await
}

#[tauri::command]
pub async fn batch_records(
    state: State<'_, DbManager>,
    table: String,
    ops: Vec<BatchOp>,
    confirm_deletes: bool,
) -> AppResult<BatchResult> {
    state.run(move |db| records::batch(&db.conn, &table, &ops, confirm_deletes)).await
}

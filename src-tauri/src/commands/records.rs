use serde_json::Value as JsonValue;
use tauri::State;

use super::{on_online, target};
use crate::online::SecretStore;
use crate::online_data::OnlinePools;
use crate::online_records;
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
    secrets: State<'_, SecretStore>,
    pools: State<'_, OnlinePools>,
    database: Option<String>,
    table: String,
    query: RecordQuery,
) -> AppResult<RecordPage> {
    on_online!(store, secrets, pools, database, |o| online_records::list_records(&o, &table, &query));
    state.run_on(target(&store, database)?, move |db| records::list_records(&db.conn, &table, &query)).await
}

#[tauri::command]
pub async fn aggregate_records(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    pools: State<'_, OnlinePools>,
    database: Option<String>,
    table: String,
    query: AggregateQuery,
) -> AppResult<AggregateResult> {
    on_online!(store, secrets, pools, database, |o| online_records::aggregate(&o, &table, &query));
    state.run_on(target(&store, database)?, move |db| records::aggregate(&db.conn, &table, &query)).await
}

#[tauri::command]
pub async fn get_record(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    pools: State<'_, OnlinePools>,
    database: Option<String>,
    table: String,
    key: RecordKey,
) -> AppResult<RecordRow> {
    on_online!(store, secrets, pools, database, |o| online_records::get_record(&o, &table, &key));
    state.run_on(target(&store, database)?, move |db| records::get_record(&db.conn, &table, &key)).await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri passes each managed state separately.
pub async fn get_attachment(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    pools: State<'_, OnlinePools>,
    database: Option<String>,
    table: String,
    key: RecordKey,
    column: String,
) -> AppResult<JsonValue> {
    on_online!(store, secrets, pools, database, |_o| async {
        Err(crate::errors::AppError::Unsupported("files in online databases can't be opened yet".into()))
    });
    state
        .run_on(target(&store, database)?, move |db| records::get_attachment(&db.conn, &table, &key, &column))
        .await
}

#[tauri::command]
pub async fn insert_record(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    pools: State<'_, OnlinePools>,
    database: Option<String>,
    table: String,
    values: RecordValues,
) -> AppResult<RecordKey> {
    on_online!(store, secrets, pools, database, |o| online_records::insert_record(&o, &table, &values));
    state.run_on(target(&store, database)?, move |db| records::insert_record(&db.conn, &table, &values)).await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri passes each managed state separately.
pub async fn update_record(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    pools: State<'_, OnlinePools>,
    database: Option<String>,
    table: String,
    key: RecordKey,
    values: RecordValues,
) -> AppResult<RecordKey> {
    on_online!(store, secrets, pools, database, |o| online_records::update_record(&o, &table, &key, &values));
    state
        .run_on(target(&store, database)?, move |db| records::update_record(&db.conn, &table, &key, &values))
        .await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri passes each managed state separately.
pub async fn delete_records(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    pools: State<'_, OnlinePools>,
    database: Option<String>,
    table: String,
    keys: Vec<RecordKey>,
    confirm: bool,
) -> AppResult<usize> {
    on_online!(store, secrets, pools, database, |o| online_records::delete_records(&o, &table, &keys, confirm));
    state
        .run_on(target(&store, database)?, move |db| records::delete_records(&db.conn, &table, &keys, confirm))
        .await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri passes each managed state separately.
pub async fn batch_records(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    pools: State<'_, OnlinePools>,
    database: Option<String>,
    table: String,
    ops: Vec<BatchOp>,
    confirm_deletes: bool,
) -> AppResult<BatchResult> {
    on_online!(store, secrets, pools, database, |o| online_records::batch(&o, &table, &ops, confirm_deletes));
    state
        .run_on(target(&store, database)?, move |db| records::batch(&db.conn, &table, &ops, confirm_deletes))
        .await
}

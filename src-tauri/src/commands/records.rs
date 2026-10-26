use serde_json::Value as JsonValue;
use tauri::State;

use super::{on_online, target};
use crate::online::SecretStore;
use crate::online_data::OnlinePools;
use crate::online_records;
use crate::database::journal::{self, UndoState};
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

/// What the database page can undo and redo.
#[tauri::command]
pub async fn undo_state(state: State<'_, DbManager>) -> AppResult<UndoState> {
    state.run(|db| Ok(db.journal.state())).await
}

/// Undoes the latest record change in the database page's database.
#[tauri::command]
pub async fn undo_records(state: State<'_, DbManager>) -> AppResult<Option<String>> {
    state.run(|db| journal::undo(&db.conn, &mut db.journal)).await
}

#[tauri::command]
pub async fn redo_records(state: State<'_, DbManager>) -> AppResult<Option<String>> {
    state.run(|db| journal::redo(&db.conn, &mut db.journal)).await
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
    state
        .run_on(target(&store, database)?, move |db| {
            db.recorded(format!("Add a {table} record"), |c| records::insert_record(c, &table, &values))
        })
        .await
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
        .run_on(target(&store, database)?, move |db| {
            db.recorded(format!("Edit a {table} record"), |c| records::update_record(c, &table, &key, &values))
        })
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
        .run_on(target(&store, database)?, move |db| {
            let label = match keys.len() {
                1 => format!("Delete a {table} record"),
                n => format!("Delete {n} {table} records"),
            };
            db.recorded(label, |c| records::delete_records(c, &table, &keys, confirm))
        })
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
    // What the change was, for Undo (e.g. "Paste 12 rows").
    label: Option<String>,
) -> AppResult<BatchResult> {
    on_online!(store, secrets, pools, database, |o| online_records::batch(&o, &table, &ops, confirm_deletes));
    state
        .run_on(target(&store, database)?, move |db| {
            let label = label.map(|l| l.chars().take(80).collect()).unwrap_or_else(|| format!("Change {} {table} records", ops.len()));
            db.recorded(label, |c| records::batch(c, &table, &ops, confirm_deletes))
        })
        .await
}

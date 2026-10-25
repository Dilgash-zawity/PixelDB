use serde_json::Value as JsonValue;
use tauri::State;

use super::{on_online, target};
use crate::online::SecretStore;
use crate::online_data::OnlinePools;
use crate::online_records;
use crate::database::{DbManager, queries};
use crate::errors::AppResult;
use crate::models::{QueryResult, SavedQuery};
use crate::workspace::WorkspaceStore;

#[tauri::command]
pub async fn run_query(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    pools: State<'_, OnlinePools>,
    database: Option<String>,
    sql: String,
    params: Option<serde_json::Map<String, JsonValue>>,
) -> AppResult<QueryResult> {
    on_online!(store, secrets, pools, database, |o| online_records::run_query(&o, &sql, &params.clone().unwrap_or_default()));
    let params = params.unwrap_or_default();
    state
        .run_on(target(&store, database)?, move |db| queries::run_query_with(&db.conn, &sql, &params))
        .await
}

#[tauri::command]
pub async fn save_query(
    state: State<'_, DbManager>,
    name: String,
    sql: String,
    design: Option<JsonValue>,
    replace: bool,
) -> AppResult<()> {
    state.run(move |db| queries::save_view(&db.conn, &name, &sql, design.as_ref(), replace)).await
}

#[tauri::command]
pub async fn get_query(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    pools: State<'_, OnlinePools>,
    database: Option<String>,
    name: String,
) -> AppResult<SavedQuery> {
    on_online!(store, secrets, pools, database, |o| online_records::saved_query(&o, &name));
    state.run_on(target(&store, database)?, move |db| queries::saved_query(&db.conn, &name)).await
}

#[tauri::command]
pub async fn drop_query(state: State<'_, DbManager>, name: String, confirm: bool) -> AppResult<()> {
    state.run(move |db| queries::drop_view(&db.conn, &name, confirm)).await
}

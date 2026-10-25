use tauri::State;

use super::{on_online, target};
use crate::online::SecretStore;
use crate::online_data::OnlinePools;
use crate::online_records;
use crate::database::{DbManager, metadata, schema};
use crate::errors::AppResult;
use crate::models::{DatabaseInfo, DiagramLayout, ObjectSummary};
use crate::workspace::WorkspaceStore;

/// Closes the current database. Returns false when none was open.
#[tauri::command]
pub async fn close_database(state: State<'_, DbManager>) -> AppResult<bool> {
    let mgr = state.inner().clone();
    DbManager::run_detached(move || mgr.close()).await
}

#[tauri::command]
pub async fn get_database_info(state: State<'_, DbManager>) -> AppResult<Option<DatabaseInfo>> {
    let mgr = state.inner().clone();
    DbManager::run_detached(move || match mgr.with_db(|db| db.info()) {
        Ok(info) => Ok(Some(info)),
        Err(crate::errors::AppError::NoDatabase) => Ok(None),
        Err(e) => Err(e),
    })
    .await
}

#[tauri::command]
pub async fn list_objects(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    pools: State<'_, OnlinePools>,
    database: Option<String>,
) -> AppResult<Vec<ObjectSummary>> {
    on_online!(store, secrets, pools, database, |o| online_records::list_objects(&o));
    state.run_on(target(&store, database)?, |db| schema::list_objects(&db.conn)).await
}

#[tauri::command]
pub async fn get_diagram_layout(state: State<'_, DbManager>) -> AppResult<DiagramLayout> {
    state.run(|db| metadata::load_layout(&db.conn)).await
}

#[tauri::command]
pub async fn save_diagram_layout(state: State<'_, DbManager>, layout: DiagramLayout) -> AppResult<()> {
    state.run(move |db| metadata::save_layout(&db.conn, &layout)).await
}

/// Closes databases opened by an app, keeping the database page's one.
#[tauri::command]
pub async fn release_databases(state: State<'_, DbManager>) -> AppResult<()> {
    let mgr = state.inner().clone();
    DbManager::run_detached(move || mgr.close_others()).await
}

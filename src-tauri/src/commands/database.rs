use tauri::State;

use crate::database::{DbManager, metadata, schema};
use crate::errors::AppResult;
use crate::models::{DatabaseInfo, DiagramLayout, ObjectSummary};

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
pub async fn list_objects(state: State<'_, DbManager>) -> AppResult<Vec<ObjectSummary>> {
    state.run(|db| schema::list_objects(&db.conn)).await
}

#[tauri::command]
pub async fn get_diagram_layout(state: State<'_, DbManager>) -> AppResult<DiagramLayout> {
    state.run(|db| metadata::load_layout(&db.conn)).await
}

#[tauri::command]
pub async fn save_diagram_layout(state: State<'_, DbManager>, layout: DiagramLayout) -> AppResult<()> {
    state.run(move |db| metadata::save_layout(&db.conn, &layout)).await
}

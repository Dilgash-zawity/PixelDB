use tauri::State;

use crate::database::{DbManager, schema};
use crate::errors::AppResult;
use crate::models::{DatabaseInfo, ObjectSummary};

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

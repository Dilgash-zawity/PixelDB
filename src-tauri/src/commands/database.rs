use tauri::State;

use crate::database::{DbManager, OpenDatabase, schema};
use crate::errors::AppResult;
use crate::models::{DatabaseInfo, ObjectSummary};

async fn activate(state: &DbManager, db: OpenDatabase) -> AppResult<DatabaseInfo> {
    let mgr = state.clone();
    DbManager::run_detached(move || {
        mgr.set(db)?;
        mgr.with_db(|db| db.info())
    })
    .await
}

/// Creates a new database file (never overwriting) and makes it current.
#[tauri::command]
pub async fn create_database(state: State<'_, DbManager>, path: String) -> AppResult<DatabaseInfo> {
    let db = DbManager::run_detached(move || OpenDatabase::create(&path)).await?;
    activate(&state, db).await
}

/// Opens an existing database file and makes it current.
#[tauri::command]
pub async fn open_database(state: State<'_, DbManager>, path: String) -> AppResult<DatabaseInfo> {
    let db = DbManager::run_detached(move || OpenDatabase::open(&path)).await?;
    activate(&state, db).await
}

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

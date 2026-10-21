use tauri::State;

use crate::database::{DbManager, OpenDatabase};
use crate::errors::AppResult;
use crate::models::DatabaseInfo;
use crate::workspace::{self, DatabaseFile, WorkspaceInfo, WorkspaceStore};

#[tauri::command]
pub async fn get_workspace(store: State<'_, WorkspaceStore>) -> AppResult<Option<WorkspaceInfo>> {
    Ok(store.get()?.map(|p| WorkspaceInfo { path: p.display().to_string() }))
}

/// Remembers the folder that holds the user's databases.
#[tauri::command]
pub async fn set_workspace(store: State<'_, WorkspaceStore>, path: String) -> AppResult<WorkspaceInfo> {
    let store = store.inner().clone();
    DbManager::run_detached(move || {
        let dir = store.set(&path)?;
        Ok(WorkspaceInfo { path: dir.display().to_string() })
    })
    .await
}

#[tauri::command]
pub async fn list_databases(store: State<'_, WorkspaceStore>) -> AppResult<Vec<DatabaseFile>> {
    let dir = store.require()?;
    DbManager::run_detached(move || workspace::list_databases(&dir)).await
}

/// Creates a database in the workspace and makes it current.
#[tauri::command]
pub async fn create_workspace_database(
    store: State<'_, WorkspaceStore>,
    db: State<'_, DbManager>,
    name: String,
) -> AppResult<DatabaseInfo> {
    let dir = store.require()?;
    let mgr = db.inner().clone();
    DbManager::run_detached(move || {
        let path = workspace::new_database_path(&dir, &name)?;
        let opened = OpenDatabase::create(&path.display().to_string())?;
        mgr.set(opened)?;
        mgr.with_db(|db| db.info())
    })
    .await
}

/// Opens a workspace database by name and makes it current. Reopening the
/// database that is already open keeps the existing connection.
#[tauri::command]
pub async fn open_workspace_database(
    store: State<'_, WorkspaceStore>,
    db: State<'_, DbManager>,
    name: String,
) -> AppResult<DatabaseInfo> {
    let dir = store.require()?;
    let mgr = db.inner().clone();
    DbManager::run_detached(move || {
        let path = workspace::resolve_database(&dir, &name)?;
        let canonical = std::fs::canonicalize(&path)?;
        let already_open = mgr.with_db(|db| Ok(db.path == canonical)).unwrap_or(false);
        if !already_open {
            mgr.set(OpenDatabase::open(&path.display().to_string())?)?;
        }
        mgr.with_db(|db| db.info())
    })
    .await
}

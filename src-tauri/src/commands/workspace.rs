use std::path::Path;

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

/// Closes the open database if it is the file at `path`, so the file can be
/// moved or removed.
fn close_if_open(mgr: &DbManager, path: &Path) -> AppResult<()> {
    let canonical = std::fs::canonicalize(path)?;
    if mgr.with_db(|db| Ok(db.path == canonical)).unwrap_or(false) {
        mgr.close()?;
    }
    Ok(())
}

#[tauri::command]
pub async fn rename_workspace_database(
    store: State<'_, WorkspaceStore>,
    db: State<'_, DbManager>,
    name: String,
    new_name: String,
) -> AppResult<()> {
    let dir = store.require()?;
    let mgr = db.inner().clone();
    DbManager::run_detached(move || {
        close_if_open(&mgr, &workspace::resolve_database(&dir, &name)?)?;
        workspace::rename_database(&dir, &name, &new_name).map(|_| ())
    })
    .await
}

/// Permanently deletes a database file; requires `confirm`.
#[tauri::command]
pub async fn delete_workspace_database(
    store: State<'_, WorkspaceStore>,
    db: State<'_, DbManager>,
    name: String,
    confirm: bool,
) -> AppResult<()> {
    let dir = store.require()?;
    let mgr = db.inner().clone();
    DbManager::run_detached(move || {
        if confirm {
            close_if_open(&mgr, &workspace::resolve_database(&dir, &name)?)?;
        }
        workspace::delete_database(&dir, &name, confirm)
    })
    .await
}

#[tauri::command]
pub async fn duplicate_workspace_database(
    store: State<'_, WorkspaceStore>,
    name: String,
    new_name: String,
) -> AppResult<()> {
    let dir = store.require()?;
    DbManager::run_detached(move || workspace::duplicate_database(&dir, &name, &new_name).map(|_| ())).await
}

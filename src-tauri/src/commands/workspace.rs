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
    let dir = store.databases()?;
    DbManager::run_detached(move || workspace::list_databases(&dir)).await
}

/// Creates a database in the workspace and makes it current.
#[tauri::command]
pub async fn create_workspace_database(
    store: State<'_, WorkspaceStore>,
    db: State<'_, DbManager>,
    name: String,
) -> AppResult<DatabaseInfo> {
    let dir = store.databases()?;
    let mgr = db.inner().clone();
    DbManager::run_detached(move || {
        let path = workspace::new_database_path(&dir, &name)?;
        let opened = OpenDatabase::create(&path.display().to_string())?;
        mgr.set(opened)?;
        mgr.with_db(|db| db.info())
    })
    .await
}

/// Opens a workspace database by name and makes it current. A database that
/// is already open (on the database page or by an app) keeps its connection.
#[tauri::command]
pub async fn open_workspace_database(
    store: State<'_, WorkspaceStore>,
    db: State<'_, DbManager>,
    name: String,
) -> AppResult<DatabaseInfo> {
    let dir = store.databases()?;
    let mgr = db.inner().clone();
    DbManager::run_detached(move || {
        mgr.open_as_current(&workspace::resolve_database(&dir, &name)?)?;
        mgr.with_db(|db| db.info())
    })
    .await
}

/// Closes the file at `path` if it is open, so it can be moved or removed.
fn close_if_open(mgr: &DbManager, path: &Path) -> AppResult<()> {
    mgr.close_path(&std::fs::canonicalize(path)?)
}

#[tauri::command]
pub async fn rename_workspace_database(
    store: State<'_, WorkspaceStore>,
    db: State<'_, DbManager>,
    name: String,
    new_name: String,
) -> AppResult<()> {
    let root = store.require()?;
    let dir = store.databases()?;
    let mgr = db.inner().clone();
    DbManager::run_detached(move || {
        close_if_open(&mgr, &workspace::resolve_database(&dir, &name)?)?;
        workspace::rename_database(&dir, &name, &new_name)?;
        // Apps address databases by name; keep them pointing at this one.
        crate::apps::rename_database_in_apps(&root, &name, &new_name)
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
    let dir = store.databases()?;
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
    let dir = store.databases()?;
    DbManager::run_detached(move || workspace::duplicate_database(&dir, &name, &new_name).map(|_| ())).await
}

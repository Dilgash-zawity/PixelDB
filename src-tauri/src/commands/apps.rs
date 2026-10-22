use serde_json::Value as JsonValue;
use tauri::State;

use crate::apps::{self, AppFile};
use crate::database::DbManager;
use crate::errors::AppResult;
use crate::workspace::WorkspaceStore;

#[tauri::command]
pub async fn list_apps(store: State<'_, WorkspaceStore>) -> AppResult<Vec<AppFile>> {
    let dir = store.require()?;
    DbManager::run_detached(move || apps::list_apps(&dir)).await
}

#[tauri::command]
pub async fn create_app(store: State<'_, WorkspaceStore>, name: String, definition: JsonValue) -> AppResult<()> {
    let dir = store.require()?;
    DbManager::run_detached(move || apps::create_app(&dir, &name, &definition)).await
}

#[tauri::command]
pub async fn load_app(store: State<'_, WorkspaceStore>, name: String) -> AppResult<JsonValue> {
    let dir = store.require()?;
    DbManager::run_detached(move || apps::load_app(&dir, &name)).await
}

#[tauri::command]
pub async fn save_app(store: State<'_, WorkspaceStore>, name: String, definition: JsonValue) -> AppResult<()> {
    let dir = store.require()?;
    DbManager::run_detached(move || apps::save_app(&dir, &name, &definition)).await
}

#[tauri::command]
pub async fn rename_app(store: State<'_, WorkspaceStore>, name: String, new_name: String) -> AppResult<()> {
    let dir = store.require()?;
    DbManager::run_detached(move || apps::rename_app(&dir, &name, &new_name)).await
}

/// Permanently deletes an app file (not its databases); requires `confirm`.
#[tauri::command]
pub async fn delete_app(store: State<'_, WorkspaceStore>, name: String, confirm: bool) -> AppResult<()> {
    let dir = store.require()?;
    DbManager::run_detached(move || apps::delete_app(&dir, &name, confirm)).await
}

#[tauri::command]
pub async fn duplicate_app(store: State<'_, WorkspaceStore>, name: String, new_name: String) -> AppResult<()> {
    let dir = store.require()?;
    DbManager::run_detached(move || apps::duplicate_app(&dir, &name, &new_name)).await
}

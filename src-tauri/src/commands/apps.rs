use serde_json::Value as JsonValue;
use tauri::State;

use crate::apps::{self, AppFile};
use crate::package;
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

/// Asks where to save, then writes app `name` and copies of its local
/// databases into one package file. The saved file name, or None when cancelled.
#[tauri::command]
pub async fn export_app_package(window: tauri::WebviewWindow, store: State<'_, WorkspaceStore>, name: String) -> AppResult<Option<String>> {
    use tauri_plugin_dialog::DialogExt;
    let dir = store.require()?;
    DbManager::run_detached(move || {
        let ext = package::PACKAGE_EXTENSION;
        let Some(file) = window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Share app")
            .set_file_name(format!("{name}.{ext}"))
            .add_filter("SlateDB app package", &[ext])
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let mut path = file.into_path().map_err(|e| crate::errors::AppError::InvalidPath(e.to_string()))?;
        if path.extension().is_none_or(|e| !e.eq_ignore_ascii_case(ext)) {
            path.set_extension(ext);
        }
        let bytes = package::write_package(&dir, &name)?;
        let tmp = path.with_extension(format!("{ext}.part"));
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, &path)?;
        Ok(path.file_name().map(|n| n.to_string_lossy().into_owned()))
    })
    .await
}

/// Asks for a package file and adds its app and databases to the workspace.
/// None when cancelled.
#[tauri::command]
pub async fn import_app_package(window: tauri::WebviewWindow, store: State<'_, WorkspaceStore>) -> AppResult<Option<package::ImportedPackage>> {
    use tauri_plugin_dialog::DialogExt;
    let dir = store.require()?;
    DbManager::run_detached(move || {
        let Some(file) = window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Open a shared app")
            .add_filter("SlateDB app package", &[package::PACKAGE_EXTENSION])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = file.into_path().map_err(|e| crate::errors::AppError::InvalidPath(e.to_string()))?;
        if std::fs::metadata(&path)?.len() > package::MAX_PACKAGE_BYTES {
            return Err(crate::errors::AppError::Validation("the package is too large".into()));
        }
        package::read_package(&dir, &std::fs::read(&path)?).map(Some)
    })
    .await
}

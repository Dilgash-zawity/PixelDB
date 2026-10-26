//! Diagnostics people can copy into a bug report: app and system facts,
//! and a log file of errors (kept in the app's log folder, never sent anywhere).

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::errors::{AppError, AppResult};

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostics {
    pub version: String,
    pub os: String,
    pub arch: String,
    pub sqlite: String,
}

#[tauri::command]
pub fn diagnostics(app: AppHandle) -> Diagnostics {
    Diagnostics {
        version: app.package_info().version.to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        sqlite: rusqlite::version().to_string(),
    }
}

/// Writes an error the frontend met to the log file.
#[tauri::command]
pub fn log_client_error(context: String, message: String) {
    let clip = |s: &str| s.chars().take(4000).collect::<String>();
    log::error!(target: "frontend", "{}: {}", clip(&context), clip(&message));
}

/// Opens the folder holding the log file in the system's file manager.
#[tauri::command]
pub fn open_log_folder(app: AppHandle) -> AppResult<()> {
    let dir = app.path().app_log_dir().map_err(|e| AppError::Io(e.to_string()))?;
    std::fs::create_dir_all(&dir)?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::Io(e.to_string()))
}

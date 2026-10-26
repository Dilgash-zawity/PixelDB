//! Import and export. Files are chosen with native dialogs opened here, so
//! the frontend never passes a file path.

use std::path::PathBuf;

use serde_json::Value as JsonValue;
use tauri::{State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use super::{on_online, target};
use crate::database::{DbManager, queries, records};
use crate::errors::{AppError, AppResult};
use crate::models::{ExportFormat, ExportResult, ImportFile, RecordQuery, RecordValues};
use crate::online::SecretStore;
use crate::online_data::OnlinePools;
use crate::online_records;
use crate::transfer::{self, MAX_EXPORT_ROWS};
use crate::workspace::WorkspaceStore;

const IMPORT_EXTENSIONS: &[&str] = &["csv", "tsv", "txt", "xlsx", "xlsm", "xlsb", "xls", "ods"];

fn path_of(file: tauri_plugin_dialog::FilePath) -> AppResult<PathBuf> {
    file.into_path().map_err(|e| AppError::InvalidPath(e.to_string()))
}

/// Asks for a CSV or spreadsheet file and reads it. None when cancelled.
#[tauri::command]
pub async fn pick_import_file(window: WebviewWindow) -> AppResult<Option<ImportFile>> {
    DbManager::run_detached(move || {
        let Some(file) = window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Import from a file")
            .add_filter("CSV or spreadsheet", IMPORT_EXTENSIONS)
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = path_of(file)?;
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let size = std::fs::metadata(&path)?.len() as usize;
        if size > transfer::MAX_IMPORT_BYTES {
            return Err(AppError::Validation(format!(
                "the file is larger than {} MB",
                transfer::MAX_IMPORT_BYTES / 1024 / 1024
            )));
        }
        let bytes = std::fs::read(&path)?;
        transfer::read_file(&name, &bytes).map(Some)
    })
    .await
}

/// Inserts imported records into `table` of the current database, all or none.
#[tauri::command]
pub async fn import_records(state: State<'_, DbManager>, table: String, rows: Vec<RecordValues>) -> AppResult<usize> {
    state
        .run(move |db| db.recorded(format!("Import {} {table} records", rows.len()), |c| records::import(c, &table, &rows)))
        .await
}

/// A file name without characters file systems refuse.
fn safe_file_name(name: &str, format: ExportFormat) -> String {
    let clean: String = name.chars().map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { '_' } else { c }).collect();
    let clean = clean.trim().trim_matches('.');
    format!("{}.{}", if clean.is_empty() { "export" } else { clean }, format.extension())
}

/// Asks where to save, then writes the file. None when cancelled.
async fn save_file(
    window: WebviewWindow,
    name: String,
    format: ExportFormat,
    columns: Vec<String>,
    types: Vec<Option<crate::models::FieldType>>,
    rows: Vec<Vec<JsonValue>>,
    truncated: bool,
) -> AppResult<Option<ExportResult>> {
    DbManager::run_detached(move || {
        let (label, ext) = match format {
            ExportFormat::Csv => ("CSV", "csv"),
            ExportFormat::Xlsx => ("Excel workbook", "xlsx"),
        };
        let Some(file) = window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Export")
            .set_file_name(safe_file_name(&name, format))
            .add_filter(label, &[ext])
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let mut path = path_of(file)?;
        if path.extension().is_none_or(|e| !e.eq_ignore_ascii_case(ext)) {
            path.set_extension(ext);
        }
        let bytes = transfer::write_rows(format, &name, &columns, &types, &rows)?;
        let tmp = path.with_extension(format!("{ext}.part"));
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, &path)?;
        let file_name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        Ok(Some(ExportResult { file_name, rows: rows.len(), truncated }))
    })
    .await
}

/// Exports the records of a table or view matching `query` (its paging is ignored).
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri passes each managed state separately.
pub async fn export_records(
    window: WebviewWindow,
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    pools: State<'_, OnlinePools>,
    database: Option<String>,
    table: String,
    query: RecordQuery,
    format: ExportFormat,
) -> AppResult<Option<ExportResult>> {
    let page = {
        let t = table.clone();
        let q = query.clone();
        let online = async {
            on_online!(store, secrets, pools, database, |o| async {
                // Page through the server: online tables are read 1000 rows at a time.
                let mut q = q.clone();
                q.limit = records::MAX_PAGE_SIZE;
                q.offset = 0;
                let mut page = online_records::list_records(&o, &t, &q).await?;
                while (page.rows.len() as i64) < page.total && page.rows.len() < MAX_EXPORT_ROWS {
                    q.offset = page.rows.len() as u64;
                    let next = online_records::list_records(&o, &t, &q).await?;
                    if next.rows.is_empty() {
                        break;
                    }
                    page.rows.extend(next.rows);
                }
                Ok(Some(page))
            });
            Ok::<_, AppError>(None)
        };
        match online.await? {
            Some(p) => p,
            None => {
                state
                    .run_on(target(&store, database.clone())?, move |db| {
                        records::list_all(&db.conn, &t, &q, MAX_EXPORT_ROWS as u64)
                    })
                    .await?
            }
        }
    };
    let truncated = (page.rows.len() as i64) < page.total;
    let rows = page.rows.into_iter().map(|r| r.values).collect();
    save_file(window, table, format, page.columns, page.types, rows, truncated).await
}

/// Exports the result of a read-only query, `name` naming the file.
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri passes each managed state separately.
pub async fn export_query(
    window: WebviewWindow,
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    pools: State<'_, OnlinePools>,
    database: Option<String>,
    sql: String,
    params: Option<serde_json::Map<String, JsonValue>>,
    name: String,
    format: ExportFormat,
) -> AppResult<Option<ExportResult>> {
    let params = params.unwrap_or_default();
    let result = {
        let (s, p) = (sql.clone(), params.clone());
        let online = async {
            on_online!(store, secrets, pools, database, |o| async {
                online_records::run_query(&o, &s, &p).await.map(Some)
            });
            Ok::<_, AppError>(None)
        };
        match online.await? {
            Some(r) => r,
            None => {
                state
                    .run_on(target(&store, database.clone())?, move |db| {
                        queries::run_query_capped(&db.conn, &sql, &params, MAX_EXPORT_ROWS)
                    })
                    .await?
            }
        }
    };
    save_file(window, name, format, result.columns, result.types, result.rows, result.truncated).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_are_made_safe() {
        assert_eq!(safe_file_name("Orders: 2024/Q1", ExportFormat::Csv), "Orders_ 2024_Q1.csv");
        assert_eq!(safe_file_name("..", ExportFormat::Xlsx), "export.xlsx");
    }
}

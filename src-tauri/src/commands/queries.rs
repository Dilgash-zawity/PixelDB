use tauri::State;

use crate::database::{DbManager, queries};
use crate::errors::AppResult;
use crate::models::QueryResult;

#[tauri::command]
pub async fn run_query(state: State<'_, DbManager>, sql: String) -> AppResult<QueryResult> {
    state.run(move |db| queries::run_query(&db.conn, &sql)).await
}

#[tauri::command]
pub async fn save_query(state: State<'_, DbManager>, name: String, sql: String, replace: bool) -> AppResult<()> {
    state.run(move |db| queries::save_view(&db.conn, &name, &sql, replace)).await
}

#[tauri::command]
pub async fn get_query_sql(state: State<'_, DbManager>, name: String) -> AppResult<String> {
    state.run(move |db| queries::view_sql(&db.conn, &name)).await
}

#[tauri::command]
pub async fn drop_query(state: State<'_, DbManager>, name: String, confirm: bool) -> AppResult<()> {
    state.run(move |db| queries::drop_view(&db.conn, &name, confirm)).await
}

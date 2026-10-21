use tauri::State;

use crate::database::{DbManager, migrations, schema};
use crate::errors::AppResult;
use crate::models::{FieldDef, SchemaChangeResult, TableDef, TableDesign, TableInfo};

#[tauri::command]
pub async fn get_table(state: State<'_, DbManager>, name: String) -> AppResult<TableInfo> {
    state.run(move |db| schema::table_info(&db.conn, &name)).await
}

#[tauri::command]
pub async fn create_table(state: State<'_, DbManager>, definition: TableDef) -> AppResult<TableInfo> {
    state
        .run(move |db| {
            let def = migrations::create_table(&db.conn, &definition)?;
            schema::table_info(&db.conn, &def.name)
        })
        .await
}

#[tauri::command]
pub async fn rename_table(state: State<'_, DbManager>, name: String, new_name: String) -> AppResult<()> {
    state.run(move |db| migrations::rename_table(&db.conn, &name, &new_name)).await
}

#[tauri::command]
pub async fn drop_table(state: State<'_, DbManager>, name: String, confirm: bool) -> AppResult<SchemaChangeResult> {
    state.run(move |db| migrations::drop_table(&db.conn, &name, confirm)).await
}

#[tauri::command]
pub async fn save_table_design(
    state: State<'_, DbManager>,
    name: String,
    design: TableDesign,
    confirm_data_loss: bool,
) -> AppResult<SchemaChangeResult> {
    state
        .run(move |db| migrations::save_table_design(&db.conn, &name, &design, confirm_data_loss))
        .await
}

#[tauri::command]
pub async fn add_field(
    state: State<'_, DbManager>,
    table: String,
    field: FieldDef,
    position: Option<usize>,
) -> AppResult<SchemaChangeResult> {
    state.run(move |db| migrations::add_field(&db.conn, &table, field, position)).await
}

#[tauri::command]
pub async fn modify_field(
    state: State<'_, DbManager>,
    table: String,
    name: String,
    field: FieldDef,
) -> AppResult<SchemaChangeResult> {
    state.run(move |db| migrations::modify_field(&db.conn, &table, &name, field)).await
}

#[tauri::command]
pub async fn rename_field(state: State<'_, DbManager>, table: String, name: String, new_name: String) -> AppResult<()> {
    state.run(move |db| migrations::rename_field(&db.conn, &table, &name, &new_name)).await
}

#[tauri::command]
pub async fn drop_field(
    state: State<'_, DbManager>,
    table: String,
    name: String,
    confirm: bool,
) -> AppResult<SchemaChangeResult> {
    state.run(move |db| migrations::drop_field(&db.conn, &table, &name, confirm)).await
}

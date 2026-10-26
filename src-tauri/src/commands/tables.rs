use tauri::State;

use super::{on_online, target};
use crate::online::SecretStore;
use crate::online_data::OnlinePools;
use crate::online_records;
use crate::database::{DbManager, indexes, migrations, schema};
use crate::errors::AppResult;
use crate::models::{FieldDef, IndexInfo, SchemaChangeResult, TableDef, TableDesign, TableInfo};
use crate::workspace::WorkspaceStore;

#[tauri::command]
pub async fn get_table(
    state: State<'_, DbManager>,
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    pools: State<'_, OnlinePools>,
    database: Option<String>,
    name: String,
) -> AppResult<TableInfo> {
    on_online!(store, secrets, pools, database, |o| online_records::table_info(&o, &name));
    state.run_on(target(&store, database)?, move |db| schema::table_info(&db.conn, &name)).await
}

#[tauri::command]
pub async fn create_table(state: State<'_, DbManager>, definition: TableDef) -> AppResult<TableInfo> {
    state
        .run(move |db| {
            let def = db.restructured(|c| migrations::create_table(c, &definition))?;
            schema::table_info(&db.conn, &def.name)
        })
        .await
}

#[tauri::command]
pub async fn rename_table(state: State<'_, DbManager>, name: String, new_name: String) -> AppResult<()> {
    state.run(move |db| db.restructured(|c| migrations::rename_table(c, &name, &new_name))).await
}

#[tauri::command]
pub async fn drop_table(state: State<'_, DbManager>, name: String, confirm: bool) -> AppResult<SchemaChangeResult> {
    state.run(move |db| db.restructured(|c| migrations::drop_table(c, &name, confirm))).await
}

#[tauri::command]
pub async fn save_table_design(
    state: State<'_, DbManager>,
    name: String,
    design: TableDesign,
    confirm_data_loss: bool,
) -> AppResult<SchemaChangeResult> {
    state
        .run(move |db| db.restructured(|c| migrations::save_table_design(c, &name, &design, confirm_data_loss)))
        .await
}

#[tauri::command]
pub async fn add_field(
    state: State<'_, DbManager>,
    table: String,
    field: FieldDef,
    position: Option<usize>,
) -> AppResult<SchemaChangeResult> {
    state.run(move |db| db.restructured(|c| migrations::add_field(c, &table, field, position))).await
}

#[tauri::command]
pub async fn modify_field(
    state: State<'_, DbManager>,
    table: String,
    name: String,
    field: FieldDef,
) -> AppResult<SchemaChangeResult> {
    state.run(move |db| db.restructured(|c| migrations::modify_field(c, &table, &name, field))).await
}

#[tauri::command]
pub async fn rename_field(state: State<'_, DbManager>, table: String, name: String, new_name: String) -> AppResult<()> {
    state.run(move |db| db.restructured(|c| migrations::rename_field(c, &table, &name, &new_name))).await
}

#[tauri::command]
pub async fn drop_field(
    state: State<'_, DbManager>,
    table: String,
    name: String,
    confirm: bool,
) -> AppResult<SchemaChangeResult> {
    state.run(move |db| db.restructured(|c| migrations::drop_field(c, &table, &name, confirm))).await
}

#[tauri::command]
pub async fn create_index(
    state: State<'_, DbManager>,
    table: String,
    columns: Vec<String>,
    unique: bool,
) -> AppResult<IndexInfo> {
    state.run(move |db| indexes::create_index(&db.conn, &table, &columns, unique)).await
}

#[tauri::command]
pub async fn drop_index(state: State<'_, DbManager>, table: String, name: String) -> AppResult<()> {
    state.run(move |db| indexes::drop_index(&db.conn, &table, &name)).await
}

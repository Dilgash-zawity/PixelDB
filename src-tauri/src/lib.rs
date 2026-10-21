pub mod commands;
pub mod database;
pub mod errors;
pub mod models;
pub mod validation;
pub mod workspace;

use database::DbManager;
use tauri::Manager;
use workspace::WorkspaceStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .manage(DbManager::default())
    .setup(|app| {
      let config_dir = app.path().app_config_dir()?;
      app.manage(WorkspaceStore::load(config_dir.join("settings.json")));
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::workspace::get_workspace,
      commands::workspace::set_workspace,
      commands::workspace::list_databases,
      commands::workspace::create_workspace_database,
      commands::workspace::open_workspace_database,
      commands::database::close_database,
      commands::database::get_database_info,
      commands::database::list_objects,
      commands::tables::get_table,
      commands::tables::create_table,
      commands::tables::rename_table,
      commands::tables::drop_table,
      commands::tables::save_table_design,
      commands::tables::add_field,
      commands::tables::modify_field,
      commands::tables::rename_field,
      commands::tables::drop_field,
      commands::records::list_records,
      commands::records::get_record,
      commands::records::get_attachment,
      commands::records::insert_record,
      commands::records::update_record,
      commands::records::delete_records,
      commands::records::batch_records,
    ])
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}

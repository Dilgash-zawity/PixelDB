pub mod apps;
pub mod commands;
pub mod database;
pub mod errors;
pub mod models;
pub mod online;
pub mod online_data;
pub mod online_records;
#[cfg(target_os = "linux")]
mod touchpad;
pub mod validation;
pub mod workspace;

use database::DbManager;
use tauri::Manager;
use workspace::WorkspaceStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_opener::init())
    .manage(DbManager::default())
    .setup(|app| {
      #[cfg(desktop)]
      {
        app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
        app.handle().plugin(tauri_plugin_process::init())?;
      }
      let config_dir = app.path().app_config_dir()?;
      app.manage(WorkspaceStore::load(config_dir.join("settings.json")));
      app.manage(online::SecretStore::new(config_dir.join("online-secrets.json")));
      app.manage(online_data::OnlinePools::default());
      #[cfg(target_os = "linux")]
      for window in app.webview_windows().values() {
        if let Err(e) = touchpad::forward_pinches(window) {
          log::warn!("touchpad pinch zoom unavailable: {e}");
        }
      }
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
      commands::workspace::rename_workspace_database,
      commands::workspace::delete_workspace_database,
      commands::workspace::duplicate_workspace_database,
      commands::online::list_online_databases,
      commands::online::create_online_database,
      commands::online::update_online_database,
      commands::online::delete_online_database,
      commands::online::test_online_database,
      commands::online::online_database_info,
      commands::online::describe_online_table,
      commands::online::explain_online_query,
      commands::apps::list_apps,
      commands::apps::create_app,
      commands::apps::load_app,
      commands::apps::save_app,
      commands::apps::rename_app,
      commands::apps::delete_app,
      commands::apps::duplicate_app,
      commands::database::close_database,
      commands::database::release_databases,
      commands::database::get_database_info,
      commands::database::list_objects,
      commands::database::get_diagram_layout,
      commands::database::save_diagram_layout,
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
      commands::records::aggregate_records,
      commands::records::get_record,
      commands::records::get_attachment,
      commands::records::insert_record,
      commands::records::update_record,
      commands::records::delete_records,
      commands::records::batch_records,
      commands::queries::run_query,
      commands::queries::save_query,
      commands::queries::get_query,
      commands::queries::drop_query,
    ])
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}

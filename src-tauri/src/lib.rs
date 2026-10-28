pub mod apps;
pub mod commands;
pub mod database;
pub mod errors;
pub mod models;
pub mod online;
pub mod online_data;
pub mod online_records;
pub mod package;
#[cfg(target_os = "linux")]
mod touchpad;
pub mod transfer;
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
      touchpad::forward_pinches(app.webview_windows().into_values().collect());
      // Errors go to a small rotating file in the app's log folder (Settings →
      // About → Open log folder), and to the terminal while developing.
      let mut log = tauri_plugin_log::Builder::default()
        .clear_targets()
        .target(tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
          file_name: Some("pixeldb".into()),
        }))
        .max_file_size(1_000_000)
        .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
        .level(if cfg!(debug_assertions) { log::LevelFilter::Info } else { log::LevelFilter::Warn })
        .level_for("sqlx", log::LevelFilter::Error);
      if cfg!(debug_assertions) {
        log = log.target(tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout));
      }
      app.handle().plugin(log.build())?;
      log::info!("PixelDB {} on {} {}", app.package_info().version, std::env::consts::OS, std::env::consts::ARCH);
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
      commands::apps::export_app_package,
      commands::apps::import_app_package,
      commands::diagnostics::diagnostics,
      commands::diagnostics::log_client_error,
      commands::diagnostics::open_log_folder,
      commands::database::close_database,
      commands::database::release_databases,
      commands::database::get_database_info,
      commands::database::list_objects,
      commands::database::get_diagram_layout,
      commands::database::save_diagram_layout,
      commands::database::get_document,
      commands::database::save_document,
      commands::tables::get_table,
      commands::tables::create_table,
      commands::tables::rename_table,
      commands::tables::drop_table,
      commands::tables::save_table_design,
      commands::tables::add_field,
      commands::tables::modify_field,
      commands::tables::rename_field,
      commands::tables::drop_field,
      commands::tables::create_index,
      commands::tables::drop_index,
      commands::transfer::pick_import_file,
      commands::transfer::import_records,
      commands::transfer::export_records,
      commands::transfer::export_query,
      commands::records::list_records,
      commands::records::aggregate_records,
      commands::records::get_record,
      commands::records::get_attachment,
      commands::records::insert_record,
      commands::records::update_record,
      commands::records::delete_records,
      commands::records::batch_records,
      commands::records::undo_state,
      commands::records::undo_records,
      commands::records::redo_records,
      commands::queries::run_query,
      commands::queries::save_query,
      commands::queries::get_query,
      commands::queries::drop_query,
    ])
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}

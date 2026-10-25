use tauri::State;

use crate::database::DbManager;
use crate::errors::AppResult;
use crate::online::{self, ConnectionSettings, ConnectionTest, OnlineDatabase, SecretStore};
use crate::online_data::{OnlinePools, OnlineRows};
use crate::online_records::{self, Online, OnlineInfo, OnlineTable};
use crate::workspace::WorkspaceStore;

#[tauri::command]
pub async fn list_online_databases(
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
) -> AppResult<Vec<OnlineDatabase>> {
    let dir = store.databases()?;
    let secrets = secrets.inner().clone();
    DbManager::run_detached(move || online::list(&dir, &secrets)).await
}

/// Saves a new connection; `secret` is the password or token.
#[tauri::command]
pub async fn create_online_database(
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    name: String,
    settings: ConnectionSettings,
    secret: Option<String>,
) -> AppResult<()> {
    let dir = store.databases()?;
    let secrets = secrets.inner().clone();
    DbManager::run_detached(move || online::create(&dir, &secrets, &name, &settings, secret.as_deref())).await
}

/// Changes and possibly renames a connection. `secret`: omitted keeps the
/// saved one, empty forgets it.
#[tauri::command]
pub async fn update_online_database(
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    name: String,
    new_name: String,
    settings: ConnectionSettings,
    secret: Option<String>,
) -> AppResult<()> {
    let root = store.require()?;
    let dir = store.databases()?;
    let secrets = secrets.inner().clone();
    DbManager::run_detached(move || {
        online::update(&dir, &secrets, &name, &new_name, &settings, secret.as_deref())?;
        if name != new_name {
            crate::apps::rename_database_in_apps(&root, &name, &new_name)?;
        }
        Ok(())
    })
    .await
}

/// Forgets a connection; requires `confirm`. The server is not touched.
#[tauri::command]
pub async fn delete_online_database(
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    name: String,
    confirm: bool,
) -> AppResult<()> {
    let dir = store.databases()?;
    let secrets = secrets.inner().clone();
    DbManager::run_detached(move || online::delete(&dir, &secrets, &name, confirm)).await
}

/// Tries a connection. Without `secret`, uses the one saved for `name`.
#[tauri::command]
pub async fn test_online_database(
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    settings: ConnectionSettings,
    secret: Option<String>,
    name: Option<String>,
) -> AppResult<ConnectionTest> {
    let secret = match (secret, name) {
        (Some(s), _) => Some(s),
        (None, Some(name)) => secrets.get(&store.databases()?, &name)?,
        (None, None) => None,
    };
    online::test(&settings, secret.as_deref()).await
}

/// The settings and secret of the saved connection `name`.
fn saved(store: &WorkspaceStore, secrets: &SecretStore, name: &str) -> AppResult<(ConnectionSettings, String)> {
    let dir = store.databases()?;
    let settings = online::settings(&dir, name)?;
    Ok((settings, secrets.get(&dir, name)?.unwrap_or_default()))
}

/// Server version, size and object counts of the online database `name`.
#[tauri::command]
pub async fn online_database_info(
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    pools: State<'_, OnlinePools>,
    name: String,
) -> AppResult<OnlineInfo> {
    let (settings, secret) = saved(&store, &secrets, &name)?;
    online_records::info(&Online::new(&pools, &settings, &secret)).await
}

/// A table's or view's columns with their server types, keys and links.
#[tauri::command]
pub async fn describe_online_table(
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    pools: State<'_, OnlinePools>,
    name: String,
    table: String,
) -> AppResult<OnlineTable> {
    let (settings, secret) = saved(&store, &secrets, &name)?;
    let t = Online::new(&pools, &settings, &secret).describe(&table).await?;
    Ok((*t).clone())
}

/// How the server would run a query; `analyze` (Postgres) runs it read-only to time it.
#[tauri::command]
pub async fn explain_online_query(
    store: State<'_, WorkspaceStore>,
    secrets: State<'_, SecretStore>,
    pools: State<'_, OnlinePools>,
    name: String,
    sql: String,
    analyze: bool,
) -> AppResult<OnlineRows> {
    let (settings, secret) = saved(&store, &secrets, &name)?;
    online_records::explain(&Online::new(&pools, &settings, &secret), &sql, analyze).await
}

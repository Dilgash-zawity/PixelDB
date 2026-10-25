use tauri::State;

use crate::database::DbManager;
use crate::errors::AppResult;
use crate::online::{self, ConnectionSettings, ConnectionTest, OnlineDatabase, SecretStore};
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

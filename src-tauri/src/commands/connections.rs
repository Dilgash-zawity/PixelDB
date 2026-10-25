use tauri::State;

use crate::connections::ConnectionStore;
use crate::database::DbManager;
use crate::errors::{AppError, AppResult};
use crate::models::{Connection, ConnectionInfo, RemoteResult, RemoteTable, ServerInfo};
use crate::remote;
use crate::workspace::{self, WorkspaceStore};

/// Runs network work on a blocking thread with its own wait on the runtime.
/// sqlx's futures can't be proven `Send` inside a command, so they never
/// cross threads: they are driven to completion where they start.
async fn online<T, Fut>(work: impl FnOnce() -> Fut + Send + 'static) -> AppResult<T>
where
    T: Send + 'static,
    Fut: std::future::Future<Output = AppResult<T>>,
{
    DbManager::run_detached(move || tauri::async_runtime::block_on(work())).await
}

#[tauri::command]
pub async fn list_connections(store: State<'_, ConnectionStore>) -> AppResult<Vec<ConnectionInfo>> {
    store.list()
}

/// Connects with the details as entered (an empty password or token falls
/// back to the saved connection `previous`) and reports the server version.
#[tauri::command]
pub async fn test_connection(
    store: State<'_, ConnectionStore>,
    connection: Connection,
    previous: Option<String>,
) -> AppResult<ServerInfo> {
    let conn = store.with_saved_secrets(connection, previous.as_deref())?;
    crate::connections::validate(&conn)?;
    online(move || async move { remote::test(&conn).await }).await
}

/// Saves a new connection, or replaces (and maybe renames) `previous`.
#[tauri::command]
pub async fn save_connection(
    store: State<'_, ConnectionStore>,
    workspace_store: State<'_, WorkspaceStore>,
    connection: Connection,
    previous: Option<String>,
) -> AppResult<ConnectionInfo> {
    // Local and online databases share the home map, so names are unique across both.
    if let Some(dir) = workspace_store.get()? {
        let name = connection.name.to_lowercase();
        let local = DbManager::run_detached(move || workspace::list_databases(&dir)).await?;
        if local.iter().any(|d| d.name.to_lowercase() == name) {
            return Err(AppError::AlreadyExists(format!("a database named {:?}", connection.name)));
        }
    }
    store.save(connection, previous.as_deref())
}

/// Forgets a connection; nothing on the server changes. Requires `confirm`.
#[tauri::command]
pub async fn delete_connection(store: State<'_, ConnectionStore>, name: String, confirm: bool) -> AppResult<()> {
    store.delete(&name, confirm)
}

#[tauri::command]
pub async fn remote_tables(store: State<'_, ConnectionStore>, name: String) -> AppResult<Vec<RemoteTable>> {
    let conn = store.get(&name)?;
    online(move || async move { remote::list_tables(&conn).await }).await
}

#[tauri::command]
pub async fn remote_preview(
    store: State<'_, ConnectionStore>,
    name: String,
    schema: Option<String>,
    table: String,
) -> AppResult<RemoteResult> {
    let conn = store.get(&name)?;
    online(move || async move { remote::preview(&conn, schema.as_deref(), &table).await }).await
}

/// Runs one read-only statement on an online database.
#[tauri::command]
pub async fn remote_query(store: State<'_, ConnectionStore>, name: String, sql: String) -> AppResult<RemoteResult> {
    let conn = store.get(&name)?;
    online(move || async move { remote::query(&conn, &sql).await }).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Provider, TlsMode};
    use serde_json::{Value as JsonValue, json};

    fn unreachable(provider: Provider) -> Connection {
        Connection {
            name: "x".into(),
            provider,
            // Port 1 on loopback refuses at once.
            host: "127.0.0.1".into(),
            port: Some(1),
            database: "db".into(),
            user: "u".into(),
            password: "p".into(),
            url: "http://127.0.0.1:1".into(),
            token: String::new(),
            tls: TlsMode::Disable,
            trust_certificate: false,
        }
    }

    #[test]
    fn refused_connections_report_errors() {
        for provider in [Provider::Postgres, Provider::Mysql, Provider::SqlServer, Provider::Sqlite] {
            let conn = unreachable(provider);
            let result = tauri::async_runtime::block_on(online(move || async move { remote::test(&conn).await }));
            assert!(matches!(result, Err(AppError::Remote(_))), "{provider:?}: {result:?}");
        }
    }

    /// Against a real server: `SLATEDB_TEST_PG=host:port:user:password:database cargo test -- --ignored live_postgres`.
    #[test]
    #[ignore]
    fn live_postgres() {
        let spec = std::env::var("SLATEDB_TEST_PG").expect("set SLATEDB_TEST_PG");
        let p: Vec<&str> = spec.split(':').collect();
        let conn = Connection {
            host: p[0].into(),
            port: Some(p[1].parse().unwrap()),
            user: p[2].into(),
            password: p[3].into(),
            database: p[4].into(),
            tls: TlsMode::Prefer,
            ..unreachable(Provider::Postgres)
        };
        let c = conn.clone();
        let info = tauri::async_runtime::block_on(online(move || async move { remote::test(&c).await })).unwrap();
        assert!(info.version.starts_with("PostgreSQL"), "{}", info.version);

        let c = conn.clone();
        let tables = tauri::async_runtime::block_on(online(move || async move { remote::list_tables(&c).await })).unwrap();
        println!("{tables:?}");
        assert!(tables.iter().any(|t| t.name == "orders" && t.schema.as_deref() == Some("public")));
        assert!(tables.iter().any(|t| t.name == "big" && t.kind == crate::models::ObjectKind::View));

        let c = conn.clone();
        let rows = tauri::async_runtime::block_on(online(move || async move { remote::preview(&c, Some("public"), "orders").await })).unwrap();
        println!("{:?}\n{:?}", rows.columns, rows.rows);
        assert_eq!(rows.columns[..5], ["id", "item", "qty", "price", "paid"]);
        assert_eq!(rows.rows[0][..5], [json!(1), json!("Tea"), json!(2), json!("4.50"), json!(true)]);
        assert_eq!(rows.rows[2][1], JsonValue::Null);

        // Writes are refused up front, and a write hidden in a CTE hits the read-only transaction.
        let c = conn.clone();
        let refused = tauri::async_runtime::block_on(online(move || async move { remote::query(&c, "DELETE FROM orders").await }));
        assert!(matches!(refused, Err(AppError::Unsupported(_))));
        let c = conn.clone();
        let hidden = tauri::async_runtime::block_on(online(move || async move { remote::query(&c, "WITH d AS (DELETE FROM orders RETURNING *) SELECT * FROM d").await }));
        assert!(matches!(&hidden, Err(AppError::Remote(m)) if m.contains("read-only")), "{hidden:?}");
        let c = conn.clone();
        let count = tauri::async_runtime::block_on(online(move || async move { remote::query(&c, "SELECT count(*) AS n FROM sales.big").await })).unwrap();
        assert_eq!(count.rows, vec![vec![json!(2)]]);

        let wrong = Connection { password: "nope".into(), ..conn };
        let err = tauri::async_runtime::block_on(online(move || async move { remote::test(&wrong).await }));
        println!("{err:?}");
        assert!(matches!(err, Err(AppError::Remote(_))));
    }
}

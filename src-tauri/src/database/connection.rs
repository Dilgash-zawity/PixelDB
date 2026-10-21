use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};

use crate::errors::{AppError, AppResult};
use crate::validation::paths::{validate_existing_database_path, validate_new_database_path};

use crate::models::{DatabaseInfo, ObjectKind};

use super::{metadata, schema};

/// `PRAGMA application_id` written to databases created by this application.
pub const APPLICATION_ID: i32 = 0x4D53_4143; // "MSAC"

pub struct OpenDatabase {
    pub path: PathBuf,
    pub conn: Connection,
}

impl OpenDatabase {
    /// Creates a new database file. Fails if the file already exists.
    pub fn create(raw_path: &str) -> AppResult<Self> {
        let path = validate_new_database_path(raw_path)?;
        // create_new makes the existence check atomic: an existing file is
        // never truncated or reused.
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::AlreadyExists => AppError::AlreadyExists(format!(
                    "{}: refusing to overwrite an existing file",
                    path.display()
                )),
                _ => AppError::Io(format!("{}: {e}", path.display())),
            })?;
        let init = || -> AppResult<OpenDatabase> {
            let conn = open_connection(&path)?;
            conn.pragma_update(None, "application_id", APPLICATION_ID)?;
            metadata::ensure(&conn)?;
            Ok(OpenDatabase { path: path.clone(), conn })
        };
        init().inspect_err(|_| {
            // Only the empty file created above is removed.
            let _ = std::fs::remove_file(&path);
        })
    }

    /// Opens an existing SQLite database file.
    pub fn open(raw_path: &str) -> AppResult<Self> {
        let path = validate_existing_database_path(raw_path)?;
        let conn = open_connection(&path)?;
        // Forces SQLite to read the schema, detecting corrupt or encrypted files.
        conn.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get::<_, i64>(0))
            .map_err(|e| AppError::InvalidDatabase(format!("{}: {e}", path.display())))?;
        Ok(OpenDatabase { path, conn })
    }

    pub fn info(&self) -> AppResult<DatabaseInfo> {
        let c = &self.conn;
        let pragma_i64 = |name: &str| -> AppResult<i64> {
            Ok(c.pragma_query_value(None, name, |r| r.get(0))?)
        };
        let pragma_str = |name: &str| -> AppResult<String> {
            Ok(c.pragma_query_value(None, name, |r| r.get(0))?)
        };
        let objects = schema::list_objects(c)?;
        Ok(DatabaseInfo {
            path: self.path.display().to_string(),
            file_size: std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0),
            sqlite_version: rusqlite::version().to_string(),
            page_size: pragma_i64("page_size")?,
            page_count: pragma_i64("page_count")?,
            encoding: pragma_str("encoding")?,
            journal_mode: pragma_str("journal_mode")?,
            user_version: pragma_i64("user_version")?,
            application_id: pragma_i64("application_id")?,
            foreign_keys: pragma_i64("foreign_keys")? == 1,
            table_count: objects.iter().filter(|o| o.kind == ObjectKind::Table).count(),
            view_count: objects.iter().filter(|o| o.kind == ObjectKind::View).count(),
        })
    }

    pub fn close(self) -> AppResult<()> {
        self.conn.close().map_err(|(_, e)| AppError::from(e))
    }
}

fn open_connection(path: &PathBuf) -> AppResult<Connection> {
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(path, flags)
        .map_err(|e| AppError::InvalidDatabase(format!("{}: {e}", path.display())))?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.pragma_update(None, "foreign_keys", true)?;
    // Schema-embedded SQL may only call functions marked innocuous.
    conn.pragma_update(None, "trusted_schema", false)?;
    let fk: bool = conn.pragma_query_value(None, "foreign_keys", |r| r.get(0))?;
    if !fk {
        return Err(AppError::Internal("could not enable foreign key enforcement".into()));
    }
    Ok(conn)
}

/// Holds the single open database. Cloning shares the same state.
#[derive(Clone, Default)]
pub struct DbManager {
    inner: Arc<Mutex<Option<OpenDatabase>>>,
}

impl DbManager {
    fn lock(&self) -> AppResult<std::sync::MutexGuard<'_, Option<OpenDatabase>>> {
        self.inner
            .lock()
            .map_err(|_| AppError::Internal("database state lock poisoned".into()))
    }

    /// Makes `db` the open database, closing the previous one.
    pub fn set(&self, db: OpenDatabase) -> AppResult<()> {
        let previous = self.lock()?.replace(db);
        if let Some(prev) = previous {
            prev.close()?;
        }
        Ok(())
    }

    /// Closes the open database. Returns false when none was open.
    pub fn close(&self) -> AppResult<bool> {
        let previous = self.lock()?.take();
        match previous {
            Some(db) => db.close().map(|_| true),
            None => Ok(false),
        }
    }

    pub fn with_db<T>(&self, f: impl FnOnce(&mut OpenDatabase) -> AppResult<T>) -> AppResult<T> {
        let mut guard = self.lock()?;
        let db = guard.as_mut().ok_or(AppError::NoDatabase)?;
        f(db)
    }

    /// Runs blocking database work off the async runtime's worker threads.
    pub async fn run<T, F>(&self, f: F) -> AppResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut OpenDatabase) -> AppResult<T> + Send + 'static,
    {
        let this = self.clone();
        tauri::async_runtime::spawn_blocking(move || this.with_db(f))
            .await
            .map_err(|e| AppError::Internal(format!("background task failed: {e}")))?
    }

    /// Runs blocking work that does not need the open database.
    pub async fn run_detached<T, F>(f: F) -> AppResult<T>
    where
        T: Send + 'static,
        F: FnOnce() -> AppResult<T> + Send + 'static,
    {
        tauri::async_runtime::spawn_blocking(f)
            .await
            .map_err(|e| AppError::Internal(format!("background task failed: {e}")))?
    }
}

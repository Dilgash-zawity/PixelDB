use std::fs::OpenOptions;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use rusqlite::functions::FunctionFlags;
use rusqlite::{Connection, OpenFlags};

use crate::errors::{AppError, AppResult};
use crate::validation::paths::{validate_existing_database_path, validate_new_database_path};

use crate::models::{DatabaseInfo, ObjectKind};

use super::journal::Journal;
use super::{metadata, schema};

/// `PRAGMA application_id` written to databases created by this application.
pub const APPLICATION_ID: i32 = 0x4D53_4143; // "MSAC"

pub struct OpenDatabase {
    pub path: PathBuf,
    pub conn: Connection,
    /// Record changes that can be undone (structure changes clear it).
    pub journal: Journal,
}

impl OpenDatabase {
    /// Runs a record change, keeping it as one undo step called `label`.
    pub fn recorded<T>(&mut self, label: impl Into<String>, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        super::journal::record(&self.conn, &mut self.journal, label, f)
    }

    /// Runs a structure change; once it succeeds, earlier record changes can no longer be undone.
    pub fn restructured<T>(&mut self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let out = f(&self.conn)?;
        self.journal.clear();
        Ok(out)
    }

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
            Ok(OpenDatabase { path: path.clone(), conn, journal: Journal::default() })
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
        Ok(OpenDatabase { path, conn, journal: Journal::default() })
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
    register_functions(&conn)?;
    Ok(conn)
}

/// Name of the Unicode lower-casing function search and text filters use.
/// SQLite's own `lower()` and `LIKE` only fold ASCII letters, so "é" would
/// not find "É". Only used in queries the app builds, never stored in the
/// schema, so the file still opens in other SQLite tools.
pub const FOLD_FUNCTION: &str = "slatedb_fold";

pub fn register_functions(conn: &Connection) -> rusqlite::Result<()> {
    conn.create_scalar_function(
        FOLD_FUNCTION,
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC | FunctionFlags::SQLITE_INNOCUOUS,
        |ctx| {
            use rusqlite::types::ValueRef;
            let text = match ctx.get_raw(0) {
                ValueRef::Null => return Ok(None),
                ValueRef::Integer(i) => i.to_string(),
                ValueRef::Real(f) => f.to_string(),
                ValueRef::Text(t) | ValueRef::Blob(t) => String::from_utf8_lossy(t).into_owned(),
            };
            Ok(Some(text.to_lowercase()))
        },
    )
}

type Shared = Arc<Mutex<OpenDatabase>>;

#[derive(Default)]
struct Registry {
    /// The database open on the database page.
    current: Option<PathBuf>,
    /// Every open connection, keyed by canonical path. Apps open their
    /// databases here too, so the database page and an app share one
    /// connection per file.
    open: HashMap<PathBuf, Shared>,
}

/// Holds the open databases. Cloning shares the same state.
#[derive(Clone, Default)]
pub struct DbManager {
    inner: Arc<Mutex<Registry>>,
}

fn lock_db(db: &Shared) -> AppResult<MutexGuard<'_, OpenDatabase>> {
    db.lock().map_err(|_| AppError::Internal("database lock poisoned".into()))
}

/// Closes a connection once no one else is using it.
fn close_shared(db: Shared) -> AppResult<()> {
    // Waits for work in flight on another thread, then closes.
    drop(lock_db(&db)?);
    match Arc::try_unwrap(db) {
        Ok(m) => m.into_inner().map_err(|_| AppError::Internal("database lock poisoned".into()))?.close(),
        // Still referenced by a task that is just finishing; it closes on drop.
        Err(_) => Ok(()),
    }
}

impl DbManager {
    fn lock(&self) -> AppResult<MutexGuard<'_, Registry>> {
        self.inner
            .lock()
            .map_err(|_| AppError::Internal("database state lock poisoned".into()))
    }

    /// Makes `db` the current database, closing the previous current one.
    /// If the file is already open (by an app), that connection is kept.
    pub fn set(&self, db: OpenDatabase) -> AppResult<()> {
        let path = db.path.clone();
        let (previous, duplicate) = {
            let mut reg = self.lock()?;
            let previous = reg.current.replace(path.clone()).filter(|p| *p != path).and_then(|p| reg.open.remove(&p));
            let duplicate = match reg.open.entry(path) {
                Entry::Occupied(_) => Some(db),
                Entry::Vacant(slot) => {
                    slot.insert(Arc::new(Mutex::new(db)));
                    None
                }
            };
            (previous, duplicate)
        };
        if let Some(db) = duplicate {
            db.close()?;
        }
        if let Some(prev) = previous {
            close_shared(prev)?;
        }
        Ok(())
    }

    /// Opens the existing file at `path` (or reuses its open connection) and
    /// makes it the current database.
    pub fn open_as_current(&self, path: &Path) -> AppResult<()> {
        let canonical = std::fs::canonicalize(path)?;
        self.with_path(&canonical, |_| Ok(()))?;
        let previous = {
            let mut reg = self.lock()?;
            reg.current.replace(canonical.clone()).filter(|p| *p != canonical).and_then(|p| reg.open.remove(&p))
        };
        previous.map_or(Ok(()), close_shared)
    }

    /// Closes the current database. Returns false when none was open.
    pub fn close(&self) -> AppResult<bool> {
        let previous = {
            let mut reg = self.lock()?;
            reg.current.take().and_then(|p| reg.open.remove(&p))
        };
        match previous {
            Some(db) => close_shared(db).map(|_| true),
            None => Ok(false),
        }
    }

    /// Closes the file at `path` if it is open, current or not.
    pub fn close_path(&self, path: &Path) -> AppResult<()> {
        let removed = {
            let mut reg = self.lock()?;
            if reg.current.as_deref() == Some(path) {
                reg.current = None;
            }
            reg.open.remove(path)
        };
        removed.map_or(Ok(()), close_shared)
    }

    /// Closes every open database except the current one.
    pub fn close_others(&self) -> AppResult<()> {
        let others: Vec<Shared> = {
            let mut reg = self.lock()?;
            let current = reg.current.clone();
            let keys: Vec<PathBuf> = reg.open.keys().filter(|k| Some(*k) != current.as_ref()).cloned().collect();
            keys.iter().filter_map(|k| reg.open.remove(k)).collect()
        };
        others.into_iter().try_for_each(close_shared)
    }

    pub fn current_path(&self) -> AppResult<Option<PathBuf>> {
        Ok(self.lock()?.current.clone())
    }

    pub fn with_db<T>(&self, f: impl FnOnce(&mut OpenDatabase) -> AppResult<T>) -> AppResult<T> {
        let db = {
            let reg = self.lock()?;
            let path = reg.current.as_ref().ok_or(AppError::NoDatabase)?;
            reg.open.get(path).cloned().ok_or(AppError::NoDatabase)?
        };
        let mut guard = lock_db(&db)?;
        f(&mut guard)
    }

    /// Runs `f` on the database file at `path`, opening it if needed. The
    /// connection stays open for later calls until it is closed.
    pub fn with_path<T>(&self, path: &Path, f: impl FnOnce(&mut OpenDatabase) -> AppResult<T>) -> AppResult<T> {
        let canonical = std::fs::canonicalize(path)?;
        let existing = self.lock()?.open.get(&canonical).cloned();
        let db = match existing {
            Some(db) => db,
            None => {
                let opened = OpenDatabase::open(&canonical.display().to_string())?;
                let mut reg = self.lock()?;
                // Another thread may have opened it meanwhile; keep theirs.
                reg.open.entry(canonical).or_insert_with(|| Arc::new(Mutex::new(opened))).clone()
            }
        };
        let mut guard = lock_db(&db)?;
        f(&mut guard)
    }

    /// Runs blocking work on the current database off the async runtime's
    /// worker threads.
    pub async fn run<T, F>(&self, f: F) -> AppResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut OpenDatabase) -> AppResult<T> + Send + 'static,
    {
        let this = self.clone();
        Self::run_detached(move || this.with_db(f)).await
    }

    /// Like `run`, on the database named `name` in the workspace folder `dir`
    /// when one is given, else on the current database.
    pub async fn run_on<T, F>(&self, target: Option<(PathBuf, String)>, f: F) -> AppResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut OpenDatabase) -> AppResult<T> + Send + 'static,
    {
        let this = self.clone();
        Self::run_detached(move || match target {
            Some((dir, name)) => this.with_path(&crate::workspace::resolve_database(&dir, &name)?, f),
            None => this.with_db(f),
        })
        .await
    }

    /// Runs blocking work that does not need a database.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make(dir: &Path, name: &str) -> PathBuf {
        let p = dir.join(format!("{name}.db"));
        let db = OpenDatabase::create(p.to_str().unwrap()).unwrap();
        db.conn.execute_batch(&format!("CREATE TABLE t(x); INSERT INTO t VALUES ('{name}');")).unwrap();
        db.close().unwrap();
        std::fs::canonicalize(p).unwrap()
    }

    fn read(db: &mut OpenDatabase) -> AppResult<String> {
        Ok(db.conn.query_row("SELECT x FROM t", [], |r| r.get(0))?)
    }

    #[test]
    fn several_databases_share_connections() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (make(dir.path(), "a"), make(dir.path(), "b"));
        let mgr = DbManager::default();
        assert!(matches!(mgr.with_db(read), Err(AppError::NoDatabase)));

        mgr.set(OpenDatabase::open(a.to_str().unwrap()).unwrap()).unwrap();
        assert_eq!(mgr.with_path(&b, read).unwrap(), "b");
        assert_eq!(mgr.with_db(read).unwrap(), "a");
        assert_eq!(mgr.lock().unwrap().open.len(), 2);

        // Making the app's database current reuses its connection.
        mgr.set(OpenDatabase::open(b.to_str().unwrap()).unwrap()).unwrap();
        assert_eq!(mgr.lock().unwrap().open.len(), 1);
        assert_eq!(mgr.with_db(read).unwrap(), "b");

        mgr.with_path(&a, read).unwrap();
        mgr.close_others().unwrap();
        assert_eq!(mgr.lock().unwrap().open.len(), 1);
        mgr.with_path(&a, read).unwrap();
        mgr.close_path(&a).unwrap();
        mgr.close_path(&b).unwrap();
        assert!(mgr.current_path().unwrap().is_none());
        assert!(!mgr.close().unwrap());
    }
}

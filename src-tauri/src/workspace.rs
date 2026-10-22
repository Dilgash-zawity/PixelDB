//! The workspace is the folder that holds the user's databases. Databases
//! are addressed by name inside it, so the frontend never passes file paths
//! for database files. The chosen folder is remembered in the app config dir.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::errors::{AppError, AppResult};
use crate::validation::paths::{ALLOWED_EXTENSIONS, check_sqlite_header};

pub const DEFAULT_EXTENSION: &str = "db";
pub const MAX_NAME_LEN: usize = 100;

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceInfo {
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DatabaseFile {
    /// File stem; the name used to address the database.
    pub name: String,
    pub file_name: String,
    pub size: u64,
    /// Last modification, milliseconds since the Unix epoch.
    pub modified: Option<u64>,
    /// False when the file does not have a SQLite header.
    pub valid: bool,
}

#[derive(Default, Serialize, Deserialize)]
struct Settings {
    workspace: Option<PathBuf>,
}

/// Remembers the workspace folder. Cloning shares the same state.
#[derive(Clone)]
pub struct WorkspaceStore {
    settings_path: PathBuf,
    current: Arc<Mutex<Option<PathBuf>>>,
}

impl WorkspaceStore {
    /// Loads the remembered folder; a folder that no longer exists is ignored.
    pub fn load(settings_path: PathBuf) -> Self {
        let saved = fs::read_to_string(&settings_path)
            .ok()
            .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
            .and_then(|s| s.workspace)
            .filter(|p| p.is_dir());
        WorkspaceStore { settings_path, current: Arc::new(Mutex::new(saved)) }
    }

    fn lock(&self) -> AppResult<std::sync::MutexGuard<'_, Option<PathBuf>>> {
        self.current
            .lock()
            .map_err(|_| AppError::Internal("workspace state lock poisoned".into()))
    }

    pub fn get(&self) -> AppResult<Option<PathBuf>> {
        Ok(self.lock()?.clone())
    }

    pub fn require(&self) -> AppResult<PathBuf> {
        self.get()?
            .ok_or_else(|| AppError::NotFound("no database folder has been chosen".into()))
    }

    /// Validates and remembers a folder.
    pub fn set(&self, raw: &str) -> AppResult<PathBuf> {
        let dir = validate_folder(raw)?;
        if let Some(parent) = self.settings_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let settings = Settings { workspace: Some(dir.clone()) };
        // Write-then-rename so a crash never leaves a truncated settings file.
        let tmp = self.settings_path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(&settings)?)?;
        fs::rename(&tmp, &self.settings_path)?;
        *self.lock()? = Some(dir.clone());
        Ok(dir)
    }
}

pub fn validate_folder(raw: &str) -> AppResult<PathBuf> {
    if raw.is_empty() || raw.contains('\0') {
        return Err(AppError::InvalidPath("folder path is empty or invalid".into()));
    }
    let path = Path::new(raw);
    if !path.is_absolute() {
        return Err(AppError::InvalidPath(format!("{raw}: path must be absolute")));
    }
    let dir = fs::canonicalize(path).map_err(|e| AppError::InvalidPath(format!("{raw}: {e}")))?;
    if !dir.is_dir() {
        return Err(AppError::InvalidPath(format!("{}: not a folder", dir.display())));
    }
    Ok(dir)
}

/// Checks a database name and returns its file name (`name.db`). Names are
/// plain file stems: no separators, no reserved device names.
pub fn database_file_name(name: &str) -> AppResult<String> {
    let bad = |why: &str| Err(AppError::Validation(format!("database name {name:?} {why}")));
    if name.trim().is_empty() {
        return bad("is empty");
    }
    if name.trim() != name {
        return bad("must not start or end with spaces");
    }
    if name.chars().count() > MAX_NAME_LEN {
        return bad("is too long");
    }
    if name.starts_with('.') || name.ends_with('.') {
        return bad("must not start or end with a dot");
    }
    if let Some(c) = name.chars().find(|c| c.is_control() || r#"<>:"/\|?*"#.contains(*c)) {
        return bad(&format!("must not contain {c:?}"));
    }
    let upper = name.to_ascii_uppercase();
    let stem = upper.split('.').next().unwrap_or("");
    let reserved = ["CON", "PRN", "AUX", "NUL"].contains(&stem)
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit());
    if reserved {
        return bad("is reserved by the operating system");
    }
    Ok(format!("{name}.{DEFAULT_EXTENSION}"))
}

fn database_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| ALLOWED_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Database files directly inside `dir`, sorted by name.
pub fn list_databases(dir: &Path) -> AppResult<Vec<DatabaseFile>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let meta = match fs::metadata(&path) {
            Ok(m) if m.is_file() => m,
            _ => continue,
        };
        if !database_extension(&path) {
            continue;
        }
        let (Some(name), Some(file_name)) = (
            path.file_stem().and_then(|s| s.to_str()),
            path.file_name().and_then(|s| s.to_str()),
        ) else {
            continue;
        };
        out.push(DatabaseFile {
            name: name.to_string(),
            file_name: file_name.to_string(),
            size: meta.len(),
            modified: meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64),
            valid: check_sqlite_header(&path).is_ok(),
        });
    }
    out.sort_by_key(|d| d.name.to_lowercase());
    Ok(out)
}

/// Resolves a database by name. The path comes from the folder listing, never
/// from joining user input, so names cannot escape the workspace.
pub fn resolve_database(dir: &Path, name: &str) -> AppResult<PathBuf> {
    let matches: Vec<DatabaseFile> = list_databases(dir)?
        .into_iter()
        .filter(|d| d.name == name)
        .collect();
    match matches.as_slice() {
        [one] => Ok(dir.join(&one.file_name)),
        [] => Err(AppError::NotFound(format!("database {name:?} in {}", dir.display()))),
        _ => Err(AppError::Validation(format!(
            "several files are named {name:?} ({}); rename all but one",
            matches.iter().map(|d| d.file_name.as_str()).collect::<Vec<_>>().join(", ")
        ))),
    }
}

/// Path for a new database. Fails if any database with that name exists.
pub fn new_database_path(dir: &Path, name: &str) -> AppResult<PathBuf> {
    let file_name = database_file_name(name)?;
    if list_databases(dir)?.iter().any(|d| d.name.eq_ignore_ascii_case(name)) {
        return Err(AppError::AlreadyExists(format!("a database named {name:?} already exists")));
    }
    Ok(dir.join(file_name))
}

/// Files SQLite may keep next to a database; they belong to it and move with it.
const SIDECAR_SUFFIXES: [&str; 3] = ["-journal", "-wal", "-shm"];

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

/// Fails if another database (case-insensitively) already uses `name`.
fn ensure_name_free(dir: &Path, name: &str, except_file: Option<&str>) -> AppResult<()> {
    let taken = list_databases(dir)?
        .iter()
        .any(|d| d.name.eq_ignore_ascii_case(name) && Some(d.file_name.as_str()) != except_file);
    if taken {
        return Err(AppError::AlreadyExists(format!("a database named {name:?} already exists")));
    }
    Ok(())
}

/// Renames a database file, keeping its extension. Returns the new path.
pub fn rename_database(dir: &Path, name: &str, new_name: &str) -> AppResult<PathBuf> {
    let from = resolve_database(dir, name)?;
    database_file_name(new_name)?;
    let from_file = from.file_name().and_then(|f| f.to_str()).unwrap_or_default().to_string();
    ensure_name_free(dir, new_name, Some(&from_file))?;
    let ext = from.extension().and_then(|e| e.to_str()).unwrap_or(DEFAULT_EXTENSION);
    let to = dir.join(format!("{new_name}.{ext}"));
    if to == from {
        return Ok(to);
    }
    // A case-only rename targets the same file on case-insensitive systems,
    // so the existence check only applies to genuinely different names.
    if !new_name.eq_ignore_ascii_case(name) && fs::symlink_metadata(&to).is_ok() {
        return Err(AppError::AlreadyExists(format!(
            "{}: refusing to overwrite an existing file",
            to.display()
        )));
    }
    fs::rename(&from, &to)?;
    for suffix in SIDECAR_SUFFIXES {
        let side = sidecar(&from, suffix);
        if side.exists() {
            fs::rename(&side, sidecar(&to, suffix))?;
        }
    }
    Ok(to)
}

/// Permanently deletes a database file and its SQLite sidecar files.
pub fn delete_database(dir: &Path, name: &str, confirm: bool) -> AppResult<()> {
    if !confirm {
        return Err(AppError::ConfirmationRequired(format!(
            "deleting database {name:?} cannot be undone"
        )));
    }
    let path = resolve_database(dir, name)?;
    fs::remove_file(&path)?;
    for suffix in SIDECAR_SUFFIXES {
        let side = sidecar(&path, suffix);
        if side.exists() {
            fs::remove_file(&side)?;
        }
    }
    Ok(())
}

/// Copies a database under a new name. `VACUUM INTO` produces a consistent
/// snapshot even if the source has an uncommitted journal.
pub fn duplicate_database(dir: &Path, name: &str, new_name: &str) -> AppResult<PathBuf> {
    let from = resolve_database(dir, name)?;
    check_sqlite_header(&from)?;
    let to = new_database_path(dir, new_name)?;
    let source = rusqlite::Connection::open_with_flags(
        &from,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| AppError::InvalidDatabase(format!("{}: {e}", from.display())))?;
    let target = to.to_str().ok_or_else(|| AppError::InvalidPath("folder path is not valid UTF-8".into()))?;
    source.execute("VACUUM INTO ?1", [target]).inspect_err(|_| {
        // The target did not exist before, so whatever is there is ours.
        let _ = fs::remove_file(&to);
    })?;
    Ok(to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(database_file_name("Sales 2024").unwrap(), "Sales 2024.db");
        for bad in ["", " x", "a/b", "..", "a\\b", "x:y", "CON", "com1", "LPT9.txt", ".hidden", "a\nb"] {
            assert!(database_file_name(bad).is_err(), "{bad:?}");
        }
        assert!(database_file_name("Console").is_ok());
    }

    #[test]
    fn listing_and_resolution() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        fs::write(d.join("notes.txt"), b"x").unwrap();
        fs::write(d.join("fake.db"), b"not sqlite").unwrap();
        fs::create_dir(d.join("folder.db")).unwrap();
        let p = new_database_path(d, "Shop").unwrap();
        crate::database::OpenDatabase::create(p.to_str().unwrap()).unwrap();

        let list = list_databases(d).unwrap();
        let names: Vec<_> = list.iter().map(|x| (x.name.as_str(), x.valid)).collect();
        assert_eq!(names, [("fake", false), ("Shop", true)]);
        assert!(resolve_database(d, "Shop").unwrap().ends_with("Shop.db"));
        assert!(resolve_database(d, "../Shop").is_err());
        assert!(matches!(new_database_path(d, "shop"), Err(AppError::AlreadyExists(_))));
    }

    #[test]
    fn remembers_folder() {
        let dir = tempfile::tempdir().unwrap();
        let settings = dir.path().join("cfg/settings.json");
        let store = WorkspaceStore::load(settings.clone());
        assert!(store.get().unwrap().is_none());
        assert!(store.set("relative").is_err());
        store.set(dir.path().to_str().unwrap()).unwrap();
        let reloaded = WorkspaceStore::load(settings);
        assert_eq!(reloaded.get().unwrap(), Some(fs::canonicalize(dir.path()).unwrap()));
    }

    #[test]
    fn rename_delete_duplicate() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let p = new_database_path(d, "Shop").unwrap();
        let db = crate::database::OpenDatabase::create(p.to_str().unwrap()).unwrap();
        db.conn.execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES (42);").unwrap();
        db.close().unwrap();
        fs::write(d.join("other.sqlite"), b"x").unwrap();

        assert!(matches!(rename_database(d, "Shop", "OTHER"), Err(AppError::AlreadyExists(_))));
        assert!(rename_database(d, "Shop", "a/b").is_err());
        assert!(rename_database(d, "Shop", "shop").unwrap().ends_with("shop.db"));
        assert!(rename_database(d, "shop", "Store").unwrap().ends_with("Store.db"));
        assert!(rename_database(d, "other", "Misc").unwrap().ends_with("Misc.sqlite"));

        assert!(matches!(duplicate_database(d, "Store", "misc"), Err(AppError::AlreadyExists(_))));
        assert!(duplicate_database(d, "Misc", "Copy").is_err());
        let copy = duplicate_database(d, "Store", "Store copy").unwrap();
        let opened = crate::database::OpenDatabase::open(copy.to_str().unwrap()).unwrap();
        let x: i64 = opened.conn.query_row("SELECT x FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(x, 42);
        let app_id: i32 = opened.conn.pragma_query_value(None, "application_id", |r| r.get(0)).unwrap();
        assert_eq!(app_id, crate::database::connection::APPLICATION_ID);
        opened.close().unwrap();

        assert!(matches!(delete_database(d, "Store", false), Err(AppError::ConfirmationRequired(_))));
        delete_database(d, "Store", true).unwrap();
        let names: Vec<_> = list_databases(d).unwrap().into_iter().map(|x| x.name).collect();
        assert_eq!(names, ["Misc", "Store copy"]);
    }
}

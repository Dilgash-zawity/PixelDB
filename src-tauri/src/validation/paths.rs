use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::errors::{AppError, AppResult};

pub const ALLOWED_EXTENSIONS: &[&str] = &["db", "sqlite", "sqlite3", "db3", "s3db", "sl3"];
const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";

fn base_checks(raw: &str) -> AppResult<PathBuf> {
    if raw.is_empty() {
        return Err(AppError::InvalidPath("path is empty".into()));
    }
    if raw.contains('\0') {
        return Err(AppError::InvalidPath("path contains a NUL byte".into()));
    }
    let path = PathBuf::from(raw);
    if !path.is_absolute() {
        return Err(AppError::InvalidPath(format!("{raw}: path must be absolute")));
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError::InvalidPath(format!(
            "{raw}: extension must be one of {}",
            ALLOWED_EXTENSIONS.join(", ")
        )));
    }
    Ok(path)
}

/// Validates the target of a new database. The file must not exist: this
/// application never overwrites existing files.
pub fn validate_new_database_path(raw: &str) -> AppResult<PathBuf> {
    let path = base_checks(raw)?;
    let file_name = path
        .file_name()
        .ok_or_else(|| AppError::InvalidPath(format!("{raw}: missing file name")))?;
    let parent = path
        .parent()
        .ok_or_else(|| AppError::InvalidPath(format!("{raw}: missing parent directory")))?;
    let parent = fs::canonicalize(parent)
        .map_err(|e| AppError::InvalidPath(format!("{}: {e}", parent.display())))?;
    if !parent.is_dir() {
        return Err(AppError::InvalidPath(format!("{}: not a directory", parent.display())));
    }
    let target = parent.join(file_name);
    if fs::symlink_metadata(&target).is_ok() {
        return Err(AppError::AlreadyExists(format!(
            "{}: refusing to overwrite an existing file",
            target.display()
        )));
    }
    Ok(target)
}

/// Validates an existing database file and checks its SQLite header.
pub fn validate_existing_database_path(raw: &str) -> AppResult<PathBuf> {
    let path = base_checks(raw)?;
    let path = fs::canonicalize(&path)
        .map_err(|e| AppError::InvalidPath(format!("{raw}: {e}")))?;
    if !path.is_file() {
        return Err(AppError::InvalidPath(format!("{}: not a regular file", path.display())));
    }
    check_sqlite_header(&path)?;
    Ok(path)
}

pub fn check_sqlite_header(path: &Path) -> AppResult<()> {
    let mut header = [0u8; 16];
    let mut file = fs::File::open(path)?;
    let read = file.read(&mut header)?;
    if read < header.len() || &header != SQLITE_HEADER {
        return Err(AppError::InvalidDatabase(format!(
            "{}: missing SQLite header",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_relative_and_bad_extensions() {
        assert!(validate_new_database_path("relative.db").is_err());
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.txt");
        assert!(validate_new_database_path(p.to_str().unwrap()).is_err());
    }

    #[test]
    fn refuses_to_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.db");
        fs::write(&p, b"data").unwrap();
        assert!(matches!(
            validate_new_database_path(p.to_str().unwrap()),
            Err(AppError::AlreadyExists(_))
        ));
    }

    #[test]
    fn rejects_non_sqlite_files() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("fake.db");
        fs::write(&p, b"this is not a database at all").unwrap();
        assert!(matches!(
            validate_existing_database_path(p.to_str().unwrap()),
            Err(AppError::InvalidDatabase(_))
        ));
    }
}

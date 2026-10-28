//! App packages: one `.slatepkg` file (a zip) holding an app and a copy of
//! each local database it uses, so it can be handed to someone else and
//! opened in their workspace. Online sources are listed but not included
//! (connection settings stay on each computer, passwords always).
//!
//! Layout: `manifest.json` (`Manifest`), `app.baseapp` (the definition) and
//! `databases/<n>.db` for the n-th entry of `manifest.databases`. Names in
//! the manifest are only labels: files are placed by index, and the new
//! names are checked like any other database or app name.

use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::apps;
use crate::errors::{AppError, AppResult};
use crate::validation::paths::check_sqlite_header;
use crate::workspace;

pub const PACKAGE_EXTENSION: &str = "slatepkg";
pub const PACKAGE_FORMAT: u64 = 1;
/// The largest package read or written.
pub const MAX_PACKAGE_BYTES: u64 = 1024 * 1024 * 1024;
const MANIFEST: &str = "manifest.json";
const APP_ENTRY: &str = "app.baseapp";
const MAX_DATABASES: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub format: u64,
    pub app: String,
    /// Local databases included, in file order.
    pub databases: Vec<String>,
    /// Online databases the app uses; not included.
    #[serde(default)]
    pub online: Vec<String>,
}

/// What an import made.
#[derive(Debug, Clone, Serialize)]
pub struct ImportedPackage {
    pub app: String,
    /// `[name in the package, name it got here]`.
    pub databases: Vec<[String; 2]>,
    /// Online databases the app expects; it works once a connection of that name exists.
    pub online: Vec<String>,
}

fn sources(def: &JsonValue) -> Vec<String> {
    let mut out: Vec<String> = def
        .get("sources")
        .and_then(JsonValue::as_array)
        .map(|s| s.iter().filter_map(|x| x.get("database")?.as_str().map(String::from)).collect())
        .unwrap_or_default();
    out.dedup();
    let mut seen = std::collections::HashSet::new();
    out.retain(|n| seen.insert(n.clone()));
    out
}

fn scratch(dir: &Path, label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    dir.join(format!(".slatedb-{label}-{}-{nanos}.part", std::process::id()))
}

fn zip_error(e: zip::result::ZipError) -> AppError {
    AppError::Validation(format!("not a PixelDB package ({e})"))
}

/// The package for app `name`: its definition and a snapshot of each local database.
pub fn write_package(workspace: &Path, name: &str) -> AppResult<Vec<u8>> {
    let def = apps::load_app(workspace, name)?;
    let dir = workspace::databases_dir(workspace)?;
    let online_names = crate::online::names(&dir)?;
    let (online, local): (Vec<String>, Vec<String>) = sources(&def).into_iter().partition(|d| online_names.contains(d));
    let manifest = Manifest { format: PACKAGE_FORMAT, app: name.to_string(), databases: local.clone(), online };

    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let deflated = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(MANIFEST, deflated).map_err(zip_error)?;
    zip.write_all(&serde_json::to_vec_pretty(&manifest)?)?;
    zip.start_file(APP_ENTRY, deflated).map_err(zip_error)?;
    zip.write_all(&serde_json::to_vec_pretty(&def)?)?;
    let mut total = 0u64;
    for (i, db) in local.iter().enumerate() {
        let from = workspace::resolve_database(&dir, db)?;
        check_sqlite_header(&from)?;
        // VACUUM INTO gives a consistent, compact copy even while the file is open.
        let snapshot = scratch(&dir, "pack");
        let result = (|| -> AppResult<Vec<u8>> {
            let source = rusqlite::Connection::open_with_flags(&from, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX)
                .map_err(|e| AppError::InvalidDatabase(format!("{db}: {e}")))?;
            let target = snapshot.to_str().ok_or_else(|| AppError::InvalidPath("folder path is not valid UTF-8".into()))?;
            source.execute("VACUUM INTO ?1", [target])?;
            Ok(fs::read(&snapshot)?)
        })();
        let _ = fs::remove_file(&snapshot);
        let bytes = result?;
        total += bytes.len() as u64;
        if total > MAX_PACKAGE_BYTES {
            return Err(AppError::Validation("the databases are too large to package together".into()));
        }
        zip.start_file(format!("databases/{i}.db"), deflated.large_file(bytes.len() as u64 >= u32::MAX as u64)).map_err(zip_error)?;
        zip.write_all(&bytes)?;
    }
    Ok(zip.finish().map_err(zip_error)?.into_inner())
}

/// `base`, or `base 2`, `base 3`… — the first that `taken` refuses.
fn free_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_string();
    }
    (2..).map(|n| format!("{base} {n}")).find(|n| !taken(n)).expect("some name is free")
}

fn read_entry(archive: &mut zip::ZipArchive<Cursor<&[u8]>>, name: &str, limit: u64) -> AppResult<Vec<u8>> {
    let entry = archive.by_name(name).map_err(zip_error)?;
    if entry.size() > limit {
        return Err(AppError::Validation(format!("{name} in the package is too large")));
    }
    let mut out = Vec::with_capacity(entry.size() as usize);
    // `take` guards against an entry whose real size exceeds what it declares.
    entry.take(limit + 1).read_to_end(&mut out)?;
    if out.len() as u64 > limit {
        return Err(AppError::Validation(format!("{name} in the package is too large")));
    }
    Ok(out)
}

/// Adds the package's databases and app to the workspace under free names,
/// pointing the app at the new names. All or nothing: on failure whatever
/// was written is removed.
pub fn read_package(workspace: &Path, bytes: &[u8]) -> AppResult<ImportedPackage> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(zip_error)?;
    let manifest: Manifest = serde_json::from_slice(&read_entry(&mut archive, MANIFEST, 1024 * 1024)?)
        .map_err(|e| AppError::Validation(format!("not a PixelDB package ({e})")))?;
    if manifest.format > PACKAGE_FORMAT {
        return Err(AppError::Validation("this package was made by a newer PixelDB; update to open it".into()));
    }
    if manifest.databases.len() > MAX_DATABASES {
        return Err(AppError::Validation("the package has too many databases".into()));
    }
    let mut def: JsonValue = serde_json::from_slice(&read_entry(&mut archive, APP_ENTRY, apps::MAX_APP_BYTES as u64)?)
        .map_err(|e| AppError::Validation(format!("the package's app is not valid ({e})")))?;
    apps::validate_definition(&def)?;

    let dir = workspace::databases_dir(workspace)?;
    let mut created: Vec<PathBuf> = Vec::new();
    let result = (|| -> AppResult<ImportedPackage> {
        let mut renamed: Vec<[String; 2]> = Vec::new();
        for (i, original) in manifest.databases.iter().enumerate() {
            let data = read_entry(&mut archive, &format!("databases/{i}.db"), MAX_PACKAGE_BYTES)?;
            let local = workspace::list_databases(&dir)?;
            let online = crate::online::names(&dir)?;
            let base = if workspace::database_file_name(original).is_ok() { original.clone() } else { format!("Database {}", i + 1) };
            let name = free_name(&base, |n| {
                local.iter().any(|d| d.name.eq_ignore_ascii_case(n))
                    || online.iter().any(|o| o.eq_ignore_ascii_case(n))
                    || renamed.iter().any(|[_, m]| m.eq_ignore_ascii_case(n))
            });
            let path = workspace::new_database_path(&dir, &name)?;
            let part = scratch(&dir, "unpack");
            let checked = (|| -> AppResult<()> {
                fs::write(&part, &data)?;
                check_sqlite_header(&part)?;
                let conn = rusqlite::Connection::open_with_flags(&part, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .map_err(|e| AppError::InvalidDatabase(format!("{original}: {e}")))?;
                let ok: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
                if ok != "ok" {
                    return Err(AppError::InvalidDatabase(format!("{original} in the package is damaged ({ok})")));
                }
                drop(conn);
                // hard_link refuses an existing target, so nothing is overwritten.
                fs::hard_link(&part, &path).or_else(|_| {
                    fs::OpenOptions::new().write(true).create_new(true).open(&path)?.write_all(&data)
                })?;
                Ok(())
            })();
            let _ = fs::remove_file(&part);
            checked?;
            created.push(path);
            renamed.push([original.clone(), name]);
        }

        if let Some(sources) = def.get_mut("sources").and_then(JsonValue::as_array_mut) {
            for s in sources {
                let Some(db) = s.get("database").and_then(JsonValue::as_str) else { continue };
                if let Some([_, to]) = renamed.iter().find(|[from, _]| from == db) {
                    s["database"] = JsonValue::String(to.clone());
                }
            }
        }
        let apps_now = apps::list_apps(workspace)?;
        let base = if workspace::check_file_stem(&manifest.app, "app").is_ok() { manifest.app.clone() } else { "Imported app".to_string() };
        let app = free_name(&base, |n| apps_now.iter().any(|a| a.name.eq_ignore_ascii_case(n)));
        apps::create_app(workspace, &app, &def)?;
        Ok(ImportedPackage { app, databases: renamed, online: manifest.online.clone() })
    })();
    if result.is_err() {
        for path in created {
            let _ = fs::remove_file(path);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_names_count_up() {
        assert_eq!(free_name("Shop", |_| false), "Shop");
        assert_eq!(free_name("Shop", |n| n == "Shop" || n == "Shop 2"), "Shop 3");
    }
}

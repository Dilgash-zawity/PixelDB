//! Apps: pages of forms, tables and charts over one or more workspace
//! databases. Each app is a JSON file in the workspace's `Apps` folder,
//! addressed by name like databases are.
//!
//! The definition is owned by the frontend (`src/lib/logic/app-def.ts`) and
//! stored as opaque JSON. It is declarative only: apps reach data through
//! the ordinary record and aggregate commands, so the backend's validation
//! and confirmation rules apply to them unchanged.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::errors::{AppError, AppResult};
use crate::workspace::check_file_stem;

pub const APPS_FOLDER: &str = "Apps";
pub const APP_EXTENSION: &str = "baseapp";
/// Newest definition format this build understands.
pub const APP_FORMAT_VERSION: u64 = 1;
pub const MAX_APP_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct AppFile {
    pub name: String,
    pub size: u64,
    /// Last modification, milliseconds since the Unix epoch.
    pub modified: Option<u64>,
    /// Databases the app uses, read from its definition; empty when the file
    /// can't be read.
    pub databases: Vec<String>,
    pub description: String,
    /// Accent color name (see `AppAccent` in app-def.ts).
    pub accent: String,
    /// False when the file is not a readable app definition.
    pub valid: bool,
}

/// The `Apps` folder inside the workspace. It must be a real folder, not a
/// link to somewhere else.
fn apps_dir(workspace: &Path, create: bool) -> AppResult<Option<PathBuf>> {
    let dir = workspace.join(APPS_FOLDER);
    match fs::symlink_metadata(&dir) {
        Ok(m) if m.is_dir() => Ok(Some(dir)),
        Ok(_) => Err(AppError::InvalidPath(format!("{}: not a folder", dir.display()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if create {
                fs::create_dir(&dir)?;
                Ok(Some(dir))
            } else {
                Ok(None)
            }
        }
        Err(e) => Err(e.into()),
    }
}

/// Checks that `def` looks like an app definition this build can handle.
pub fn validate_definition(def: &JsonValue) -> AppResult<()> {
    let obj = def
        .as_object()
        .ok_or_else(|| AppError::Validation("the app definition is not an object".into()))?;
    match obj.get("version").and_then(JsonValue::as_u64) {
        Some(v) if (1..=APP_FORMAT_VERSION).contains(&v) => {}
        Some(v) => {
            return Err(AppError::Unsupported(format!(
                "this app was made by a newer version of Base (format {v}); update Base to open it"
            )));
        }
        None => return Err(AppError::Validation("the app definition has no version".into())),
    }
    if !obj.get("pages").is_some_and(JsonValue::is_array) {
        return Err(AppError::Validation("the app definition has no pages".into()));
    }
    Ok(())
}

fn databases_of(def: &JsonValue) -> Vec<String> {
    def.get("sources")
        .and_then(JsonValue::as_array)
        .map(|s| s.iter().filter_map(|x| x.get("database")?.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

fn text_of(def: Option<&JsonValue>, key: &str) -> String {
    def.and_then(|d| d.get(key)).and_then(JsonValue::as_str).unwrap_or_default().to_string()
}

fn read_definition(path: &Path) -> AppResult<JsonValue> {
    let meta = fs::metadata(path)?;
    if meta.len() as usize > MAX_APP_BYTES {
        return Err(AppError::Validation(format!("{}: the app file is too large", path.display())));
    }
    let def: JsonValue = serde_json::from_slice(&fs::read(path)?)
        .map_err(|e| AppError::Validation(format!("{}: not a valid app file ({e})", path.display())))?;
    validate_definition(&def)?;
    Ok(def)
}

fn encode(def: &JsonValue) -> AppResult<Vec<u8>> {
    validate_definition(def)?;
    let bytes = serde_json::to_vec_pretty(def)?;
    if bytes.len() > MAX_APP_BYTES {
        return Err(AppError::Validation("the app is too large to save".into()));
    }
    Ok(bytes)
}

fn is_app_file(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case(APP_EXTENSION))
}

/// App files in the workspace, sorted by name.
pub fn list_apps(workspace: &Path) -> AppResult<Vec<AppFile>> {
    let Some(dir) = apps_dir(workspace, false)? else { return Ok(Vec::new()) };
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) if m.is_file() => m,
            _ => continue,
        };
        let Some(name) = path.file_stem().and_then(|s| s.to_str()).filter(|_| is_app_file(&path)) else {
            continue;
        };
        let def = read_definition(&path).ok();
        out.push(AppFile {
            name: name.to_string(),
            size: meta.len(),
            modified: meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64),
            databases: def.as_ref().map(databases_of).unwrap_or_default(),
            description: text_of(def.as_ref(), "description"),
            accent: text_of(def.as_ref(), "accent"),
            valid: def.is_some(),
        });
    }
    out.sort_by_key(|a| a.name.to_lowercase());
    Ok(out)
}

/// Resolves an app by name from the folder listing, never by joining input.
pub fn resolve_app(workspace: &Path, name: &str) -> AppResult<PathBuf> {
    let dir = apps_dir(workspace, false)?.ok_or_else(|| AppError::NotFound(format!("app {name:?}")))?;
    list_apps(workspace)?
        .into_iter()
        .find(|a| a.name == name)
        .map(|a| dir.join(format!("{}.{APP_EXTENSION}", a.name)))
        .filter(|p| p.is_file())
        .ok_or_else(|| AppError::NotFound(format!("app {name:?}")))
}

fn ensure_name_free(workspace: &Path, name: &str, except: Option<&str>) -> AppResult<()> {
    let taken = list_apps(workspace)?.iter().any(|a| a.name.eq_ignore_ascii_case(name) && Some(a.name.as_str()) != except);
    if taken {
        return Err(AppError::AlreadyExists(format!("an app named {name:?} already exists")));
    }
    Ok(())
}

fn new_app_path(workspace: &Path, name: &str) -> AppResult<PathBuf> {
    check_file_stem(name, "app")?;
    ensure_name_free(workspace, name, None)?;
    let dir = apps_dir(workspace, true)?.ok_or_else(|| AppError::Internal("apps folder missing".into()))?;
    Ok(dir.join(format!("{name}.{APP_EXTENSION}")))
}

/// Writes a new file; an existing file is never overwritten.
fn write_new(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::AlreadyExists => {
            AppError::AlreadyExists(format!("{}: refusing to overwrite an existing file", path.display()))
        }
        _ => AppError::Io(format!("{}: {e}", path.display())),
    })?;
    file.write_all(bytes).inspect_err(|_| {
        let _ = fs::remove_file(path);
    })?;
    Ok(())
}

pub fn create_app(workspace: &Path, name: &str, def: &JsonValue) -> AppResult<()> {
    let bytes = encode(def)?;
    write_new(&new_app_path(workspace, name)?, &bytes)
}

pub fn load_app(workspace: &Path, name: &str) -> AppResult<JsonValue> {
    read_definition(&resolve_app(workspace, name)?)
}

/// Replaces an app's definition. Write-then-rename, so a crash never leaves
/// a truncated file.
pub fn save_app(workspace: &Path, name: &str, def: &JsonValue) -> AppResult<()> {
    let path = resolve_app(workspace, name)?;
    let bytes = encode(def)?;
    let tmp = path.with_extension(format!("{APP_EXTENSION}.tmp"));
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, &path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })?;
    Ok(())
}

pub fn rename_app(workspace: &Path, name: &str, new_name: &str) -> AppResult<()> {
    let from = resolve_app(workspace, name)?;
    check_file_stem(new_name, "app")?;
    ensure_name_free(workspace, new_name, Some(name))?;
    let to = from.with_file_name(format!("{new_name}.{APP_EXTENSION}"));
    if to == from {
        return Ok(());
    }
    if !new_name.eq_ignore_ascii_case(name) && fs::symlink_metadata(&to).is_ok() {
        return Err(AppError::AlreadyExists(format!("{}: refusing to overwrite an existing file", to.display())));
    }
    fs::rename(&from, &to)?;
    Ok(())
}

pub fn delete_app(workspace: &Path, name: &str, confirm: bool) -> AppResult<()> {
    if !confirm {
        return Err(AppError::ConfirmationRequired(format!("deleting app {name:?} cannot be undone")));
    }
    fs::remove_file(resolve_app(workspace, name)?)?;
    Ok(())
}

pub fn duplicate_app(workspace: &Path, name: &str, new_name: &str) -> AppResult<()> {
    let def = load_app(workspace, name)?;
    create_app(workspace, new_name, &def)
}

/// Rewrites every app that uses database `from` to use `to` instead, after
/// the database file was renamed. Unreadable apps are left alone.
pub fn rename_database_in_apps(workspace: &Path, from: &str, to: &str) -> AppResult<()> {
    for app in list_apps(workspace)?.into_iter().filter(|a| a.valid && a.databases.iter().any(|d| d == from)) {
        let mut def = load_app(workspace, &app.name)?;
        if let Some(sources) = def.get_mut("sources").and_then(JsonValue::as_array_mut) {
            for s in sources {
                if s.get("database").and_then(JsonValue::as_str) == Some(from) {
                    s["database"] = JsonValue::String(to.to_string());
                }
            }
        }
        save_app(workspace, &app.name, &def)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn def(dbs: &[&str]) -> JsonValue {
        let sources: Vec<_> = dbs.iter().map(|d| json!({"alias": d.to_lowercase(), "database": d})).collect();
        json!({"version": 1, "name": "x", "sources": sources, "pages": []})
    }

    #[test]
    fn create_list_load_save() {
        let tmp = tempfile::tempdir().unwrap();
        let w = tmp.path();
        assert!(list_apps(w).unwrap().is_empty());
        create_app(w, "Shop", &def(&["Sales", "Stock"])).unwrap();
        assert!(matches!(create_app(w, "shop", &def(&[])), Err(AppError::AlreadyExists(_))));
        assert!(create_app(w, "a/b", &def(&[])).is_err());
        assert!(create_app(w, "Bad", &json!({"pages": []})).is_err());
        assert!(matches!(
            create_app(w, "Future", &json!({"version": 99, "pages": []})),
            Err(AppError::Unsupported(_))
        ));
        fs::write(w.join(APPS_FOLDER).join("Broken.baseapp"), b"{").unwrap();
        fs::write(w.join(APPS_FOLDER).join("notes.txt"), b"x").unwrap();

        let list = list_apps(w).unwrap();
        let names: Vec<_> = list.iter().map(|a| (a.name.as_str(), a.valid)).collect();
        assert_eq!(names, [("Broken", false), ("Shop", true)]);
        assert_eq!(list[1].databases, ["Sales", "Stock"]);
        assert!(load_app(w, "Broken").is_err());
        assert!(load_app(w, "../Shop").is_err());

        let mut d = load_app(w, "Shop").unwrap();
        d["name"] = json!("changed");
        save_app(w, "Shop", &d).unwrap();
        assert_eq!(load_app(w, "Shop").unwrap()["name"], "changed");
        assert!(save_app(w, "Shop", &json!([])).is_err());
        assert_eq!(load_app(w, "Shop").unwrap()["name"], "changed");
    }

    #[test]
    fn rename_duplicate_delete() {
        let tmp = tempfile::tempdir().unwrap();
        let w = tmp.path();
        create_app(w, "Shop", &def(&["Sales"])).unwrap();
        create_app(w, "Other", &def(&["Stock"])).unwrap();
        assert!(matches!(rename_app(w, "Shop", "other"), Err(AppError::AlreadyExists(_))));
        rename_app(w, "Shop", "Store").unwrap();
        duplicate_app(w, "Store", "Store copy").unwrap();
        assert!(matches!(delete_app(w, "Store", false), Err(AppError::ConfirmationRequired(_))));
        delete_app(w, "Store", true).unwrap();
        let names: Vec<_> = list_apps(w).unwrap().into_iter().map(|a| a.name).collect();
        assert_eq!(names, ["Other", "Store copy"]);

        rename_database_in_apps(w, "Sales", "Revenue").unwrap();
        assert_eq!(load_app(w, "Store copy").unwrap()["sources"][0]["database"], "Revenue");
        assert_eq!(load_app(w, "Other").unwrap()["sources"][0]["database"], "Stock");
    }
}

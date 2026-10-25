//! Online databases: saved connections to database servers (PostgreSQL,
//! MySQL, Turso…) shown next to the local files. Each one is a small JSON
//! file `<workspace>/DB/<name>.online` holding everything but the password
//! or token, addressed by name like local databases. Secrets live in the app
//! config dir instead (`SecretStore`), so a workspace folder that is synced
//! or shared never carries them.
//!
//! For now a connection can be saved, edited, tested and removed; browsing
//! its tables comes later.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::errors::{AppError, AppResult};
use crate::workspace::check_file_stem;

pub const ONLINE_EXTENSION: &str = "online";
const FORMAT_VERSION: u64 = 1;
const MAX_FILE_BYTES: u64 = 64 * 1024;
const TEST_TIMEOUT: Duration = Duration::from_secs(15);

/// How the app talks to the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Engine {
    Postgres,
    Mysql,
    /// libSQL (SQLite) over HTTP, as Turso serves it.
    Libsql,
}

/// Hosting services and plain servers; the frontend's `PROVIDERS` in
/// `logic/online.ts` mirrors these ids.
pub const PROVIDERS: &[(&str, Engine)] = &[
    ("postgres", Engine::Postgres),
    ("neon", Engine::Postgres),
    ("supabase", Engine::Postgres),
    ("cockroachdb", Engine::Postgres),
    ("mysql", Engine::Mysql),
    ("mariadb", Engine::Mysql),
    ("planetscale", Engine::Mysql),
    ("libsql", Engine::Libsql),
    ("turso", Engine::Libsql),
];

/// Where a connection points. Everything except the password or token.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionSettings {
    pub provider: String,
    pub engine: Engine,
    pub host: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub database: String,
    #[serde(default)]
    pub user: String,
    /// Encrypt the connection (always on for libSQL, which uses HTTPS).
    #[serde(default = "yes")]
    pub ssl: bool,
}

fn yes() -> bool {
    true
}

#[derive(Serialize, Deserialize)]
struct OnlineFile {
    version: u64,
    #[serde(flatten)]
    settings: ConnectionSettings,
}

#[derive(Debug, Clone, Serialize)]
pub struct OnlineDatabase {
    pub name: String,
    /// None when the file can't be read.
    pub settings: Option<ConnectionSettings>,
    /// A password or token is saved for it on this computer.
    pub has_secret: bool,
    /// Last modification, milliseconds since the Unix epoch.
    pub modified: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionTest {
    /// What the server says it is, e.g. "PostgreSQL 16.4".
    pub server: String,
    pub millis: u64,
}

impl ConnectionSettings {
    pub fn validate(&self) -> AppResult<()> {
        let bad = |m: String| Err(AppError::Validation(m));
        match PROVIDERS.iter().find(|(id, _)| *id == self.provider) {
            None => return bad(format!("unknown provider {:?}", self.provider)),
            Some((_, engine)) if *engine != self.engine => {
                return bad(format!("{} does not use {:?}", self.provider, self.engine));
            }
            _ => {}
        }
        let host = self.host.as_str();
        if host.is_empty() {
            return bad("the server address is empty".into());
        }
        if host.len() > 253 || host.chars().any(|c| c.is_whitespace() || c.is_control() || "/\\?#@".contains(c)) {
            return bad(format!("{host:?} is not a server address"));
        }
        if self.port == Some(0) {
            return bad("the port must be between 1 and 65535".into());
        }
        for (what, v) in [("database name", &self.database), ("user name", &self.user)] {
            if v.chars().count() > 128 || v.chars().any(char::is_control) {
                return bad(format!("the {what} is not valid"));
            }
        }
        if self.engine != Engine::Libsql && self.user.is_empty() {
            return bad("the user name is empty".into());
        }
        Ok(())
    }
}

fn millis(meta: &fs::Metadata) -> Option<u64> {
    meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64)
}

fn read_settings(path: &Path) -> AppResult<ConnectionSettings> {
    if fs::metadata(path)?.len() > MAX_FILE_BYTES {
        return Err(AppError::Validation(format!("{}: file is too large", path.display())));
    }
    let file: OnlineFile = serde_json::from_slice(&fs::read(path)?)?;
    if file.version > FORMAT_VERSION {
        return Err(AppError::Unsupported("this connection was saved by a newer version of the app".into()));
    }
    file.settings.validate()?;
    Ok(file.settings)
}

fn encode(settings: &ConnectionSettings) -> AppResult<Vec<u8>> {
    settings.validate()?;
    Ok(serde_json::to_vec_pretty(&OnlineFile { version: FORMAT_VERSION, settings: settings.clone() })?)
}

/// `(name, path)` of each connection file directly inside `dir`, sorted by name.
fn entries(dir: &Path) -> AppResult<Vec<(String, PathBuf, fs::Metadata)>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let Ok(meta) = fs::metadata(&path) else { continue };
        let is_ours = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case(ONLINE_EXTENSION));
        if !meta.is_file() || !is_ours {
            continue;
        }
        if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
            out.push((name.to_string(), path.clone(), meta));
        }
    }
    out.sort_by_key(|(n, ..)| n.to_lowercase());
    Ok(out)
}

/// Names of the online databases in `dir`.
pub fn names(dir: &Path) -> AppResult<Vec<String>> {
    Ok(entries(dir)?.into_iter().map(|(n, ..)| n).collect())
}

pub fn list(dir: &Path, secrets: &SecretStore) -> AppResult<Vec<OnlineDatabase>> {
    entries(dir)?
        .into_iter()
        .map(|(name, path, meta)| {
            Ok(OnlineDatabase {
                settings: read_settings(&path).ok(),
                has_secret: secrets.get(dir, &name)?.is_some(),
                modified: millis(&meta),
                name,
            })
        })
        .collect()
}

/// Resolves a connection by name from the folder listing, never by joining input.
fn resolve(dir: &Path, name: &str) -> AppResult<PathBuf> {
    entries(dir)?
        .into_iter()
        .find(|(n, ..)| n == name)
        .map(|(_, p, _)| p)
        .ok_or_else(|| AppError::NotFound(format!("online database {name:?}")))
}

pub fn settings(dir: &Path, name: &str) -> AppResult<ConnectionSettings> {
    read_settings(&resolve(dir, name)?)
}

/// Fails if a local or online database other than `except` uses `name`.
fn ensure_name_free(dir: &Path, name: &str, except: Option<&str>) -> AppResult<()> {
    check_file_stem(name, "database")?;
    let local = crate::workspace::list_databases(dir)?.into_iter().map(|d| d.name);
    let taken = local
        .chain(names(dir)?.into_iter().filter(|n| Some(n.as_str()) != except))
        .any(|n| n.eq_ignore_ascii_case(name));
    if taken {
        return Err(AppError::AlreadyExists(format!("a database named {name:?} already exists")));
    }
    Ok(())
}

/// Saves a new connection. `secret`, when given, is kept in `secrets`.
pub fn create(dir: &Path, secrets: &SecretStore, name: &str, settings: &ConnectionSettings, secret: Option<&str>) -> AppResult<()> {
    ensure_name_free(dir, name, None)?;
    let bytes = encode(settings)?;
    let path = dir.join(format!("{name}.{ONLINE_EXTENSION}"));
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::AlreadyExists => AppError::AlreadyExists(format!("{}: refusing to overwrite an existing file", path.display())),
        _ => AppError::Io(format!("{}: {e}", path.display())),
    })?;
    file.write_all(&bytes).inspect_err(|_| {
        let _ = fs::remove_file(&path);
    })?;
    if let Some(s) = secret.filter(|s| !s.is_empty()) {
        secrets.set(dir, name, Some(s))?;
    }
    Ok(())
}

/// Replaces a connection's settings, renaming it to `new_name`. `secret`:
/// None keeps the saved one, an empty string forgets it.
pub fn update(
    dir: &Path,
    secrets: &SecretStore,
    name: &str,
    new_name: &str,
    settings: &ConnectionSettings,
    secret: Option<&str>,
) -> AppResult<()> {
    let from = resolve(dir, name)?;
    ensure_name_free(dir, new_name, Some(name))?;
    let bytes = encode(settings)?;
    let to = dir.join(format!("{new_name}.{ONLINE_EXTENSION}"));
    if to != from && !new_name.eq_ignore_ascii_case(name) && fs::symlink_metadata(&to).is_ok() {
        return Err(AppError::AlreadyExists(format!("{}: refusing to overwrite an existing file", to.display())));
    }
    // Write-then-rename, so a crash never leaves a truncated file.
    let tmp = from.with_extension(format!("{ONLINE_EXTENSION}.tmp"));
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, &from).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })?;
    if to != from {
        fs::rename(&from, &to)?;
        secrets.rename(dir, name, new_name)?;
    }
    if let Some(s) = secret {
        secrets.set(dir, new_name, Some(s).filter(|s| !s.is_empty()))?;
    }
    Ok(())
}

/// Forgets a connection and its secret. The server and its data are not touched.
pub fn delete(dir: &Path, secrets: &SecretStore, name: &str, confirm: bool) -> AppResult<()> {
    if !confirm {
        return Err(AppError::ConfirmationRequired(format!("removing the connection {name:?}")));
    }
    fs::remove_file(resolve(dir, name)?)?;
    secrets.set(dir, name, None)
}

// ── Testing a connection ──

pub(crate) fn connection_error(e: impl std::fmt::Display) -> AppError {
    let text = e.to_string();
    AppError::Connection(text.strip_prefix("error returned from database: ").unwrap_or(&text).to_string())
}

/// Connects, asks the server for its version and disconnects.
pub async fn test(settings: &ConnectionSettings, secret: Option<&str>) -> AppResult<ConnectionTest> {
    settings.validate()?;
    let started = Instant::now();
    let server = tokio::time::timeout(TEST_TIMEOUT, server_version(settings, secret))
        .await
        .map_err(|_| AppError::Connection(format!("{} did not answer within {} seconds", settings.host, TEST_TIMEOUT.as_secs())))??;
    Ok(ConnectionTest { server, millis: started.elapsed().as_millis() as u64 })
}

async fn server_version(s: &ConnectionSettings, secret: Option<&str>) -> AppResult<String> {
    use sqlx::Connection;
    let secret = secret.unwrap_or_default();
    match s.engine {
        Engine::Postgres => {
            use sqlx::postgres::{PgConnectOptions, PgConnection, PgSslMode};
            let mut opts = PgConnectOptions::new()
                .host(&s.host)
                .port(s.port.unwrap_or(5432))
                .username(&s.user)
                .ssl_mode(if s.ssl { PgSslMode::Require } else { PgSslMode::Prefer });
            if !s.database.is_empty() {
                opts = opts.database(&s.database);
            }
            // An empty secret means no password; sending one fails where none is set.
            if !secret.is_empty() {
                opts = opts.password(secret);
            }
            let mut conn = PgConnection::connect_with(&opts).await.map_err(connection_error)?;
            let (v,): (String,) = sqlx::query_as("SELECT version()").fetch_one(&mut conn).await.map_err(connection_error)?;
            let _ = conn.close().await;
            // "PostgreSQL 16.4 on x86_64-pc-linux-gnu, compiled by …" → "PostgreSQL 16.4"
            Ok(v.split(" on ").next().unwrap_or(&v).trim().to_string())
        }
        Engine::Mysql => {
            use sqlx::mysql::{MySqlConnectOptions, MySqlConnection, MySqlSslMode};
            let mut opts = MySqlConnectOptions::new()
                .host(&s.host)
                .port(s.port.unwrap_or(3306))
                .username(&s.user)
                .ssl_mode(if s.ssl { MySqlSslMode::Required } else { MySqlSslMode::Preferred });
            if !s.database.is_empty() {
                opts = opts.database(&s.database);
            }
            // An empty secret means no password; sending one fails where none is set.
            if !secret.is_empty() {
                opts = opts.password(secret);
            }
            let mut conn = MySqlConnection::connect_with(&opts).await.map_err(connection_error)?;
            let (v,): (String,) = sqlx::query_as("SELECT VERSION()").fetch_one(&mut conn).await.map_err(connection_error)?;
            let _ = conn.close().await;
            let engine = if v.to_ascii_lowercase().contains("mariadb") { "MariaDB" } else { "MySQL" };
            Ok(format!("{engine} {}", v.split('-').next().unwrap_or(&v)))
        }
        Engine::Libsql => libsql_version(s, secret).await,
    }
}

/// Runs `SELECT sqlite_version()` through the Hrana-over-HTTP pipeline API.
async fn libsql_version(s: &ConnectionSettings, token: &str) -> AppResult<String> {
    let url = match s.port {
        Some(p) => format!("https://{}:{p}/v2/pipeline", s.host),
        None => format!("https://{}/v2/pipeline", s.host),
    };
    let body = serde_json::json!({
        "requests": [
            { "type": "execute", "stmt": { "sql": "SELECT sqlite_version()" } },
            { "type": "close" }
        ]
    });
    let mut req = reqwest::Client::new().post(&url).json(&body);
    if !token.is_empty() {
        req = req.bearer_auth(token);
    }
    let res = req.send().await.map_err(connection_error)?;
    let status = res.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(AppError::Connection("the server rejected the token".into()));
    }
    let text = res.text().await.map_err(connection_error)?;
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::Connection(format!("there is no database at {}", s.host)));
    }
    if !status.is_success() {
        return Err(AppError::Connection(format!("the server answered {status}: {}", text.chars().take(200).collect::<String>())));
    }
    let v: serde_json::Value = serde_json::from_str(&text).map_err(connection_error)?;
    let first = &v["results"][0];
    if first["type"] == "error" {
        return Err(AppError::Connection(first["error"]["message"].as_str().unwrap_or("the query failed").to_string()));
    }
    let version = first["response"]["result"]["rows"][0][0]["value"]
        .as_str()
        .ok_or_else(|| AppError::Connection("the server's answer was not understood".into()))?;
    Ok(format!("libSQL (SQLite {version})"))
}

// ── Secrets ──

/// Passwords and tokens of online databases, kept in the app config dir and
/// keyed by the workspace's `DB` folder and the connection name. Cloning
/// shares the same state.
#[derive(Clone)]
pub struct SecretStore {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl SecretStore {
    pub fn new(path: PathBuf) -> Self {
        SecretStore { path, lock: Arc::new(Mutex::new(())) }
    }

    fn key(dir: &Path, name: &str) -> String {
        format!("{}\n{name}", dir.display())
    }

    fn read(&self) -> BTreeMap<String, String> {
        fs::read(&self.path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    fn write(&self, all: &BTreeMap<String, String>) -> AppResult<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        options.open(&tmp)?.write_all(&serde_json::to_vec_pretty(all)?)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    fn guard(&self) -> AppResult<std::sync::MutexGuard<'_, ()>> {
        self.lock.lock().map_err(|_| AppError::Internal("secret store lock poisoned".into()))
    }

    pub fn get(&self, dir: &Path, name: &str) -> AppResult<Option<String>> {
        let _g = self.guard()?;
        Ok(self.read().remove(&Self::key(dir, name)))
    }

    /// Saves (Some) or forgets (None) a secret.
    pub fn set(&self, dir: &Path, name: &str, secret: Option<&str>) -> AppResult<()> {
        let _g = self.guard()?;
        let mut all = self.read();
        let key = Self::key(dir, name);
        let changed = match secret {
            Some(s) => all.insert(key, s.to_string()).as_deref() != Some(s),
            None => all.remove(&key).is_some(),
        };
        if changed { self.write(&all) } else { Ok(()) }
    }

    pub fn rename(&self, dir: &Path, from: &str, to: &str) -> AppResult<()> {
        let _g = self.guard()?;
        let mut all = self.read();
        match all.remove(&Self::key(dir, from)) {
            Some(s) => {
                all.insert(Self::key(dir, to), s);
                self.write(&all)
            }
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn neon() -> ConnectionSettings {
        ConnectionSettings {
            provider: "neon".into(),
            engine: Engine::Postgres,
            host: "ep-cool-1234.eu-central-1.aws.neon.tech".into(),
            port: None,
            database: "neondb".into(),
            user: "owner".into(),
            ssl: true,
        }
    }

    #[test]
    fn validates_settings() {
        assert!(neon().validate().is_ok());
        let bad = [
            ConnectionSettings { provider: "oracle".into(), ..neon() },
            ConnectionSettings { engine: Engine::Mysql, ..neon() },
            ConnectionSettings { host: "".into(), ..neon() },
            ConnectionSettings { host: "a b".into(), ..neon() },
            ConnectionSettings { host: "x.com/db".into(), ..neon() },
            ConnectionSettings { port: Some(0), ..neon() },
            ConnectionSettings { user: "".into(), ..neon() },
        ];
        for s in bad {
            assert!(s.validate().is_err(), "{s:?}");
        }
        let turso = ConnectionSettings { provider: "turso".into(), engine: Engine::Libsql, user: "".into(), database: "".into(), ..neon() };
        assert!(turso.validate().is_ok());
    }

    #[test]
    fn save_rename_delete_with_secrets() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("DB");
        fs::create_dir(&dir).unwrap();
        let secrets = SecretStore::new(tmp.path().join("cfg/secrets.json"));
        let db = crate::database::OpenDatabase::create(dir.join("Shop.db").to_str().unwrap()).unwrap();
        db.close().unwrap();

        assert!(matches!(create(&dir, &secrets, "shop", &neon(), None), Err(AppError::AlreadyExists(_))));
        create(&dir, &secrets, "Cloud", &neon(), Some("pw")).unwrap();
        assert!(matches!(create(&dir, &secrets, "CLOUD", &neon(), None), Err(AppError::AlreadyExists(_))));
        let listed = list(&dir, &secrets).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].settings.as_ref(), Some(&neon()));
        assert!(listed[0].has_secret);
        let saved = fs::read_to_string(dir.join("Cloud.online")).unwrap();
        assert!(!saved.contains("pw"), "the password stays out of the workspace folder");

        let moved = ConnectionSettings { database: "app".into(), ..neon() };
        assert!(matches!(update(&dir, &secrets, "Cloud", "Shop", &moved, None), Err(AppError::AlreadyExists(_))));
        update(&dir, &secrets, "Cloud", "Remote", &moved, None).unwrap();
        assert_eq!(settings(&dir, "Remote").unwrap(), moved);
        assert_eq!(secrets.get(&dir, "Remote").unwrap().as_deref(), Some("pw"));
        assert!(secrets.get(&dir, "Cloud").unwrap().is_none());
        update(&dir, &secrets, "Remote", "Remote", &moved, Some("")).unwrap();
        assert!(secrets.get(&dir, "Remote").unwrap().is_none());

        // Local databases can't take an online database's name either.
        assert!(matches!(crate::workspace::new_database_path(&dir, "remote"), Err(AppError::AlreadyExists(_))));

        assert!(matches!(delete(&dir, &secrets, "Remote", false), Err(AppError::ConfirmationRequired(_))));
        delete(&dir, &secrets, "Remote", true).unwrap();
        assert!(names(&dir).unwrap().is_empty());
    }
}


//! Saved connections to online databases. They live in the app config dir
//! (`connections.json`), not the workspace folder, so passwords and tokens
//! never end up in a folder that may be synced or shared.

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::errors::{AppError, AppResult};
use crate::models::{Connection, ConnectionInfo, Engine};
use crate::workspace::check_file_stem;

#[derive(Default, Serialize, Deserialize)]
struct ConnectionsFile {
    #[serde(default)]
    connections: Vec<Connection>,
}

/// Cloning shares the same state.
#[derive(Clone)]
pub struct ConnectionStore {
    path: PathBuf,
    saved: Arc<Mutex<Vec<Connection>>>,
}

impl ConnectionStore {
    /// Loads saved connections; an unreadable file starts empty (and is only
    /// replaced on the next save).
    pub fn load(path: PathBuf) -> Self {
        let saved = fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<ConnectionsFile>(&s).ok())
            .map(|f| f.connections)
            .unwrap_or_default();
        ConnectionStore { path, saved: Arc::new(Mutex::new(saved)) }
    }

    fn lock(&self) -> AppResult<std::sync::MutexGuard<'_, Vec<Connection>>> {
        self.saved
            .lock()
            .map_err(|_| AppError::Internal("connections lock poisoned".into()))
    }

    pub fn list(&self) -> AppResult<Vec<ConnectionInfo>> {
        let mut out: Vec<ConnectionInfo> = self.lock()?.iter().map(ConnectionInfo::from).collect();
        out.sort_by_key(|c| c.name.to_lowercase());
        Ok(out)
    }

    pub fn get(&self, name: &str) -> AppResult<Connection> {
        self.lock()?
            .iter()
            .find(|c| c.name == name)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("online database {name:?}")))
    }

    /// `conn` with an empty password or token taken from the saved
    /// connection `previous` (the form leaves them blank when editing).
    pub fn with_saved_secrets(&self, mut conn: Connection, previous: Option<&str>) -> AppResult<Connection> {
        if let Some(prev) = previous {
            let old = self.get(prev)?;
            if conn.password.is_empty() {
                conn.password = old.password;
            }
            if conn.token.is_empty() {
                conn.token = old.token;
            }
        }
        Ok(conn)
    }

    /// Adds a connection, or replaces `previous` (which may be renamed).
    pub fn save(&self, conn: Connection, previous: Option<&str>) -> AppResult<ConnectionInfo> {
        let conn = self.with_saved_secrets(normalize(conn), previous)?;
        validate(&conn)?;
        let mut saved = self.lock()?;
        let clash = saved
            .iter()
            .any(|c| Some(c.name.as_str()) != previous && c.name.to_lowercase() == conn.name.to_lowercase());
        if clash {
            return Err(AppError::AlreadyExists(format!("an online database named {:?}", conn.name)));
        }
        let mut next = saved.clone();
        match previous {
            Some(prev) => {
                let slot = next
                    .iter_mut()
                    .find(|c| c.name == prev)
                    .ok_or_else(|| AppError::NotFound(format!("online database {prev:?}")))?;
                *slot = conn.clone();
            }
            None => next.push(conn.clone()),
        }
        self.write(&next)?;
        *saved = next;
        Ok(ConnectionInfo::from(&conn))
    }

    /// Forgets a connection; the database on the server is not touched.
    pub fn delete(&self, name: &str, confirm: bool) -> AppResult<()> {
        if !confirm {
            return Err(AppError::ConfirmationRequired(format!("removing the online database {name:?}")));
        }
        let mut saved = self.lock()?;
        if !saved.iter().any(|c| c.name == name) {
            return Err(AppError::NotFound(format!("online database {name:?}")));
        }
        let next: Vec<Connection> = saved.iter().filter(|c| c.name != name).cloned().collect();
        self.write(&next)?;
        *saved = next;
        Ok(())
    }

    fn write(&self, connections: &[Connection]) -> AppResult<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let body = serde_json::to_vec_pretty(&ConnectionsFile { connections: connections.to_vec() })?;
        let tmp = self.path.with_extension("json.tmp");
        write_private(&tmp, &body)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

/// Writes a file only the current user can read (it holds passwords).
fn write_private(path: &std::path::Path, body: &[u8]) -> AppResult<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)?;
        f.write_all(body)?;
        f.sync_all()?;
    }
    #[cfg(not(unix))]
    fs::write(path, body)?;
    Ok(())
}

fn normalize(mut c: Connection) -> Connection {
    c.host = c.host.trim().to_string();
    c.database = c.database.trim().to_string();
    c.user = c.user.trim().to_string();
    c.url = c.url.trim().trim_end_matches('/').to_string();
    c.token = c.token.trim().to_string();
    c
}

pub fn validate(c: &Connection) -> AppResult<()> {
    check_file_stem(&c.name, "database")?;
    let bad = |m: &str| Err(AppError::Validation(m.into()));
    match c.provider.engine() {
        Engine::Libsql => {
            let lower = c.url.to_ascii_lowercase();
            if !["libsql://", "https://", "http://"].iter().any(|s| lower.starts_with(s)) {
                return bad("the database URL must start with libsql://, https:// or http://");
            }
            if c.url.chars().any(char::is_whitespace) {
                return bad("the database URL must not contain spaces");
            }
        }
        _ => {
            if c.host.is_empty() || c.host.chars().any(|ch| ch.is_whitespace() || ch == '/') {
                return bad("enter the server address as a host name, like db.example.com");
            }
            if c.user.is_empty() {
                return bad("enter a user name");
            }
            if c.provider.engine() == Engine::Postgres && c.database.is_empty() {
                return bad("enter the database name");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Provider, TlsMode};

    fn pg(name: &str) -> Connection {
        Connection {
            name: name.into(),
            provider: Provider::Neon,
            host: " ep-1.neon.tech ".into(),
            port: Some(5432),
            database: "shop".into(),
            user: "alex".into(),
            password: "secret".into(),
            url: String::new(),
            token: String::new(),
            tls: TlsMode::Require,
            trust_certificate: false,
        }
    }

    #[test]
    fn save_edit_delete() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("connections.json");
        let store = ConnectionStore::load(path.clone());
        let info = store.save(pg("Shop"), None).unwrap();
        assert!(info.has_password);
        assert_eq!(info.host, "ep-1.neon.tech");
        assert!(matches!(store.save(pg("shop"), None), Err(AppError::AlreadyExists(_))));

        // Editing with a blank password keeps the saved one, and may rename.
        let edited = Connection { name: "Shop online".into(), password: String::new(), ..pg("x") };
        store.save(edited, Some("Shop")).unwrap();
        let reloaded = ConnectionStore::load(path.clone());
        assert_eq!(reloaded.get("Shop online").unwrap().password, "secret");
        assert!(reloaded.get("Shop").is_err());

        assert!(matches!(reloaded.delete("Shop online", false), Err(AppError::ConfirmationRequired(_))));
        reloaded.delete("Shop online", true).unwrap();
        assert!(ConnectionStore::load(path).list().unwrap().is_empty());
    }

    #[test]
    fn validation() {
        assert!(validate(&Connection { host: "a b".into(), ..pg("x") }).is_err());
        assert!(validate(&Connection { name: "a/b".into(), ..pg("x") }).is_err());
        let turso = Connection { provider: Provider::Turso, url: "libsql://db.turso.io".into(), ..pg("t") };
        assert!(validate(&turso).is_ok());
        assert!(validate(&Connection { url: "ftp://x".into(), ..turso }).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("connections.json");
        ConnectionStore::load(path.clone()).save(pg("Shop"), None).unwrap();
        assert_eq!(fs::metadata(path).unwrap().permissions().mode() & 0o777, 0o600);
    }
}

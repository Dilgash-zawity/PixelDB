use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::ObjectKind;

/// Where an online database lives. Hosted services speak one of the engine
/// protocols (Supabase and Neon are Postgres, Turso is libSQL).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Postgres,
    Mysql,
    SqlServer,
    /// A libSQL (sqld) server over HTTP.
    Sqlite,
    Supabase,
    Neon,
    Turso,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Postgres,
    Mysql,
    SqlServer,
    Libsql,
}

impl Provider {
    pub fn engine(self) -> Engine {
        match self {
            Provider::Postgres | Provider::Supabase | Provider::Neon => Engine::Postgres,
            Provider::Mysql => Engine::Mysql,
            Provider::SqlServer => Engine::SqlServer,
            Provider::Sqlite | Provider::Turso => Engine::Libsql,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TlsMode {
    Disable,
    #[default]
    Prefer,
    Require,
}

/// A saved connection to an online database, secrets included. Only
/// `ConnectionInfo` (no password or token) is sent back to the frontend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    pub name: String,
    pub provider: Provider,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub database: String,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub password: String,
    /// libSQL: `libsql://…`, `https://…` or `http://…`.
    #[serde(default)]
    pub url: String,
    /// libSQL: the bearer token.
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub tls: TlsMode,
    /// SQL Server: accept a self-signed server certificate.
    #[serde(default)]
    pub trust_certificate: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionInfo {
    pub name: String,
    pub provider: Provider,
    pub host: String,
    pub port: Option<u16>,
    pub database: String,
    pub user: String,
    pub url: String,
    pub tls: TlsMode,
    pub trust_certificate: bool,
    pub has_password: bool,
    pub has_token: bool,
}

impl From<&Connection> for ConnectionInfo {
    fn from(c: &Connection) -> Self {
        ConnectionInfo {
            name: c.name.clone(),
            provider: c.provider,
            host: c.host.clone(),
            port: c.port,
            database: c.database.clone(),
            user: c.user.clone(),
            url: c.url.clone(),
            tls: c.tls,
            trust_certificate: c.trust_certificate,
            has_password: !c.password.is_empty(),
            has_token: !c.token.is_empty(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerInfo {
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RemoteTable {
    /// Schema (Postgres, SQL Server); None where a database has only one.
    pub schema: Option<String>,
    pub name: String,
    pub kind: ObjectKind,
}

/// Rows read from an online database, every value as JSON (numbers that fit
/// stay numbers, everything else is text as the server shows it).
#[derive(Debug, Clone, Default, Serialize)]
pub struct RemoteResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<JsonValue>>,
    pub truncated: bool,
    pub elapsed_ms: u64,
}

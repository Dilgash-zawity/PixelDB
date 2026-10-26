//! Tauri IPC commands. Each command is a thin wrapper that runs the matching
//! `database::*` function on a blocking thread.

pub mod apps;
pub mod database;
pub mod diagnostics;
pub mod online;
pub mod queries;
pub mod records;
pub mod tables;
pub mod transfer;
pub mod workspace;

use std::path::PathBuf;

use crate::errors::AppResult;
use crate::online::{ConnectionSettings, SecretStore};
use crate::workspace::WorkspaceStore;

/// Which database a command acts on: the named workspace database when an
/// app passes `database`, else the one open on the database page.
pub(crate) fn target(store: &WorkspaceStore, database: Option<String>) -> AppResult<Option<(PathBuf, String)>> {
    database.map(|name| Ok((store.databases()?, name))).transpose()
}

/// The settings and secret of the online connection `database` names, when
/// it names one rather than a local database (names are unique across both).
pub(crate) fn online_target(
    store: &WorkspaceStore,
    secrets: &SecretStore,
    database: Option<&str>,
) -> AppResult<Option<(ConnectionSettings, String)>> {
    let Some(name) = database else { return Ok(None) };
    let dir = store.databases()?;
    if !crate::online::names(&dir)?.iter().any(|n| n == name) {
        return Ok(None);
    }
    let settings = crate::online::settings(&dir, name)?;
    Ok(Some((settings, secrets.get(&dir, name)?.unwrap_or_default())))
}

/// Runs `$online` (with `o`, an `online_records::Online`) when `$database`
/// names an online connection; otherwise falls through.
macro_rules! on_online {
    ($store:expr, $secrets:expr, $pools:expr, $database:expr, |$o:ident| $online:expr) => {
        if let Some((settings, secret)) = super::online_target(&$store, &$secrets, $database.as_deref())? {
            let $o = crate::online_records::Online::new(&$pools, &settings, &secret);
            return $online.await;
        }
    };
}
pub(crate) use on_online;

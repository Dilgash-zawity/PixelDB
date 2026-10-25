//! Tauri IPC commands. Each command is a thin wrapper that runs the matching
//! `database::*` function on a blocking thread.

pub mod apps;
pub mod connections;
pub mod database;
pub mod queries;
pub mod records;
pub mod tables;
pub mod workspace;

use std::path::PathBuf;

use crate::errors::AppResult;
use crate::workspace::WorkspaceStore;

/// Which database a command acts on: the named workspace database when an
/// app passes `database`, else the one open on the database page.
pub(crate) fn target(store: &WorkspaceStore, database: Option<String>) -> AppResult<Option<(PathBuf, String)>> {
    database.map(|name| Ok((store.require()?, name))).transpose()
}

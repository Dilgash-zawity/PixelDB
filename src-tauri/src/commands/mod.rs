//! Tauri IPC commands. Each command is a thin wrapper that runs the matching
//! `database::*` function on a blocking thread.

pub mod database;
pub mod records;
pub mod tables;
